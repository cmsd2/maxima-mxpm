use serde::{Deserialize, Serialize};

use crate::index::Source;

/// Metadata written to `.mxpm.json` at install time.
#[derive(Debug, Serialize, Deserialize)]
pub struct InstallMetadata {
    pub name: String,
    pub version: Option<String>,
    pub installed_at: String,
    /// The source used at install time, with resolved values:
    /// - git: `ref` is the actual commit hash that was checked out
    /// - tarball: `hash`/`hash_algorithm` are filled in from the download
    pub source: Source,
    pub registry: String,
}
