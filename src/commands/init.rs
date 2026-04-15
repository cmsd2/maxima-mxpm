use std::fs;
use std::path::Path;

use serde::Serialize;
use tera::{Context, Tera};

use crate::errors::MxpmError;
use crate::output::{self, OutputFormat};

/// Validate a package name according to mxpm conventions.
///
/// Rules:
/// - 2–64 characters
/// - Lowercase ASCII letters, digits, and hyphens only
/// - Must start with a letter
/// - Cannot start with `maxima-`
pub fn validate_package_name(name: &str) -> Result<(), MxpmError> {
    if name.len() < 2 || name.len() > 64 {
        return Err(MxpmError::InvalidPackageName(format!(
            "'{name}' must be between 2 and 64 characters"
        )));
    }

    if !name.starts_with(|c: char| c.is_ascii_lowercase()) {
        return Err(MxpmError::InvalidPackageName(format!(
            "'{name}' must start with a lowercase letter"
        )));
    }

    if let Some(c) = name
        .chars()
        .find(|c| !c.is_ascii_lowercase() && !c.is_ascii_digit() && *c != '-')
    {
        return Err(MxpmError::InvalidPackageName(format!(
            "'{name}' contains invalid character '{c}'; only lowercase letters, digits, and hyphens are allowed"
        )));
    }

    if name.starts_with("maxima-") {
        return Err(MxpmError::InvalidPackageName(format!(
            "'{name}' cannot start with 'maxima-'"
        )));
    }

    Ok(())
}

#[derive(Serialize)]
struct InitResult {
    name: String,
    path: String,
    template: String,
    files: Vec<String>,
}

/// Template definitions: (filename_template, content_template)
const BASIC_TEMPLATES: &[(&str, &str)] = &[
    (
        "manifest.toml",
        include_str!("../../templates/basic/manifest.toml.tera"),
    ),
    (
        "{{ name }}.mac",
        include_str!("../../templates/basic/entry.mac.tera"),
    ),
    (
        "rtest_{{ name }}.mac",
        include_str!("../../templates/basic/rtest.mac.tera"),
    ),
    (
        "doc/{{ name }}.md",
        include_str!("../../templates/basic/doc.md.tera"),
    ),
    (
        ".github/workflows/docs.yml",
        include_str!("../../templates/basic/docs-ci.yml.tera"),
    ),
    (
        ".github/workflows/pages.yml",
        include_str!("../../templates/basic/pages.yml.tera"),
    ),
    (
        "README.md",
        include_str!("../../templates/basic/README.md.tera"),
    ),
    (
        ".gitignore",
        include_str!("../../templates/basic/gitignore.tera"),
    ),
];

pub fn run(
    name: &str,
    path: Option<&str>,
    template: &str,
    format: OutputFormat,
) -> Result<(), MxpmError> {
    validate_package_name(name)?;

    if template != "basic" {
        return Err(MxpmError::InvalidPackageName(format!(
            "unknown template '{template}'; available templates: basic"
        )));
    }

    let target = Path::new(path.unwrap_or(name));

    if target.exists() {
        return Err(MxpmError::Io(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            format!("directory '{}' already exists", target.display()),
        )));
    }

    fs::create_dir_all(target)?;

    let files = generate_from_templates(name, target, BASIC_TEMPLATES)?;

    match format {
        OutputFormat::Json => {
            output::print_json(&InitResult {
                name: name.to_string(),
                path: target.display().to_string(),
                template: template.to_string(),
                files,
            })?;
        }
        OutputFormat::Human => {
            eprintln!("Created package '{name}' in {}", target.display());
            eprintln!();
            eprintln!("Next steps:");
            eprintln!("  cd {}", target.display());
            eprintln!("  git init");
            eprintln!("  # Edit manifest.toml with your package details");
            eprintln!("  # Edit {name}.mac with your code");
        }
    }

    Ok(())
}

fn generate_from_templates(
    name: &str,
    dir: &Path,
    templates: &[(&str, &str)],
) -> Result<Vec<String>, MxpmError> {
    let mut tera = Tera::default();
    let mut context = Context::new();
    context.insert("name", name);

    let mut created = Vec::new();
    for (filename_tpl, content_tpl) in templates {
        // Render the filename (may contain {{ name }})
        let filename = tera
            .render_str(filename_tpl, &context)
            .map_err(|e| MxpmError::Io(std::io::Error::other(e)))?;

        // Render the content
        let content = tera
            .render_str(content_tpl, &context)
            .map_err(|e| MxpmError::Io(std::io::Error::other(e)))?;

        let file_path = dir.join(&filename);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&file_path, &content)?;
        created.push(filename);
    }

    Ok(created)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn valid_names() {
        assert!(validate_package_name("my-pkg").is_ok());
        assert!(validate_package_name("ab").is_ok());
        assert!(validate_package_name("foo123").is_ok());
        assert!(validate_package_name("a-b-c").is_ok());
    }

    #[test]
    fn name_too_short() {
        assert!(validate_package_name("a").is_err());
    }

    #[test]
    fn name_too_long() {
        let long = "a".repeat(65);
        assert!(validate_package_name(&long).is_err());
    }

    #[test]
    fn name_starts_with_digit() {
        assert!(validate_package_name("1pkg").is_err());
    }

    #[test]
    fn name_starts_with_hyphen() {
        assert!(validate_package_name("-pkg").is_err());
    }

    #[test]
    fn name_uppercase() {
        assert!(validate_package_name("MyPkg").is_err());
    }

    #[test]
    fn name_underscore() {
        assert!(validate_package_name("my_pkg").is_err());
    }

    #[test]
    fn name_maxima_prefix() {
        assert!(validate_package_name("maxima-foo").is_err());
    }

    #[test]
    fn init_creates_files() {
        let tmp = TempDir::new().unwrap();
        let target = tmp.path().join("test-pkg");

        run(
            "test-pkg",
            Some(target.to_str().unwrap()),
            "basic",
            OutputFormat::Human,
        )
        .unwrap();

        assert!(target.join("manifest.toml").exists());
        assert!(target.join("test-pkg.mac").exists());
        assert!(target.join("rtest_test-pkg.mac").exists());
        assert!(target.join("doc/test-pkg.md").exists());
        assert!(target.join(".github/workflows/docs.yml").exists());
        assert!(target.join(".github/workflows/pages.yml").exists());
        assert!(target.join("README.md").exists());
        assert!(target.join(".gitignore").exists());

        let manifest = fs::read_to_string(target.join("manifest.toml")).unwrap();
        assert!(manifest.contains("name = \"test-pkg\""));
        assert!(manifest.contains("entry = \"test-pkg.mac\""));
        assert!(manifest.contains("doc = \"doc/test-pkg.md\""));

        let entry = fs::read_to_string(target.join("test-pkg.mac")).unwrap();
        assert!(entry.contains("load(\"test-pkg-index.lisp\")"));

        let doc = fs::read_to_string(target.join("doc/test-pkg.md")).unwrap();
        assert!(doc.contains("# Package test-pkg"));
    }

    #[test]
    fn init_errors_on_existing_dir() {
        let tmp = TempDir::new().unwrap();
        let target = tmp.path().join("existing");
        fs::create_dir(&target).unwrap();

        let result = run(
            "existing",
            Some(target.to_str().unwrap()),
            "basic",
            OutputFormat::Human,
        );
        assert!(result.is_err());
    }

    #[test]
    fn init_unknown_template() {
        let tmp = TempDir::new().unwrap();
        let target = tmp.path().join("test-pkg");

        let result = run(
            "test-pkg",
            Some(target.to_str().unwrap()),
            "ffi",
            OutputFormat::Human,
        );
        assert!(result.is_err());
    }
}
