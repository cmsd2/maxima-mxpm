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
    chapter: String,
    keywords: Vec<String>,
}

/// Maxima's directory for a language's manual: `pt-BR` is `doc/info/pt_BR`
fn info_dir_name(lang: &str) -> String {
    match lang.split_once(['-', '_']) {
        Some((language, region)) => {
            format!("{}_{}", language.to_lowercase(), region.to_uppercase())
        }
        None => lang.to_lowercase(),
    }
}

/// Generate the core docs package from Maxima's manual, in English or, with
/// `lang`, from the translation in `doc/info/<lang>/`
pub fn run(
    maxima_src: &str,
    output_dir: Option<&str>,
    no_build: bool,
    xml_dir: Option<&str>,
    mdbook: bool,
    lang: Option<&str>,
) -> Result<(), MxpmError> {
    let src = Path::new(maxima_src);
    let doc_info = src.join("doc").join("info");
    if !doc_info.exists() {
        return Err(MxpmError::MakeinfoFailed {
            message: format!("Maxima source directory not found: {}", doc_info.display()),
        });
    }
    // The manual to read: the English one, or a translation beside it
    let manual = match lang {
        Some(lang) => {
            let dir = doc_info.join(info_dir_name(lang));
            if !dir.join("maxima.texi").exists() {
                return Err(MxpmError::MakeinfoFailed {
                    message: format!("No Maxima manual in {lang}: {}", dir.display()),
                });
            }
            dir
        }
        None => doc_info.clone(),
    };

    let out = match output_dir {
        Some(d) => PathBuf::from(d),
        None => std::env::current_dir()?,
    };
    fs::create_dir_all(&out)?;

    // Use --xml-dir as persistent working directory, or a temp dir
    let _temp_dir; // keep alive for the duration of run()
    let work_path = if let Some(dir) = xml_dir {
        let p = PathBuf::from(dir);
        fs::create_dir_all(&p)?;
        p
    } else {
        _temp_dir = tempfile::tempdir()?;
        _temp_dir.path().to_path_buf()
    };

    // Step 1: Preprocess Texinfo
    eprintln!("Preprocessing Texinfo source...");
    // A translation includes what it lacks from the English tree, as
    // Maxima's own build does with `-I ..`
    let english = lang.map(|_| doc_info.as_path());
    let texi_path =
        preprocess::preprocess_texi(&manual, &work_path, &src.join("configure.ac"), english)?;

    // Step 2: Run makeinfo --xml
    eprintln!("Running makeinfo --xml...");
    let xml_path = preprocess::run_makeinfo_xml(&texi_path, &work_path, lang.is_some())?;
    if xml_dir.is_some() {
        eprintln!("XML saved to {}", xml_path.display());
    }

    // Step 3: Parse XML
    eprintln!("Parsing XML...");
    let xml_content = fs::read_to_string(&xml_path)?;
    let symbols = xml_parser::parse_xml(&xml_content)?;
    eprintln!("Extracted {} symbols", symbols.len());

    // Step 4: Group by category + subcategory and emit markdown
    let doc_dir = out.join("doc");
    fs::create_dir_all(&doc_dir)?;
    let category_groups = emit::emit_markdown_files(&symbols, &doc_dir)?;
    let file_count: usize = category_groups.iter().map(|(_, files)| files.len()).sum();
    eprintln!(
        "Wrote {} files across {} categories",
        file_count,
        category_groups.len()
    );

    // Step 5: Generate main doc file with includes
    let main_doc = match lang {
        Some(lang) => format!("maxima-core-docs.{lang}.md"),
        None => "maxima-core-docs.md".to_string(),
    };
    emit::emit_main_doc(&category_groups, &doc_dir.join(&main_doc))?;

    // Step 6: Copy figures, the translation's own or else the English ones
    let figures_src = match manual.join("figures") {
        own if own.is_dir() => own,
        _ => doc_info.join("figures"),
    };
    if figures_src.is_dir() {
        let figures_dst = doc_dir.join("figures");
        copy_dir(&figures_src, &figures_dst)?;
        let count = fs::read_dir(&figures_dst)?.count();
        eprintln!("Copied {} figures", count);
    }

    // Step 7: Generate manifest and placeholder .mac
    emit::emit_manifest(&out, src, lang)?;

    eprintln!("Generated package at {}", out.display());

    // Step 8: Optionally run mxpm doc build (always includes slim index)
    if !no_build {
        eprintln!("\nRunning doc build...");
        let md_path = doc_dir.join(&main_doc).to_string_lossy().to_string();
        let slim_path = doc_dir
            .join(crate::doc_index::slim_doc_index_file_name(
                "maxima-core-docs",
                lang.unwrap_or(crate::doc_index::ENGLISH),
            ))
            .to_string_lossy()
            .to_string();
        let out_str = out.to_string_lossy().to_string();
        match lang {
            Some(lang) => {
                super::run_translated_build(Some(&md_path), Some(&out_str), lang, Some(&slim_path))?
            }
            None => super::run_build(
                Some(&md_path),
                Some(&out_str),
                false,
                mdbook,
                Some(&slim_path),
            )?,
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_language_names_maxima_s_directory() {
        assert_eq!(info_dir_name("es"), "es");
        assert_eq!(info_dir_name("pt-BR"), "pt_BR");
        assert_eq!(info_dir_name("pt-br"), "pt_BR");
        assert_eq!(info_dir_name("pt_BR"), "pt_BR");
    }
}
