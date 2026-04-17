//! Load doc-index files from disk.
//!
//! Scans `~/.maxima/` for installed package documentation. For each package
//! directory, it first checks `manifest.toml` for a `doc` field and derives
//! the doc-index path from that. If no manifest exists, it falls back to
//! globbing `doc/*-doc-index.json`.
//!
//! Used by downstream consumers (LSP, MCP) to provide hover docs, completions,
//! and search.

use std::path::{Path, PathBuf};

use mxpm_core::manifest;
use mxpm_core::paths;

use crate::DocIndex;

/// Load a single doc-index JSON file.
pub fn load_from_file(path: &Path) -> Result<DocIndex, String> {
    let content = std::fs::read_to_string(path).map_err(|e| format!("read error: {}", e))?;
    serde_json::from_str(&content).map_err(|e| format!("parse error: {}", e))
}

/// Scan a directory for installed packages and load their doc-index files.
///
/// Typically called with the Maxima user directory (`~/.maxima/`).
///
/// For each subdirectory, discovery works as follows:
/// 1. If `manifest.toml` exists and has a `doc` field, derive the doc-index
///    path: replace the doc source extension with `-doc-index.json` in the
///    same directory (e.g. `doc = "doc/my-pkg.md"` → `doc/my-pkg-doc-index.json`).
/// 2. Otherwise, glob `doc/*-doc-index.json` as a fallback.
///
/// Returns successfully loaded indices; logs errors to stderr.
pub fn scan_installed(userdir: &Path) -> Vec<DocIndex> {
    let mut indices = Vec::new();

    if !userdir.is_dir() {
        return indices;
    }

    let entries = match std::fs::read_dir(userdir) {
        Ok(e) => e,
        Err(_) => return indices,
    };

    for pkg_entry in entries.flatten() {
        let pkg_path = pkg_entry.path();
        if !pkg_path.is_dir() {
            continue;
        }

        // Try manifest-based discovery first
        if let Some(path) = doc_index_path_from_manifest(&pkg_path) {
            match load_from_file(&path) {
                Ok(index) => {
                    indices.push(index);
                    continue;
                }
                Err(e) => {
                    eprintln!("[doc-index] Failed to load {}: {}", path.display(), e);
                }
            }
        }

        // Fallback: glob doc/*-doc-index.json
        let doc_dir = pkg_path.join("doc");
        if !doc_dir.is_dir() {
            continue;
        }

        let doc_entries = match std::fs::read_dir(&doc_dir) {
            Ok(e) => e,
            Err(_) => continue,
        };

        for file_entry in doc_entries.flatten() {
            let file_name = file_entry.file_name();
            let name = file_name.to_string_lossy();
            if !name.ends_with("-doc-index.json") {
                continue;
            }

            let path = file_entry.path();
            match load_from_file(&path) {
                Ok(index) => indices.push(index),
                Err(e) => {
                    eprintln!("[doc-index] Failed to load {}: {}", path.display(), e);
                }
            }
        }
    }

    indices
}

/// Derive the doc-index JSON path from a package's `manifest.toml`.
///
/// Reads the manifest, extracts the `doc` field, and computes:
/// `<pkg_dir>/<doc_dir>/<stem>-doc-index.json`
///
/// For example, `doc = "doc/my-pkg.md"` in `~/.maxima/my-pkg/manifest.toml`
/// yields `~/.maxima/my-pkg/doc/my-pkg-doc-index.json`.
fn doc_index_path_from_manifest(pkg_dir: &Path) -> Option<PathBuf> {
    let m = manifest::load_manifest(pkg_dir)?;
    let doc_field = m.package.doc?;
    let doc_path = Path::new(&doc_field);
    let stem = doc_path.file_stem()?.to_str()?;
    let doc_dir = doc_path.parent().unwrap_or(Path::new("."));
    let index_name = format!("{stem}-doc-index.json");
    Some(pkg_dir.join(doc_dir).join(index_name))
}

/// Convenience: resolve `maxima_userdir()` and scan for all installed doc indices.
pub fn load_all_installed() -> Vec<DocIndex> {
    match paths::maxima_userdir() {
        Some(dir) => scan_installed(&dir),
        None => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_empty_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let indices = scan_installed(tmp.path());
        assert!(indices.is_empty());
    }

    #[test]
    fn scan_nonexistent_dir() {
        let indices = scan_installed(Path::new("/nonexistent/path"));
        assert!(indices.is_empty());
    }

    #[test]
    fn scan_finds_glob_fallback() {
        let tmp = tempfile::tempdir().unwrap();

        // Package without manifest — should use glob fallback
        let pkg_doc = tmp.path().join("test-pkg/doc");
        std::fs::create_dir_all(&pkg_doc).unwrap();

        let index = crate::parse_markdown(
            "## Intro\n\nHello.\n\n### Function: foo (x)\n\nDoes foo.\n",
            "test-pkg",
            "doc/test-pkg.md",
        );
        let json = serde_json::to_string_pretty(&index).unwrap();
        std::fs::write(pkg_doc.join("test-pkg-doc-index.json"), &json).unwrap();

        let indices = scan_installed(tmp.path());
        assert_eq!(indices.len(), 1);
        assert_eq!(indices[0].package, "test-pkg");
        assert!(indices[0].symbols.contains_key("foo"));
    }

    #[test]
    fn scan_uses_manifest_doc_field() {
        let tmp = tempfile::tempdir().unwrap();
        let pkg_dir = tmp.path().join("my-pkg");
        let doc_dir = pkg_dir.join("doc");
        std::fs::create_dir_all(&doc_dir).unwrap();

        // Write manifest with doc field
        std::fs::write(
            pkg_dir.join("manifest.toml"),
            r#"[package]
name = "my-pkg"
version = "1.0.0"
description = "Test"
license = "MIT"
entry = "my-pkg.mac"
doc = "doc/my-pkg.md"
"#,
        )
        .unwrap();

        // Write doc-index at the manifest-derived path
        let index = crate::parse_markdown(
            "### Function: bar (x)\n\nDoes bar.\n",
            "my-pkg",
            "doc/my-pkg.md",
        );
        let json = serde_json::to_string_pretty(&index).unwrap();
        std::fs::write(doc_dir.join("my-pkg-doc-index.json"), &json).unwrap();

        let indices = scan_installed(tmp.path());
        assert_eq!(indices.len(), 1);
        assert_eq!(indices[0].package, "my-pkg");
        assert!(indices[0].symbols.contains_key("bar"));
    }
}
