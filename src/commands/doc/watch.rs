//! File watching and live-reload for doc builds.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use notify_debouncer_mini::new_debouncer;

use crate::errors::MxpmError;

use super::includes::collect_watch_paths;
use super::mdbook::regenerate_mdbook_src;
use super::{resolve_doc_source, run_build};

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

    let source_path = Path::new(&source.file).canonicalize()?;
    let watch_paths = collect_watch_paths(&source_path);
    eprintln!(
        "Watching {} file{} for changes... (Ctrl-C to stop)",
        watch_paths.len(),
        if watch_paths.len() == 1 { "" } else { "s" }
    );

    let out_dir_str = source.out_dir.to_string_lossy().to_string();
    let file_str = source.file.clone();

    watch_and_rebuild(&watch_paths, move || {
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

    let source_path = Path::new(&source.file).canonicalize()?;
    let watch_paths = collect_watch_paths(&source_path);
    eprintln!(
        "Watching {} file{} for changes... (Ctrl-C to stop)",
        watch_paths.len(),
        if watch_paths.len() == 1 { "" } else { "s" }
    );

    let md_path = PathBuf::from(&source.file);
    let stem = source.stem.clone();

    let result = watch_and_rebuild(&watch_paths, move || {
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

/// Watch one or more files for changes and call `on_change` for each detected modification.
///
/// Blocks until Ctrl-C is pressed.
fn watch_and_rebuild(watch_paths: &[PathBuf], on_change: impl Fn()) -> Result<(), MxpmError> {
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

    for watch_path in watch_paths {
        debouncer
            .watcher()
            .watch(watch_path, notify::RecursiveMode::NonRecursive)
            .map_err(|e| MxpmError::MakeinfoFailed {
                message: format!("failed to watch {}: {}", watch_path.display(), e),
            })?;
    }

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
