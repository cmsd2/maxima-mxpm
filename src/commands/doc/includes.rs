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
            result.push_str(&included);
            if !included.ends_with('\n') {
                result.push('\n');
            }
        } else {
            result.push_str(line);
            result.push('\n');
        }
    }
    Ok(result)
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
