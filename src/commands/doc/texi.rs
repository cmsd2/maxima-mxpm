//! Texinfo generation: Pandoc invocation, makeinfo, and post-processing.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use regex::Regex;

use crate::errors::MxpmError;

/// Invoke `pandoc` to convert a Markdown file to Texinfo.
pub(super) fn invoke_pandoc(md_path: &str, texi_dest: &Path) -> Result<(), MxpmError> {
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
        .arg("--wrap=none")
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

/// Invoke `makeinfo` to compile a `.texi` file into `.info`.
pub(super) fn invoke_makeinfo(texi_path: &str) -> Result<PathBuf, MxpmError> {
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
pub(super) fn invoke_makeinfo_xml(
    texi_path: &str,
    output_dir: &Path,
) -> Result<PathBuf, MxpmError> {
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

/// Post-process Pandoc's Texinfo output to add `@deffn`/`@defvr` blocks.
///
/// Converts headings matching our convention:
///   `@subsection Function: name (args)` -> `@deffn {Function} name (@var{args})`
///   `@subsection Variable: name`        -> `@defvr {Variable} name`
///
/// Also injects `@setfilename`, `@printindex fn`, and `@printindex vr`.
pub(super) fn postprocess_texi(texi_path: &Path, stem: &str) -> Result<(), MxpmError> {
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

/// Read the `@setfilename` directive from a `.texi` file.
pub(super) fn read_setfilename(texi_path: &Path) -> Option<String> {
    let content = fs::read_to_string(texi_path).ok()?;
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("@setfilename ") {
            return Some(rest.trim().to_string());
        }
    }
    None
}
