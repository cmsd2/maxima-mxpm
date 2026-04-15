use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use notify_debouncer_mini::new_debouncer;
use regex::Regex;

use crate::errors::MxpmError;
use crate::info_index;
use crate::manifest;

/// Resolved documentation source information.
struct DocSource {
    file: String,
    out_dir: PathBuf,
    is_markdown: bool,
    stem: String,
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
fn resolve_doc_source(
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

    // If Markdown, convert to .texi first via Pandoc.
    // Place the .texi next to the .md source (not in the output dir).
    let texi_path = if is_markdown {
        let texi_dir = path.parent().unwrap_or(Path::new(".")).canonicalize()?;
        let texi_dest = texi_dir.join(format!("{}.texi", stem));
        eprintln!("Converting Markdown to Texinfo via Pandoc...");
        invoke_pandoc(file, &texi_dest)?;
        eprintln!("Wrote {}", texi_dest.display());

        // Post-process to add @deffn/@defvr blocks from our heading conventions
        postprocess_texi(&texi_dest, stem)?;

        texi_dest.to_string_lossy().to_string()
    } else {
        file.to_string()
    };

    // 1. makeinfo --force → .info
    eprintln!("Running makeinfo...");
    let info_path = invoke_makeinfo(&texi_path)?;

    // If output dir differs from source dir, copy the .info there
    let info_dest = out_dir.join(info_path.file_name().unwrap());
    if info_dest != info_path {
        fs::copy(&info_path, &info_dest)?;
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

    // 3. Optional XML
    if xml {
        let xml_path = invoke_makeinfo_xml(&texi_path, out_dir)?;
        eprintln!("Wrote {}", xml_path.display());
    }

    // 4. Optional mdBook
    if mdbook {
        if is_markdown {
            // When manifest-driven, put book next to the source file (doc/), not package root
            let mdbook_dir = path.parent().unwrap_or(Path::new(".")).canonicalize()?;
            generate_mdbook(file, stem, &mdbook_dir)?;
        } else {
            // From .texi, generate XML first then convert
            let xml_path = invoke_makeinfo_xml(&texi_path, out_dir)?;
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
        invoke_makeinfo(file)?
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

/// Watch a doc source file and rebuild on changes.
pub fn run_watch(
    file: Option<&str>,
    output_dir: Option<&str>,
    xml: bool,
    mdbook: bool,
) -> Result<(), MxpmError> {
    let source = resolve_doc_source(file, output_dir)?;

    // Initial build
    run_build(
        Some(&source.file),
        Some(source.out_dir.to_str().unwrap()),
        xml,
        mdbook,
    )?;

    let watch_path = Path::new(&source.file).canonicalize()?;
    eprintln!(
        "Watching {} for changes... (Ctrl-C to stop)",
        watch_path.display()
    );

    let out_dir_str = source.out_dir.to_string_lossy().to_string();
    let file_str = source.file.clone();

    watch_and_rebuild(&watch_path, move || {
        eprintln!("Change detected, rebuilding...");
        match run_build(Some(&file_str), Some(&out_dir_str), xml, mdbook) {
            Ok(()) => eprintln!("Rebuild complete."),
            Err(e) => eprintln!("Rebuild failed: {}", e),
        }
    })
}

/// Watch a doc source file and serve mdBook HTML with live reload.
///
/// Spawns `mdbook serve` on the book directory (which handles HTTP + livereload),
/// then watches the source `.md` file and regenerates `book/src/` on changes.
/// `mdbook serve` detects the source changes and rebuilds HTML automatically.
pub fn run_serve(
    file: Option<&str>,
    port: u16,
    hostname: &str,
    open: bool,
) -> Result<(), MxpmError> {
    let source = resolve_doc_source(file, None)?;

    if !source.is_markdown {
        return Err(MxpmError::MakeinfoFailed {
            message: "doc serve requires a .md source file for live preview".to_string(),
        });
    }

    // Check mdbook is available
    let which = Command::new("which").arg("mdbook").output();
    match which {
        Ok(output) if output.status.success() => {}
        _ => {
            return Err(MxpmError::MakeinfoFailed {
                message: "mdbook not found; install mdbook for doc serve".to_string(),
            });
        }
    }

    // Initial full build with mdbook
    run_build(
        Some(&source.file),
        Some(source.out_dir.to_str().unwrap()),
        false,
        true,
    )?;

    // Resolve book directory (next to the source file)
    let source_dir = Path::new(&source.file)
        .parent()
        .unwrap_or(Path::new("."))
        .canonicalize()?;
    let book_dir = source_dir.join("book");

    if !book_dir.join("book.toml").exists() {
        return Err(MxpmError::MakeinfoFailed {
            message: format!("book.toml not found in {}", book_dir.display()),
        });
    }

    // Spawn mdbook serve as a child process
    let mut args = vec![
        "serve".to_string(),
        "-p".to_string(),
        port.to_string(),
        "-n".to_string(),
        hostname.to_string(),
    ];
    if open {
        args.push("--open".to_string());
    }

    let mut child = Command::new("mdbook")
        .args(&args)
        .current_dir(&book_dir)
        .spawn()
        .map_err(|e| MxpmError::MakeinfoFailed {
            message: format!("failed to start mdbook serve: {}", e),
        })?;

    eprintln!("Serving at http://{}:{}", hostname, port);

    let watch_path = Path::new(&source.file).canonicalize()?;
    eprintln!(
        "Watching {} for changes... (Ctrl-C to stop)",
        watch_path.display()
    );

    let md_path = source.file.clone();
    let stem = source.stem.clone();

    let result = watch_and_rebuild(&watch_path, move || {
        eprintln!("Change detected, updating mdBook source...");
        match regenerate_mdbook_src(&md_path, &stem, &source_dir) {
            Ok(_) => eprintln!("Updated. Browser should reload."),
            Err(e) => eprintln!("Update failed: {}", e),
        }
    });

    // Clean up child process
    child.kill().ok();
    child.wait().ok();

    result
}

/// Watch a file for changes and call `on_change` for each detected modification.
///
/// Blocks until Ctrl-C is pressed.
fn watch_and_rebuild(watch_path: &Path, on_change: impl Fn()) -> Result<(), MxpmError> {
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .map_err(|e| MxpmError::MakeinfoFailed {
        message: format!("failed to set Ctrl-C handler: {}", e),
    })?;

    let (tx, rx) = std::sync::mpsc::channel();
    let mut debouncer =
        new_debouncer(Duration::from_millis(300), tx).map_err(|e| MxpmError::MakeinfoFailed {
            message: format!("failed to create file watcher: {}", e),
        })?;

    debouncer
        .watcher()
        .watch(watch_path, notify::RecursiveMode::NonRecursive)
        .map_err(|e| MxpmError::MakeinfoFailed {
            message: format!("failed to watch {}: {}", watch_path.display(), e),
        })?;

    while running.load(Ordering::SeqCst) {
        match rx.recv_timeout(Duration::from_millis(200)) {
            Ok(Ok(_events)) => {
                on_change();
            }
            Ok(Err(e)) => {
                eprintln!("Watch error: {:?}", e);
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                // Check running flag
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                break;
            }
        }
    }

    eprintln!("\nStopping.");
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

/// Invoke `makeinfo` to compile a `.texi` file into `.info`.
fn invoke_makeinfo(texi_path: &str) -> Result<PathBuf, MxpmError> {
    let texi = Path::new(texi_path);

    // Check makeinfo is available
    let which = Command::new("which")
        .arg("makeinfo")
        .output()
        .map_err(|_| MxpmError::MakeinfoNotFound)?;

    if !which.status.success() {
        return Err(MxpmError::MakeinfoNotFound);
    }

    // Run makeinfo --force <file> in the source directory
    let cwd = texi
        .parent()
        .unwrap_or(Path::new("."))
        .canonicalize()
        .map_err(|e| MxpmError::MakeinfoFailed {
            message: format!("cannot resolve directory: {}", e),
        })?;

    let filename = texi.file_name().ok_or_else(|| MxpmError::MakeinfoFailed {
        message: "invalid texi path".to_string(),
    })?;

    let result = Command::new("makeinfo")
        .arg("--force")
        .arg(filename)
        .current_dir(&cwd)
        .output()
        .map_err(|e| MxpmError::MakeinfoFailed {
            message: format!("failed to run makeinfo: {}", e),
        })?;

    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr);
        return Err(MxpmError::MakeinfoFailed {
            message: stderr.trim().to_string(),
        });
    }

    // Determine the output .info filename.
    // Try to read @setfilename from the .texi, otherwise derive from basename.
    let info_name = read_setfilename(texi).unwrap_or_else(|| {
        let stem = texi.file_stem().unwrap().to_string_lossy();
        format!("{}.info", stem)
    });

    let info_path = cwd.join(&info_name);
    if !info_path.exists() {
        return Err(MxpmError::MakeinfoFailed {
            message: format!("makeinfo succeeded but {} not found", info_name),
        });
    }

    Ok(info_path)
}

/// Invoke `makeinfo --xml` to compile a `.texi` file into XML.
fn invoke_makeinfo_xml(texi_path: &str, output_dir: &Path) -> Result<PathBuf, MxpmError> {
    let texi = Path::new(texi_path);

    // Derive XML filename from @setfilename or basename
    let stem = read_setfilename(texi)
        .map(|name| {
            // Strip .info suffix if present
            name.strip_suffix(".info").unwrap_or(&name).to_string()
        })
        .unwrap_or_else(|| texi.file_stem().unwrap().to_string_lossy().to_string());
    let xml_filename = format!("{}.xml", stem);
    let xml_path = output_dir.join(&xml_filename);

    let result = Command::new("makeinfo")
        .arg("--xml")
        .arg("--force")
        .arg(format!("--output={}", xml_path.display()))
        .arg(texi)
        .output()
        .map_err(|e| MxpmError::MakeinfoFailed {
            message: format!("failed to run makeinfo --xml: {}", e),
        })?;

    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr);
        return Err(MxpmError::MakeinfoFailed {
            message: format!("makeinfo --xml failed: {}", stderr.trim()),
        });
    }

    if !xml_path.exists() {
        return Err(MxpmError::MakeinfoFailed {
            message: format!("makeinfo --xml succeeded but {} not found", xml_filename),
        });
    }

    Ok(xml_path)
}

/// Invoke `pandoc` to convert a Markdown file to Texinfo.
fn invoke_pandoc(md_path: &str, texi_dest: &Path) -> Result<(), MxpmError> {
    let which = Command::new("which")
        .arg("pandoc")
        .output()
        .map_err(|_| MxpmError::PandocNotFound)?;

    if !which.status.success() {
        return Err(MxpmError::PandocNotFound);
    }

    let result = Command::new("pandoc")
        .arg(md_path)
        .arg("-s")
        .arg("-o")
        .arg(texi_dest)
        .output()
        .map_err(|e| MxpmError::PandocFailed {
            message: format!("failed to run pandoc: {}", e),
        })?;

    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr);
        return Err(MxpmError::PandocFailed {
            message: stderr.trim().to_string(),
        });
    }

    Ok(())
}

/// Post-process Pandoc's Texinfo output to add `@deffn`/`@defvr` blocks.
///
/// Converts headings matching our convention:
///   `@subsection Function: name (args)` → `@deffn {Function} name (@var{args})`
///   `@subsection Variable: name`        → `@defvr {Variable} name`
///
/// Also injects `@setfilename`, `@printindex fn`, and `@printindex vr`.
fn postprocess_texi(texi_path: &Path, stem: &str) -> Result<(), MxpmError> {
    let content = fs::read_to_string(texi_path)?;
    let mut lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();

    let func_re = Regex::new(r"^@subsection Function:\s+(\S+)\s*\((.*?)\)\s*$").unwrap();
    let var_re = Regex::new(r"^@subsection Variable:\s+(\S+)\s*$").unwrap();

    // Track which definition blocks we've opened so we can close them
    let mut in_deffn = false;
    let mut result_lines: Vec<String> = Vec::new();

    // Inject @setfilename after @input texinfo if not present
    if !content.contains("@setfilename") {
        for (i, line) in lines.iter().enumerate() {
            if line.starts_with("\\input texinfo") {
                lines.insert(i + 1, format!("@setfilename {}.info", stem));
                break;
            }
        }
    }

    for line in &lines {
        // Close deffn before @node lines (Pandoc puts @node before @subsection)
        if in_deffn && line.starts_with("@node ") {
            result_lines.push("@end deffn".to_string());
            result_lines.push(String::new());
            in_deffn = false;
        }

        if let Some(caps) = func_re.captures(line) {
            // Close previous definition block if still open
            if in_deffn {
                result_lines.push("@end deffn".to_string());
                result_lines.push(String::new());
            }
            let name = &caps[1];
            let raw_args = &caps[2];
            // Wrap each arg in @var{}
            let args: Vec<String> = raw_args
                .split(',')
                .map(|a| {
                    let a = a.trim();
                    if a.is_empty() {
                        String::new()
                    } else {
                        format!("@var{{{}}}", a)
                    }
                })
                .filter(|a| !a.is_empty())
                .collect();
            result_lines.push(format!(
                "@deffn {{Function}} {} ({})",
                name,
                args.join(", ")
            ));
            in_deffn = true;
        } else if let Some(caps) = var_re.captures(line) {
            if in_deffn {
                result_lines.push("@end deffn".to_string());
                result_lines.push(String::new());
            }
            in_deffn = false;
            let name = &caps[1];
            result_lines.push(format!("@defvr {{Variable}} {}", name));
            result_lines.push("@defvrx".to_string()); // marker for second-pass defvr closing
        } else {
            result_lines.push(line.clone());
        }
    }

    // Close any trailing deffn
    if in_deffn {
        result_lines.push("@end deffn".to_string());
    }

    // Second pass: close @defvr blocks and remove markers.
    // A @defvr block ends when we see: @deffn, @defvr, @section, @subsection, @node, @bye, @chapter, @printindex
    let end_triggers = [
        "@deffn",
        "@defvr",
        "@section",
        "@subsection",
        "@node",
        "@bye",
        "@chapter",
        "@appendix",
        "@printindex",
    ];
    let mut final_lines: Vec<String> = Vec::new();
    let mut in_defvr = false;

    for line in &result_lines {
        if line == "@defvrx" {
            in_defvr = true;
            continue;
        }

        if in_defvr && end_triggers.iter().any(|t| line.starts_with(t)) {
            final_lines.push("@end defvr".to_string());
            final_lines.push(String::new());
            in_defvr = false;
        }

        final_lines.push(line.clone());
    }

    if in_defvr {
        // Insert @end defvr before @bye
        if let Some(bye_pos) = final_lines.iter().position(|l| l.starts_with("@bye")) {
            final_lines.insert(bye_pos, "@end defvr".to_string());
            final_lines.insert(bye_pos + 1, String::new());
        } else {
            final_lines.push("@end defvr".to_string());
        }
    }

    // Inject a separate index node + @printindex before @bye if not already present
    if !content.contains("@printindex")
        && let Some(bye_pos) = final_lines.iter().position(|l| l.starts_with("@bye"))
    {
        let idx = [
            String::new(),
            "@node Function and variable index".to_string(),
            "@appendix Function and variable index".to_string(),
            "@printindex fn".to_string(),
            "@printindex vr".to_string(),
            String::new(),
        ];
        for (i, line) in idx.into_iter().enumerate() {
            final_lines.insert(bye_pos + i, line);
        }
    }

    let output = final_lines.join("\n");
    fs::write(texi_path, output)?;

    Ok(())
}

/// Generate mdBook source from a Markdown file and build HTML.
fn generate_mdbook(md_path: &str, stem: &str, out_dir: &Path) -> Result<(), MxpmError> {
    let book_dir = regenerate_mdbook_src(md_path, stem, out_dir)?;

    // Run mdbook build if available
    invoke_mdbook_build(&book_dir)?;

    Ok(())
}

/// Regenerate mdBook source files from a Markdown file.
///
/// Creates/updates `book/src/` with split sections and SUMMARY.md.
/// Returns the book directory path. Does NOT run `mdbook build`.
fn regenerate_mdbook_src(md_path: &str, stem: &str, out_dir: &Path) -> Result<PathBuf, MxpmError> {
    let book_dir = out_dir.join("book");
    let src_dir = book_dir.join("src");
    fs::create_dir_all(&src_dir)?;

    let md_content = fs::read_to_string(md_path)?;

    // Generate book.toml
    let book_toml = format!("[book]\ntitle = \"{stem}\"\nlanguage = \"en\"\n\n[output.html]\n");
    fs::write(book_dir.join("book.toml"), book_toml)?;

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
        let rendered = render_mdbook_content(&md_content);
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
    eprintln!("Wrote mdBook source to {}", book_dir.display());

    Ok(book_dir)
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

/// Read the `@setfilename` directive from a `.texi` file.
fn read_setfilename(texi_path: &Path) -> Option<String> {
    let content = fs::read_to_string(texi_path).ok()?;
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("@setfilename ") {
            return Some(rest.trim().to_string());
        }
    }
    None
}
