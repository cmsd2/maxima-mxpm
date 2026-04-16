//! Generate markdown documentation from Maxima's Texinfo source.
//!
//! Reads Maxima's `.texi` source, runs `makeinfo --xml` to produce structured
//! XML, then extracts per-symbol documentation and emits markdown files that
//! follow the standard authoring conventions (`### Function:`, `### Variable:`).
//!
//! The output is a complete mxpm package that can be built with `mxpm doc build`.

mod emit;
mod markdown;
mod preprocess;
mod xml_parser;

use std::fs;
use std::path::{Path, PathBuf};

use crate::errors::MxpmError;

/// A symbol extracted from the Texinfo XML.
#[derive(Debug)]
struct ExtractedSymbol {
    name: String,
    symbol_type: String,
    signatures: Vec<String>,
    body_md: String,
    _examples: Vec<(String, String)>,
    see_also: Vec<String>,
    category: String,
}

pub fn run(maxima_src: &str, output_dir: Option<&str>, no_build: bool) -> Result<(), MxpmError> {
    let src = Path::new(maxima_src);
    let doc_info = src.join("doc").join("info");
    if !doc_info.exists() {
        return Err(MxpmError::MakeinfoFailed {
            message: format!("Maxima source directory not found: {}", doc_info.display()),
        });
    }

    let out = match output_dir {
        Some(d) => PathBuf::from(d),
        None => std::env::current_dir()?,
    };
    fs::create_dir_all(&out)?;

    // Step 1: Preprocess Texinfo
    let work_dir = tempfile::tempdir()?;
    eprintln!("Preprocessing Texinfo source...");
    let texi_path = preprocess::preprocess_texi(&doc_info, work_dir.path())?;

    // Step 2: Run makeinfo --xml
    eprintln!("Running makeinfo --xml...");
    let xml_path = preprocess::run_makeinfo_xml(&texi_path, work_dir.path())?;

    // Step 3: Parse XML
    eprintln!("Parsing XML...");
    let xml_content = fs::read_to_string(&xml_path)?;
    let symbols = xml_parser::parse_xml(&xml_content)?;
    eprintln!("Extracted {} symbols", symbols.len());

    // Step 4: Group by category and emit markdown
    let doc_dir = out.join("doc");
    fs::create_dir_all(&doc_dir)?;
    let categories = emit::emit_markdown_files(&symbols, &doc_dir)?;
    eprintln!("Wrote {} category files", categories.len());

    // Step 5: Generate main doc file with includes
    emit::emit_main_doc(&categories, &doc_dir)?;

    // Step 6: Copy figures
    let figures_src = doc_info.join("figures");
    if figures_src.is_dir() {
        let figures_dst = doc_dir.join("figures");
        copy_dir(&figures_src, &figures_dst)?;
        let count = fs::read_dir(&figures_dst)?.count();
        eprintln!("Copied {} figures", count);
    }

    // Step 7: Generate manifest and placeholder .mac
    emit::emit_manifest(&out, src)?;

    eprintln!("Generated package at {}", out.display());

    // Step 8: Optionally run mxpm doc build
    if !no_build {
        eprintln!("\nRunning doc build...");
        super::run_build(Some("doc/maxima-core-docs.md"), None, false, false)?;
    }

    Ok(())
}

/// Recursively copy a directory.
fn copy_dir(src: &Path, dst: &Path) -> Result<(), MxpmError> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let dest = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir(&entry.path(), &dest)?;
        } else {
            fs::copy(entry.path(), dest)?;
        }
    }
    Ok(())
}
