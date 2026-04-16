//! Documentation build commands: build, index, watch, serve.

pub mod generate_core_docs;
mod includes;
mod mdbook;
mod texi;
pub mod watch;

use std::fs;
use std::path::{Path, PathBuf};

use crate::doc_index;
use crate::errors::MxpmError;
use crate::info_index;
use crate::manifest;

/// Resolved documentation source information.
pub(crate) struct DocSource {
    pub file: String,
    pub out_dir: PathBuf,
    pub is_markdown: bool,
    pub stem: String,
}

/// Walk up from `start_dir` looking for `manifest.toml`.
/// Returns the directory containing it, or `None` if not found.
fn find_manifest_dir(start_dir: &Path) -> Option<PathBuf> {
    let mut dir = start_dir.to_path_buf();
    loop {
        if dir.join("manifest.toml").exists() {
            return Some(dir);
        }
        if !dir.pop() {
            return None;
        }
    }
}

/// Resolve the doc source file and output directory.
///
/// If `file` is `None`, reads from `manifest.toml`. If `output_dir` is `None`,
/// defaults to the package root (manifest-driven) or the source file's directory.
///
/// When `file` is provided, walks up parent directories looking for `manifest.toml`
/// to determine the package root for artifact placement.
pub(crate) fn resolve_doc_source(
    file: Option<&str>,
    output_dir: Option<&str>,
) -> Result<DocSource, MxpmError> {
    let (resolved_file, manifest_root) = match file {
        Some(f) => {
            let manifest_root = Path::new(f).parent().and_then(|p| {
                let abs = if p.as_os_str().is_empty() {
                    Path::new(".").canonicalize().ok()?
                } else {
                    p.canonicalize().ok()?
                };
                find_manifest_dir(&abs)
            });
            (f.to_string(), manifest_root)
        }
        None => {
            let manifest_path = Path::new("manifest.toml");
            if !manifest_path.exists() {
                return Err(MxpmError::ManifestNotFound {
                    path: "manifest.toml".to_string(),
                });
            }
            let contents = fs::read_to_string(manifest_path)?;
            let manifest =
                manifest::parse_manifest(&contents).map_err(|e| MxpmError::MakeinfoFailed {
                    message: format!("failed to parse manifest.toml: {}", e),
                })?;
            let doc_path = manifest.package.doc.ok_or_else(|| MxpmError::MakeinfoFailed {
                message: "no doc path in manifest.toml; add `doc = \"doc/<name>.md\"` to [package] or pass a file argument".to_string(),
            })?;
            let parent = manifest_path.parent().unwrap_or(Path::new("."));
            let root = if parent.as_os_str().is_empty() {
                PathBuf::from(".")
            } else {
                parent.to_path_buf()
            };
            let resolved = root.join(&doc_path).to_string_lossy().to_string();
            (resolved, Some(root))
        }
    };

    let path = Path::new(&resolved_file);

    let is_markdown = resolved_file.ends_with(".md") || resolved_file.ends_with(".markdown");
    let is_texi = resolved_file.ends_with(".texi") || resolved_file.ends_with(".texinfo");

    if !is_markdown && !is_texi {
        return Err(MxpmError::MakeinfoFailed {
            message: format!(
                "expected a .texi, .texinfo, or .md file, got: {}",
                path.file_name().unwrap_or_default().to_string_lossy()
            ),
        });
    }

    if !path.exists() {
        return Err(MxpmError::InfoFileNotFound {
            path: resolved_file.to_string(),
        });
    }

    let out_dir = match output_dir {
        Some(d) => {
            let p = PathBuf::from(d);
            fs::create_dir_all(&p)?;
            p.canonicalize()?
        }
        None => {
            if let Some(ref root) = manifest_root {
                root.canonicalize()?
            } else {
                path.parent().unwrap_or(Path::new(".")).canonicalize()?
            }
        }
    };

    let stem = path.file_stem().unwrap().to_string_lossy().to_string();

    Ok(DocSource {
        file: resolved_file,
        out_dir,
        is_markdown,
        stem,
    })
}

/// Determine package name from manifest (if available) or file stem.
fn determine_package_name(source: &DocSource) -> String {
    // Try reading manifest.toml from the output dir or its parents
    let manifest_path = source.out_dir.join("manifest.toml");
    if let Ok(contents) = fs::read_to_string(&manifest_path)
        && let Ok(m) = manifest::parse_manifest(&contents)
    {
        return m.package.name;
    }
    source.stem.clone()
}

/// Build all documentation artifacts from a source file.
///
/// Accepts `.texi`, `.texinfo`, or `.md` input.
/// Always generates `.info` + `*-index.lisp`. Optionally generates `.xml`
/// and/or mdBook source.
///
/// If `file` is `None`, reads the doc source path from `manifest.toml` in the
/// current directory and places outputs in the package root.
pub fn run_build(
    file: Option<&str>,
    output_dir: Option<&str>,
    xml: bool,
    mdbook: bool,
) -> Result<(), MxpmError> {
    let source = resolve_doc_source(file, output_dir)?;
    let file = &source.file;
    let path = Path::new(file);
    let is_markdown = source.is_markdown;
    let out_dir = &source.out_dir;
    let stem = &source.stem;

    check_staleness(path, out_dir, stem);

    // If Markdown, expand includes and convert to .texi via Pandoc.
    // Place the .texi next to the .md source (not in the output dir).
    let texi_path = if is_markdown {
        let texi_dir = path.parent().unwrap_or(Path::new(".")).canonicalize()?;
        let texi_dest = texi_dir.join(format!("{}.texi", stem));

        // Expand <!-- include: ... --> directives into a single file for pandoc
        let parsed_includes = includes::parse_includes(path)?;
        let pandoc_input = if parsed_includes.is_empty() {
            file.to_string()
        } else {
            let expanded = includes::expand_includes(path)?;
            let expanded_path = texi_dir.join(format!("{}.expanded.md", stem));
            fs::write(&expanded_path, &expanded)?;
            eprintln!(
                "Expanded {} include{} into {}",
                parsed_includes.len(),
                if parsed_includes.len() == 1 { "" } else { "s" },
                expanded_path.display()
            );
            expanded_path.to_string_lossy().to_string()
        };

        eprintln!("Converting Markdown to Texinfo via Pandoc...");
        texi::invoke_pandoc(&pandoc_input, &texi_dest)?;
        eprintln!("Wrote {}", texi_dest.display());

        // Post-process to add @deffn/@defvr blocks from our heading conventions
        texi::postprocess_texi(&texi_dest, stem)?;

        texi_dest.to_string_lossy().to_string()
    } else {
        file.to_string()
    };

    // 1. makeinfo --force -> .info
    eprintln!("Running makeinfo...");
    let info_path = texi::invoke_makeinfo(&texi_path)?;

    // If output dir differs from source dir, copy the .info there
    // Also copy any split files (.info-1, .info-2, etc.)
    let info_dest = out_dir.join(info_path.file_name().unwrap());
    if info_dest != info_path {
        fs::copy(&info_path, &info_dest)?;
        let info_basename = info_path.file_stem().unwrap_or_default().to_string_lossy();
        if let Some(info_dir) = info_path.parent() {
            for entry in fs::read_dir(info_dir)?.flatten() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if name_str.starts_with(&*info_basename) && name_str.contains(".info-") {
                    fs::copy(entry.path(), out_dir.join(&name))?;
                }
            }
        }
    }
    eprintln!("Wrote {}", info_dest.display());

    // 2. Build Lisp index
    let index = info_index::build_index(&info_dest)?;
    let lisp = info_index::render_lisp(&index, None);
    let info_stem = info_dest.file_stem().unwrap_or_default().to_string_lossy();
    let index_path = out_dir.join(format!("{}-index.lisp", info_stem));
    fs::write(&index_path, &lisp)?;
    eprintln!("Wrote {}", index_path.display());

    let items = index.deffn_defvr_entries.len();
    let sections = index.section_entries.len();
    if items == 0 && sections == 0 {
        eprintln!("Warning: empty index — no items or sections found.");
    } else {
        eprintln!(
            "Index: {} item{}, {} section{}",
            items,
            if items == 1 { "" } else { "s" },
            sections,
            if sections == 1 { "" } else { "s" },
        );
    }

    // 3. Doc index JSON (markdown sources only)
    if is_markdown {
        let md_content = if path.with_extension("expanded.md").exists() {
            fs::read_to_string(path.with_extension("expanded.md"))?
        } else {
            fs::read_to_string(path)?
        };
        let package_name = determine_package_name(&source);
        let doc_idx = doc_index::parse_markdown(&md_content, &package_name, file)?;
        let json = serde_json::to_string_pretty(&doc_idx)
            .map_err(|e| MxpmError::Io(std::io::Error::other(e)))?;
        let doc_index_dir = path.parent().unwrap_or(Path::new("."));
        let doc_index_path = doc_index_dir.join(format!("{}-doc-index.json", stem));
        fs::write(&doc_index_path, &json)?;
        eprintln!("Wrote {}", doc_index_path.display());

        let sym_count = doc_idx.symbols.len();
        let sec_count = doc_idx.sections.len();
        eprintln!(
            "Doc index: {} symbol{}, {} section{}",
            sym_count,
            if sym_count == 1 { "" } else { "s" },
            sec_count,
            if sec_count == 1 { "" } else { "s" },
        );

        // Lint the doc index
        let lint_warnings = doc_index::lint::lint_doc_index(&doc_idx);
        for w in &lint_warnings {
            let prefix = match w.level {
                doc_index::lint::LintLevel::Warn => "Warning",
                doc_index::lint::LintLevel::Info => "Note",
            };
            match &w.symbol {
                Some(sym) => eprintln!("{prefix}: '{sym}': {}", w.message),
                None => eprintln!("{prefix}: {}", w.message),
            }
        }
    }

    // 4. Optional XML
    if xml {
        let xml_path = texi::invoke_makeinfo_xml(&texi_path, out_dir)?;
        eprintln!("Wrote {}", xml_path.display());
    }

    // 5. Optional mdBook
    if mdbook {
        if is_markdown {
            // When manifest-driven, put book next to the source file (doc/), not package root
            let mdbook_dir = path.parent().unwrap_or(Path::new(".")).canonicalize()?;
            self::mdbook::generate_mdbook(path, stem, &mdbook_dir)?;
        } else {
            // From .texi, generate XML first then convert
            let xml_path = texi::invoke_makeinfo_xml(&texi_path, out_dir)?;
            eprintln!("Wrote {}", xml_path.display());
            eprintln!("Warning: mdBook from .texi XML is not yet implemented.");
            eprintln!("Hint: use a .md source file for mdBook support.");
            let _ = xml_path;
        }
    }

    Ok(())
}

/// Build a Maxima help index from a `.texi` or `.info` file.
pub fn run_index(
    file: &str,
    output: Option<&str>,
    install_path: Option<&str>,
) -> Result<(), MxpmError> {
    let info_path = if file.ends_with(".texi") || file.ends_with(".texinfo") {
        eprintln!("Running makeinfo on {}...", file);
        texi::invoke_makeinfo(file)?
    } else {
        PathBuf::from(file)
    };

    if !info_path.exists() {
        return Err(MxpmError::InfoFileNotFound {
            path: info_path.display().to_string(),
        });
    }

    let index = info_index::build_index(&info_path)?;

    let lisp = info_index::render_lisp(&index, install_path);

    if output == Some("-") {
        print!("{}", lisp);
    } else {
        let output_path = match output {
            Some(path) => PathBuf::from(path),
            None => {
                // Default: <basename>-index.lisp next to the .info file
                let stem = info_path.file_stem().unwrap_or_default().to_string_lossy();
                let parent = info_path.parent().unwrap_or(Path::new("."));
                parent.join(format!("{}-index.lisp", stem))
            }
        };

        fs::write(&output_path, &lisp)?;
        eprintln!("Wrote {}", output_path.display());
    }

    let items = index.deffn_defvr_entries.len();
    let sections = index.section_entries.len();
    if items == 0 && sections == 0 {
        eprintln!("Warning: empty index — no items or sections found.");
    } else {
        eprintln!(
            "Index: {} item{}, {} section{}",
            items,
            if items == 1 { "" } else { "s" },
            sections,
            if sections == 1 { "" } else { "s" },
        );
    }

    Ok(())
}

/// Check if doc artifacts are missing or older than the source file.
/// Prints informational notes — does not affect the build.
fn check_staleness(source_path: &Path, out_dir: &Path, stem: &str) {
    let source_mtime = source_path.metadata().and_then(|m| m.modified()).ok();

    let artifacts = [
        out_dir.join(format!("{}.info", stem)),
        out_dir.join(format!("{}-index.lisp", stem)),
    ];

    for artifact in &artifacts {
        if !artifact.exists() {
            eprintln!(
                "Note: {} is missing; will be generated.",
                artifact.file_name().unwrap().to_string_lossy()
            );
        } else if let (Some(src_t), Ok(art_meta)) = (source_mtime, artifact.metadata())
            && let Ok(art_t) = art_meta.modified()
            && src_t > art_t
        {
            eprintln!(
                "Note: {} is older than source; will be regenerated.",
                artifact.file_name().unwrap().to_string_lossy()
            );
        }
    }
}
