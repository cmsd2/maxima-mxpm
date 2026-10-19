//! Include directive expansion for markdown doc sources.

use std::fs;
use std::path::{Path, PathBuf};

use regex::Regex;

use crate::errors::MxpmError;

/// An include directive found in a markdown source file.
#[derive(Debug, Clone)]
pub(super) struct IncludeEntry {
    /// The resolved path to the included file.
    pub path: PathBuf,
}

/// Expand `<!-- include: path -->` directives in a markdown file.
///
/// Returns the expanded content with all includes inlined.
/// Paths are resolved relative to the source file's directory.
///
/// Each included file is set off by a blank line before and after it.
/// Pandoc's Markdown only recognises a heading after a blank line, so
/// without one a file that starts with `### Function: ...` would be merged
/// into the last paragraph of the file before it (typically a "See also"
/// line), losing that function's `@deffn` and its help-index entry.
pub(super) fn expand_includes(source_path: &Path) -> Result<String, MxpmError> {
    let content = fs::read_to_string(source_path)?;
    let base_dir = source_path.parent().unwrap_or(Path::new("."));
    let include_re = Regex::new(r"^<!--\s*include:\s*(\S+)\s*-->$").unwrap();

    let mut result = String::new();
    for line in content.lines() {
        if let Some(caps) = include_re.captures(line) {
            let include_path = base_dir.join(&caps[1]);
            if !include_path.exists() {
                return Err(MxpmError::MakeinfoFailed {
                    message: format!(
                        "included file not found: {} (from {})",
                        include_path.display(),
                        source_path.display()
                    ),
                });
            }
            let included = fs::read_to_string(&include_path)?;
            ensure_blank_line(&mut result);
            result.push_str(&included);
            if !included.ends_with('\n') {
                result.push('\n');
            }
            ensure_blank_line(&mut result);
        } else {
            result.push_str(line);
            result.push('\n');
        }
    }
    Ok(result)
}

/// End `text` with a blank line, unless it is empty or already does.
fn ensure_blank_line(text: &mut String) {
    if text.is_empty() || text.ends_with("\n\n") {
        return;
    }
    if !text.ends_with('\n') {
        text.push('\n');
    }
    text.push('\n');
}

/// Parse include directives from a markdown file without expanding them.
///
/// Returns a list of included file paths, in order.
pub(super) fn parse_includes(source_path: &Path) -> Result<Vec<IncludeEntry>, MxpmError> {
    let content = fs::read_to_string(source_path)?;
    let base_dir = source_path.parent().unwrap_or(Path::new("."));
    let include_re = Regex::new(r"^<!--\s*include:\s*(\S+)\s*-->$").unwrap();

    let mut entries = Vec::new();
    for line in content.lines() {
        if let Some(caps) = include_re.captures(line) {
            entries.push(IncludeEntry {
                path: base_dir.join(&caps[1]),
            });
        }
    }
    Ok(entries)
}

/// Collect all paths that should be watched: the source file plus any included files.
pub(super) fn collect_watch_paths(source_path: &Path) -> Vec<PathBuf> {
    let mut paths = vec![source_path.to_path_buf()];
    if let Ok(includes) = parse_includes(source_path) {
        for entry in includes {
            if entry.path.exists() {
                paths.push(entry.path);
            }
        }
    }
    paths
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fresh directory under the system temp dir for one test.
    fn test_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("mxpm-includes-{}-{}", name, std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn included_files_are_separated_by_blank_lines() {
        let dir = test_dir("separated");
        fs::write(
            dir.join("a.md"),
            "### Function: a ()\n\nText.\n\nSee also: `b`\n",
        )
        .unwrap();
        fs::write(dir.join("b.md"), "### Function: b ()\n\nMore text.\n").unwrap();
        fs::write(
            dir.join("main.md"),
            "## Section\n\n<!-- include: a.md -->\n<!-- include: b.md -->\n## Next\n",
        )
        .unwrap();

        let out = expand_includes(&dir.join("main.md")).unwrap();

        // b's heading must start its own block, not continue a's last paragraph
        assert!(out.contains("See also: `b`\n\n### Function: b ()"), "{out}");
        // and a heading after an include is set off too
        assert!(out.contains("More text.\n\n## Next"), "{out}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn included_file_without_trailing_newline() {
        let dir = test_dir("no-newline");
        fs::write(dir.join("a.md"), "See also: `x`").unwrap();
        fs::write(dir.join("b.md"), "### Function: b ()").unwrap();
        fs::write(
            dir.join("main.md"),
            "<!-- include: a.md -->\n<!-- include: b.md -->\n",
        )
        .unwrap();

        let out = expand_includes(&dir.join("main.md")).unwrap();

        assert_eq!(out, "See also: `x`\n\n### Function: b ()\n\n");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn existing_blank_lines_are_not_multiplied() {
        let dir = test_dir("no-extra");
        fs::write(dir.join("a.md"), "### Function: a ()\n\n").unwrap();
        fs::write(
            dir.join("main.md"),
            "# Title\n\n<!-- include: a.md -->\n\n## After\n",
        )
        .unwrap();

        let out = expand_includes(&dir.join("main.md")).unwrap();

        assert!(!out.contains("\n\n\n\n"), "{out:?}");
        assert!(
            out.starts_with("# Title\n\n### Function: a ()\n\n"),
            "{out:?}"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_include_is_an_error() {
        let dir = test_dir("missing");
        fs::write(dir.join("main.md"), "<!-- include: nope.md -->\n").unwrap();

        assert!(expand_includes(&dir.join("main.md")).is_err());
        let _ = fs::remove_dir_all(&dir);
    }
}
