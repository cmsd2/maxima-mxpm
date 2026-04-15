use serde::Deserialize;

/// Parsed contents of a package's `manifest.toml`.
#[derive(Debug, Deserialize)]
pub struct Manifest {
    pub package: PackageInfo,
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
}

#[derive(Debug, Deserialize)]
pub struct AuthorInfo {
    pub names: Vec<String>,
}

/// Try to parse a manifest.toml from its string contents.
pub fn parse_manifest(contents: &str) -> Result<Manifest, toml::de::Error> {
    toml::from_str(contents)
}
