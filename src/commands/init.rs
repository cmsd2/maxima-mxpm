use std::fs;
use std::path::Path;

use serde::Serialize;
use tera::{Context, Tera};

use crate::errors::MxpmError;
use crate::output::{self, OutputFormat};

use super::new::{BASIC_TEMPLATES, validate_package_name};

#[derive(Serialize)]
struct InitResult {
    name: String,
    entry: String,
    files: Vec<String>,
}

/// Render a single template file into `dir`, skipping if it already exists.
/// Returns the filename if written, None if skipped.
fn render_template(
    tera: &mut Tera,
    context: &Context,
    dir: &Path,
    filename_tpl: &str,
    content_tpl: &str,
) -> Result<Option<String>, MxpmError> {
    let filename = tera
        .render_str(filename_tpl, context)
        .map_err(|e| MxpmError::Io(std::io::Error::other(e)))?;

    let file_path = dir.join(&filename);
    if file_path.exists() {
        return Ok(None);
    }

    let content = tera
        .render_str(content_tpl, context)
        .map_err(|e| MxpmError::Io(std::io::Error::other(e)))?;

    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&file_path, &content)?;
    Ok(Some(filename))
}

/// Labels and template filename prefixes for optional extras.
const EXTRAS: &[(&str, &[&str])] = &[
    (".gitignore", &[".gitignore"]),
    ("README.md", &["README.md"]),
    ("Doc template", &["doc/"]),
    ("Test file", &["rtest_"]),
    ("CI workflows", &[".github/workflows/"]),
];

pub fn run(
    name: Option<&str>,
    yes: bool,
    format: OutputFormat,
    _config: &crate::config::Config,
) -> Result<(), MxpmError> {
    let dir = std::env::current_dir()?;
    run_in_dir(&dir, name, yes, format)
}

fn run_in_dir(
    dir: &Path,
    name: Option<&str>,
    yes: bool,
    format: OutputFormat,
) -> Result<(), MxpmError> {
    // 1. Check not already initialized
    if dir.join("manifest.toml").exists() {
        return Err(MxpmError::PublishFailed {
            message: "manifest.toml already exists; this directory is already a package"
                .to_string(),
        });
    }

    // 2. Determine package name
    let pkg_name = match name {
        Some(n) => {
            validate_package_name(n)?;
            n.to_string()
        }
        None => {
            let dir_name = dir
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("my-package")
                .to_string();
            validate_package_name(&dir_name).map_err(|_| {
                MxpmError::InvalidPackageName(format!(
                    "directory name '{dir_name}' is not a valid package name; use --name to specify one"
                ))
            })?;
            dir_name
        }
    };

    // 3. Auto-detect entry point
    let mut mac_files: Vec<String> = fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            if name.ends_with(".mac") && e.file_type().map(|t| t.is_file()).unwrap_or(false) {
                Some(name)
            } else {
                None
            }
        })
        .collect();
    mac_files.sort();

    let entry = match mac_files.len() {
        0 => {
            let default = format!("{pkg_name}.mac");
            if matches!(format, OutputFormat::Human) {
                eprintln!("No .mac files found; entry point will be {default}");
            }
            default
        }
        1 => {
            let entry = mac_files[0].clone();
            if matches!(format, OutputFormat::Human) {
                eprintln!("Detected entry point: {entry}");
            }
            entry
        }
        _ => {
            if yes {
                let entry = mac_files[0].clone();
                if matches!(format, OutputFormat::Human) {
                    eprintln!("Multiple .mac files found; using {entry}");
                }
                entry
            } else {
                let selection = dialoguer::Select::new()
                    .with_prompt("Multiple .mac files found. Which is the entry point?")
                    .items(&mac_files)
                    .default(0)
                    .interact()
                    .map_err(|e| MxpmError::Io(std::io::Error::other(e)))?;
                mac_files[selection].clone()
            }
        }
    };

    // 4. Determine which extras to generate
    let selections: Vec<usize> = if !yes && matches!(format, OutputFormat::Human) {
        let labels: Vec<&str> = EXTRAS.iter().map(|(label, _)| *label).collect();
        let defaults: Vec<bool> = vec![true; labels.len()];

        dialoguer::MultiSelect::new()
            .with_prompt("Generate additional files")
            .items(&labels)
            .defaults(&defaults)
            .interact()
            .map_err(|e| MxpmError::Io(std::io::Error::other(e)))?
    } else {
        vec![]
    };

    // Check if doc/test extras were selected (affects manifest content)
    let selected_prefixes: Vec<&str> = selections
        .iter()
        .flat_map(|&idx| EXTRAS[idx].1.iter().copied())
        .collect();
    let include_doc = selected_prefixes.iter().any(|p| p.starts_with("doc/"));
    let include_test = selected_prefixes.iter().any(|p| p.starts_with("rtest_"));

    // 5. Set up template context
    let mut tera = Tera::default();
    let mut context = Context::new();
    context.insert("name", &pkg_name);
    context.insert("entry", &entry);
    context.insert("mac_name", &pkg_name.replace('-', "_"));
    context.insert("include_doc", &include_doc);
    context.insert("include_test", &include_test);

    // 6. Generate manifest.toml from template
    let manifest_tpl = BASIC_TEMPLATES
        .iter()
        .find(|(name, _)| *name == "manifest.toml")
        .expect("manifest.toml template must exist");

    render_template(&mut tera, &context, dir, manifest_tpl.0, manifest_tpl.1)?;

    let mut created = vec!["manifest.toml".to_string()];

    if matches!(format, OutputFormat::Human) {
        eprintln!("Created manifest.toml");
    }

    // 7. Generate selected extras
    for idx in &selections {
        let (_, prefixes) = EXTRAS[*idx];
        for (tpl_name, tpl_content) in BASIC_TEMPLATES {
            if prefixes.iter().any(|p| tpl_name.starts_with(p)) {
                if let Some(f) = render_template(&mut tera, &context, dir, tpl_name, tpl_content)? {
                    if matches!(format, OutputFormat::Human) {
                        eprintln!("  Created {f}");
                    }
                    created.push(f);
                } else if matches!(format, OutputFormat::Human) {
                    eprintln!("  {} already exists, skipped.", tpl_name);
                }
            }
        }
    }

    // 8. Summary
    match format {
        OutputFormat::Json => {
            output::print_json(&InitResult {
                name: pkg_name,
                entry,
                files: created,
            })?;
        }
        OutputFormat::Human => {
            eprintln!();
            eprintln!("Package initialized. Edit manifest.toml to fill in details.");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn init_creates_manifest() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("my-pkg");
        fs::create_dir(&dir).unwrap();

        run_in_dir(&dir, Some("my-pkg"), true, OutputFormat::Human).unwrap();

        assert!(dir.join("manifest.toml").exists());
        let manifest = fs::read_to_string(dir.join("manifest.toml")).unwrap();
        assert!(manifest.contains("name = \"my-pkg\""));
        assert!(manifest.contains("entry = \"my-pkg.mac\""));
    }

    #[test]
    fn init_detects_single_mac_file() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("test-pkg");
        fs::create_dir(&dir).unwrap();
        fs::write(dir.join("solver.mac"), "/* code */").unwrap();

        run_in_dir(&dir, Some("test-pkg"), true, OutputFormat::Human).unwrap();

        let manifest = fs::read_to_string(dir.join("manifest.toml")).unwrap();
        assert!(manifest.contains("entry = \"solver.mac\""));
    }

    #[test]
    fn init_errors_if_manifest_exists() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("existing");
        fs::create_dir(&dir).unwrap();
        fs::write(dir.join("manifest.toml"), "[package]").unwrap();

        let result = run_in_dir(&dir, Some("existing"), true, OutputFormat::Human);
        assert!(result.is_err());
    }

    #[test]
    fn init_picks_first_mac_with_yes() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("multi");
        fs::create_dir(&dir).unwrap();
        fs::write(dir.join("alpha.mac"), "/* a */").unwrap();
        fs::write(dir.join("beta.mac"), "/* b */").unwrap();

        run_in_dir(&dir, Some("multi"), true, OutputFormat::Human).unwrap();

        let manifest = fs::read_to_string(dir.join("manifest.toml")).unwrap();
        assert!(manifest.contains("entry = \"alpha.mac\""));
    }

    #[test]
    fn init_infers_name_from_dir() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("cool-pkg");
        fs::create_dir(&dir).unwrap();

        run_in_dir(&dir, None, true, OutputFormat::Human).unwrap();

        let manifest = fs::read_to_string(dir.join("manifest.toml")).unwrap();
        assert!(manifest.contains("name = \"cool-pkg\""));
    }
}
