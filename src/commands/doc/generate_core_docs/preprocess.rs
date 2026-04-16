//! Texinfo source preprocessing and makeinfo XML generation.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::errors::MxpmError;

use super::copy_dir;

/// Preprocess Maxima's doc/info/ tree for XML generation.
///
/// Copies the source to a working directory, processes `.texi.in` and
/// `.texi.m4` templates, and patches `category-macros.texi` for XML-mode
/// figure handling.
pub(super) fn preprocess_texi(
    doc_info: &Path,
    work_dir: &Path,
) -> Result<PathBuf, MxpmError> {
    // Copy doc/info/ contents to working directory
    copy_dir(doc_info, work_dir)?;

    // Extract version from configure.ac
    let configure_ac = doc_info
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.join("configure.ac"))
        .unwrap_or_default();
    let version = if configure_ac.exists() {
        extract_version(&configure_ac)?
    } else {
        "5.47.0".to_string()
    };

    // Process .texi.in files
    process_texi_in_files(work_dir, &version)?;

    // Process .texi.m4 files
    process_texi_m4_files(work_dir)?;

    // Patch category-macros.texi for XML mode
    patch_category_macros(work_dir)?;

    let texi_path = work_dir.join("maxima.texi");
    if !texi_path.exists() {
        return Err(MxpmError::MakeinfoFailed {
            message: "maxima.texi not found after preprocessing".to_string(),
        });
    }
    Ok(texi_path)
}

pub(super) fn extract_version(configure_ac: &Path) -> Result<String, MxpmError> {
    let content = fs::read_to_string(configure_ac)?;
    // Look for AC_INIT([maxima], [5.47.0], ...)
    for line in content.lines() {
        if line.starts_with("AC_INIT")
            && let Some(start) = line.find('[')
                && let Some(mid) = line[start + 1..].find('[') {
                    let rest = &line[start + 1 + mid + 1..];
                    if let Some(end) = rest.find(']') {
                        return Ok(rest[..end].to_string());
                    }
                }
    }
    Ok("5.47.0".to_string())
}

fn process_texi_in_files(work_dir: &Path, version: &str) -> Result<(), MxpmError> {
    for entry in fs::read_dir(work_dir)? {
        let entry = entry?;
        let path = entry.path();
        if let Some(name) = path.file_name().and_then(|n| n.to_str())
            && name.ends_with(".texi.in") {
                let content = fs::read_to_string(&path)?;
                let output = content
                    .replace("@manual_version@", version)
                    .replace("@abs_srcdir@", &work_dir.to_string_lossy());
                let out_name = name.strip_suffix(".in").unwrap();
                fs::write(work_dir.join(out_name), output)?;
            }
    }
    Ok(())
}

fn process_texi_m4_files(work_dir: &Path) -> Result<(), MxpmError> {
    // Copy math.m4.in to math.m4 if it exists
    let m4_in = work_dir.join("math.m4.in");
    let m4_out = work_dir.join("math.m4");
    if m4_in.exists() {
        fs::copy(&m4_in, &m4_out)?;
    }

    let mut m4_files = Vec::new();
    for entry in fs::read_dir(work_dir)? {
        let entry = entry?;
        let path = entry.path();
        if let Some(name) = path.file_name().and_then(|n| n.to_str())
            && name.ends_with(".texi.m4") {
                m4_files.push(path.clone());
            }
    }

    for m4_file in m4_files {
        let stem = m4_file
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .strip_suffix(".m4")
            .unwrap();
        let out_path = work_dir.join(stem);

        if m4_out.exists() {
            let output = Command::new("m4")
                .arg("--prefix-builtins")
                .arg(&m4_out)
                .arg(&m4_file)
                .current_dir(work_dir)
                .output()
                .map_err(|e| MxpmError::MakeinfoFailed {
                    message: format!("Failed to run m4: {e}"),
                })?;
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                eprintln!("Warning: m4 failed for {}: {}", m4_file.display(), stderr);
                // Fall back to copying without m4
                fs::copy(&m4_file, &out_path)?;
            } else {
                fs::write(&out_path, &output.stdout)?;
            }
        } else {
            // No math.m4, just copy
            fs::copy(&m4_file, &out_path)?;
        }
    }
    Ok(())
}

fn patch_category_macros(work_dir: &Path) -> Result<(), MxpmError> {
    let path = work_dir.join("category-macros.texi");
    if !path.exists() {
        return Ok(());
    }
    let mut content = fs::read_to_string(&path)?;
    // Append XML-mode no-op macros for figure handling
    content.push_str(
        r#"
@ifxml
@unmacro figure
@macro figure {file}
(Figure \file\)
@end macro
@unmacro smallfigure
@macro smallfigure {file}
(Figure \file\)
@end macro
@unmacro altfigure
@macro altfigure {file, altfile}
(Figure \file\)
@end macro
@end ifxml
"#,
    );
    fs::write(&path, content)?;
    Ok(())
}

/// Run `makeinfo --xml` on the preprocessed Texinfo source.
pub(super) fn run_makeinfo_xml(
    texi_path: &Path,
    work_dir: &Path,
) -> Result<PathBuf, MxpmError> {
    let xml_path = work_dir.join("maxima.xml");
    let output = Command::new("makeinfo")
        .args(["--xml", "--no-warn"])
        .arg(texi_path)
        .arg("-o")
        .arg(&xml_path)
        .current_dir(work_dir)
        .output()
        .map_err(|e| MxpmError::MakeinfoFailed {
            message: format!("Failed to run makeinfo: {e}"),
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(MxpmError::MakeinfoFailed {
            message: format!("makeinfo --xml failed:\n{stderr}"),
        });
    }

    Ok(xml_path)
}
