//! Package manifest (`manifest.toml`) types and parser.

use serde::Deserialize;

/// Parsed contents of a package's `manifest.toml`.
#[derive(Debug, Deserialize)]
pub struct Manifest {
    pub package: PackageInfo,
    pub lisp: Option<LispInfo>,
    pub test: Option<TestInfo>,
}

#[derive(Debug, Deserialize)]
pub struct PackageInfo {
    pub name: String,
    pub version: String,
    pub description: String,
    pub license: String,
    pub entry: String,
    pub authors: Option<AuthorInfo>,
    pub homepage: Option<String>,
    pub repository: Option<String>,
    pub keywords: Option<Vec<String>>,
    pub maxima: Option<String>,
    pub doc: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TestInfo {
    pub files: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct AuthorInfo {
    pub names: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct LispInfo {
    pub quicklisp_systems: Option<Vec<String>>,
}

/// Try to parse a manifest.toml from its string contents.
pub fn parse_manifest(contents: &str) -> Result<Manifest, toml::de::Error> {
    toml::from_str(contents)
}

/// Read and parse a manifest.toml from a directory.
///
/// Looks for `manifest.toml` in the given directory.
pub fn load_manifest(dir: &std::path::Path) -> Option<Manifest> {
    let path = dir.join("manifest.toml");
    let content = std::fs::read_to_string(&path).ok()?;
    parse_manifest(&content).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimal_manifest() {
        let toml = r#"
[package]
name = "test-pkg"
version = "1.0.0"
description = "A test"
license = "MIT"
entry = "test.mac"
"#;
        let m = parse_manifest(toml).unwrap();
        assert_eq!(m.package.name, "test-pkg");
        assert_eq!(m.package.version, "1.0.0");
        assert_eq!(m.package.entry, "test.mac");
        assert!(m.package.authors.is_none());
    }

    #[test]
    fn parse_full_manifest() {
        let toml = r#"
[package]
name = "diophantine"
version = "1.2.0"
description = "Solver for Diophantine equations"
license = "GPL-3.0-or-later"
entry = "diophantine.mac"
homepage = "https://example.com"
repository = "https://github.com/example/repo"
keywords = ["math", "number-theory"]
maxima = ">= 5.47"

[package.authors]
names = ["Test Author", "Another Author"]
"#;
        let m = parse_manifest(toml).unwrap();
        assert_eq!(m.package.name, "diophantine");
        assert_eq!(m.package.homepage.unwrap(), "https://example.com");
        assert_eq!(m.package.keywords.unwrap().len(), 2);
        let authors = m.package.authors.unwrap();
        assert_eq!(authors.names.len(), 2);
    }

    #[test]
    fn parse_manifest_with_doc() {
        let toml = r#"
[package]
name = "my-pkg"
version = "1.0.0"
description = "A package with docs"
license = "MIT"
entry = "my-pkg.mac"
doc = "doc/my-pkg.md"
"#;
        let m = parse_manifest(toml).unwrap();
        assert_eq!(m.package.doc.unwrap(), "doc/my-pkg.md");
    }

    #[test]
    fn parse_manifest_with_test() {
        let toml = r#"
[package]
name = "my-pkg"
version = "1.0.0"
description = "A package with tests"
license = "MIT"
entry = "my-pkg.mac"

[test]
files = ["rtest_my-pkg.mac", "rtest_extra.mac"]
"#;
        let m = parse_manifest(toml).unwrap();
        let test = m.test.unwrap();
        assert_eq!(test.files, vec!["rtest_my-pkg.mac", "rtest_extra.mac"]);
    }

    #[test]
    fn parse_manifest_with_lisp() {
        let toml = r#"
[package]
name = "numerics"
version = "0.1.0"
description = "NumPy-like numerical computing"
license = "MIT"
entry = "numerics.mac"

[lisp]
quicklisp_systems = ["magicl"]
"#;
        let m = parse_manifest(toml).unwrap();
        let lisp = m.lisp.unwrap();
        assert_eq!(lisp.quicklisp_systems.unwrap(), vec!["magicl"]);
    }

    #[test]
    fn missing_required_field() {
        let toml = r#"
[package]
name = "test"
"#;
        assert!(parse_manifest(toml).is_err());
    }
}
