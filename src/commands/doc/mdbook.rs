//! mdBook source generation and building.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use regex::Regex;

use crate::errors::MxpmError;

use super::includes::{self, IncludeEntry};

/// Generate mdBook source from a Markdown file and build HTML.
pub(super) fn generate_mdbook(md_path: &Path, stem: &str, out_dir: &Path) -> Result<(), MxpmError> {
    let book_dir = regenerate_mdbook_src(md_path, stem, out_dir)?;
    invoke_mdbook_build(&book_dir)?;
    Ok(())
}

/// Regenerate mdBook source files from a Markdown file.
///
/// If the source file contains `<!-- include: ... -->` directives, each included
/// file becomes a separate mdBook chapter. Otherwise, falls back to splitting by
/// `##` headings.
///
/// Creates/updates `book/src/` with split sections and SUMMARY.md.
/// Returns the book directory path. Does NOT run `mdbook build`.
pub(super) fn regenerate_mdbook_src(
    md_path: &Path,
    stem: &str,
    out_dir: &Path,
) -> Result<PathBuf, MxpmError> {
    let book_dir = out_dir.join("book");
    let src_dir = book_dir.join("src");
    fs::create_dir_all(&src_dir)?;

    let md_content = fs::read_to_string(md_path)?;

    // Generate book.toml
    let book_toml = format!("[book]\ntitle = \"{stem}\"\nlanguage = \"en\"\n\n[output.html]\n");
    fs::write(book_dir.join("book.toml"), book_toml)?;

    let parsed_includes = includes::parse_includes(md_path)?;

    if !parsed_includes.is_empty() {
        // Include-based mode: inline content becomes intro, each include is a chapter
        regenerate_mdbook_from_includes(md_path, &md_content, &parsed_includes, stem, &src_dir)?;
    } else {
        // Legacy mode: split by ## headings
        regenerate_mdbook_from_headings(&md_content, stem, &src_dir)?;
    }

    // Sync figures/ directory into book/src/ so image references resolve
    let md_dir = md_path.parent().unwrap_or(Path::new("."));
    let figures_src = md_dir.join("figures");
    let figures_dst = src_dir.join("figures");
    if figures_src.is_dir() {
        // Remove stale symlink or directory
        if figures_dst.is_symlink() || figures_dst.exists() {
            if figures_dst.is_symlink() || !figures_dst.is_dir() {
                fs::remove_file(&figures_dst).ok();
            } else {
                fs::remove_dir_all(&figures_dst).ok();
            }
        }
        #[cfg(unix)]
        {
            let abs_src = figures_src.canonicalize()?;
            std::os::unix::fs::symlink(&abs_src, &figures_dst)?;
        }
        #[cfg(not(unix))]
        {
            // Copy figures directory on non-unix platforms
            fs::create_dir_all(&figures_dst)?;
            for entry in fs::read_dir(&figures_src)?.flatten() {
                if entry.file_type().map_or(false, |t| t.is_file()) {
                    fs::copy(entry.path(), figures_dst.join(entry.file_name()))?;
                }
            }
        }
    }

    eprintln!("Wrote mdBook source to {}", book_dir.display());
    Ok(book_dir)
}

/// A section parsed from the main doc file for mdBook generation.
struct MdBookSection {
    title: String,
    content: String,
    includes: Vec<PathBuf>,
}

/// Generate mdBook chapters from include directives.
///
/// Parses the source file into sections (split by `##` headings). Each section
/// becomes a top-level mdBook chapter. Includes within a section become nested
/// sub-chapters under that section's chapter.
fn regenerate_mdbook_from_includes(
    source_path: &Path,
    source_content: &str,
    _includes: &[IncludeEntry],
    _stem: &str,
    src_dir: &Path,
) -> Result<(), MxpmError> {
    let include_re = Regex::new(r"^<!--\s*include:\s*(\S+)\s*-->$").unwrap();
    let base_dir = source_path.parent().unwrap_or(Path::new("."));

    // Parse into sections split by ## headings
    let mut sections: Vec<MdBookSection> = Vec::new();
    let mut current = MdBookSection {
        title: String::new(),
        content: String::new(),
        includes: Vec::new(),
    };

    for line in source_content.lines() {
        if line.starts_with("# ") && !line.starts_with("## ") {
            // Skip top-level title
            continue;
        }
        if let Some(title) = line.strip_prefix("## ") {
            // Flush current section if it has any content or includes
            if !current.title.is_empty()
                || !current.content.trim().is_empty()
                || !current.includes.is_empty()
            {
                sections.push(current);
            }
            current = MdBookSection {
                title: title.trim().to_string(),
                content: String::new(),
                includes: Vec::new(),
            };
        } else if let Some(caps) = include_re.captures(line) {
            current.includes.push(base_dir.join(&caps[1]));
        } else {
            current.content.push_str(line);
            current.content.push('\n');
        }
    }
    // Flush last section
    if !current.title.is_empty()
        || !current.content.trim().is_empty()
        || !current.includes.is_empty()
    {
        sections.push(current);
    }

    // Generate SUMMARY.md and chapter files
    let mut summary = String::from("# Summary\n\n");

    for section in &sections {
        let slug = slugify(&section.title);
        let filename = if slug.is_empty() {
            "index.md".to_string()
        } else {
            format!("{}.md", slug)
        };

        // Write section chapter file
        let trimmed = section.content.trim();
        let chapter_content = if trimmed.is_empty() {
            format!("# {}\n", section.title)
        } else {
            format!("# {}\n\n{}\n", section.title, trimmed)
        };
        fs::write(src_dir.join(&filename), chapter_content)?;

        let label = if section.title.is_empty() {
            "Introduction".to_string()
        } else {
            section.title.clone()
        };
        summary.push_str(&format!("- [{}]({})\n", label, filename));

        // Write included files as nested sub-chapters
        for include_path in &section.includes {
            if !include_path.exists() {
                return Err(MxpmError::MakeinfoFailed {
                    message: format!(
                        "included file not found: {} (from {})",
                        include_path.display(),
                        source_path.display()
                    ),
                });
            }
            let content = fs::read_to_string(include_path)?;
            let rendered = render_mdbook_content(&content);

            let title = extract_chapter_title(&content).unwrap_or_else(|| {
                include_path
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string()
            });

            let inc_filename = include_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();

            let page_content = promote_first_heading(&rendered);
            fs::write(src_dir.join(&inc_filename), page_content)?;
            summary.push_str(&format!("  - [{}]({})\n", title, inc_filename));
        }
    }

    fs::write(src_dir.join("SUMMARY.md"), summary)?;
    Ok(())
}

/// Extract a chapter title from the first heading in markdown content.
///
/// Recognizes `### Function: name (args)` -> `name`, `## title` -> `title`.
fn extract_chapter_title(content: &str) -> Option<String> {
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("### Function: ") {
            // Extract just the function name
            let name = rest.split_whitespace().next()?;
            return Some(name.to_string());
        }
        if let Some(rest) = line.strip_prefix("### Variable: ") {
            return Some(rest.trim().to_string());
        }
        if let Some(title) = line.strip_prefix("## ") {
            return Some(title.trim().to_string());
        }
        if let Some(title) = line.strip_prefix("# ") {
            return Some(title.trim().to_string());
        }
    }
    None
}

/// Promote the first `##` or `###` heading to `#` for use as the mdBook page heading.
fn promote_first_heading(content: &str) -> String {
    let mut promoted = false;
    content
        .lines()
        .map(|line| {
            if !promoted && line.starts_with("### ") {
                promoted = true;
                // render_mdbook_line already transforms ### Function: headings,
                // so we may see "---\n### `name` (...) — Function". Just promote ### to #.
                line.replacen("### ", "# ", 1)
            } else if !promoted && line.starts_with("## ") {
                promoted = true;
                line.replacen("## ", "# ", 1)
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Legacy mdBook generation: split by ## headings.
fn regenerate_mdbook_from_headings(
    md_content: &str,
    stem: &str,
    src_dir: &Path,
) -> Result<(), MxpmError> {
    // Split by ## headings into separate pages.
    // The # heading becomes the book title (already in book.toml).
    let mut sections: Vec<(String, String)> = Vec::new();
    let mut current_title = String::new();
    let mut current_content = String::new();

    for line in md_content.lines() {
        if let Some(title) = line.strip_prefix("## ") {
            // Flush previous section
            if !current_title.is_empty() || !current_content.trim().is_empty() {
                sections.push((current_title.clone(), current_content.clone()));
            }
            current_title = title.trim().to_string();
            // Promote ## to # within each page so it's the page heading
            current_content = format!("# {}\n", current_title);
        } else if line.starts_with("# ") {
            // Skip the top-level heading — it's the book title
            continue;
        } else {
            current_content.push_str(&render_mdbook_line(line));
            current_content.push('\n');
        }
    }
    if !current_title.is_empty() || !current_content.trim().is_empty() {
        sections.push((current_title, current_content));
    }

    // Write sections to files + SUMMARY.md
    let mut summary = String::from("# Summary\n\n");

    if sections.is_empty() {
        // No ## headings at all — use the whole file as one page
        let rendered = render_mdbook_content(md_content);
        fs::write(src_dir.join("chapter-1.md"), rendered)?;
        summary.push_str(&format!("- [{}](chapter-1.md)\n", stem));
    } else {
        for (i, (title, content)) in sections.iter().enumerate() {
            let slug = slugify(title);
            let filename = if slug.is_empty() {
                format!("chapter-{}.md", i + 1)
            } else {
                format!("{}.md", slug)
            };
            let label = if title.is_empty() {
                format!("Chapter {}", i + 1)
            } else {
                title.clone()
            };
            summary.push_str(&format!("- [{}]({})\n", label, filename));
            fs::write(src_dir.join(&filename), content)?;
        }
    }

    fs::write(src_dir.join("SUMMARY.md"), summary)?;
    Ok(())
}

/// Render a single line for mdBook output.
///
/// Transforms `### Function: name (args)` and `### Variable: name` headings
/// into styled definition blocks for nicer HTML rendering.
fn render_mdbook_line(line: &str) -> String {
    let func_re = Regex::new(r"^### Function:\s+(\S+)\s*\((.*?)\)\s*$").unwrap();
    let var_re = Regex::new(r"^### Variable:\s+(\S+)\s*$").unwrap();

    if let Some(caps) = func_re.captures(line) {
        let name = &caps[1];
        let args = caps[2].trim();
        if args.is_empty() {
            format!("---\n### `{}` () — Function", name)
        } else {
            format!("---\n### `{}` (*{}*) — Function", name, args)
        }
    } else if let Some(caps) = var_re.captures(line) {
        let name = &caps[1];
        format!("---\n### `{}` — Variable", name)
    } else {
        line.to_string()
    }
}

/// Render markdown content for mdBook, transforming definition headings.
fn render_mdbook_content(content: &str) -> String {
    content
        .lines()
        .map(render_mdbook_line)
        .collect::<Vec<_>>()
        .join("\n")
}

/// Run `mdbook build` in the book directory, if mdbook is installed.
fn invoke_mdbook_build(book_dir: &Path) -> Result<(), MxpmError> {
    let which = Command::new("which").arg("mdbook").output();

    match which {
        Ok(output) if output.status.success() => {}
        _ => {
            eprintln!("mdbook not found; skipping HTML build. Install mdbook to build HTML.");
            return Ok(());
        }
    }

    let result = Command::new("mdbook")
        .arg("build")
        .current_dir(book_dir)
        .output()
        .map_err(|e| MxpmError::MakeinfoFailed {
            message: format!("failed to run mdbook build: {}", e),
        })?;

    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr);
        eprintln!("Warning: mdbook build failed: {}", stderr.trim());
    } else {
        eprintln!("Built mdBook HTML in {}", book_dir.join("book").display());
    }

    Ok(())
}

/// Convert a title to a URL-friendly slug.
fn slugify(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}
