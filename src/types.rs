use serde::{Deserialize, Serialize};

use crate::index::Source;

/// Metadata written to `.mxpm.json` at install time.
#[derive(Debug, Serialize, Deserialize)]
pub struct InstallMetadata {
    pub name: String,
    pub version: Option<String>,
    pub installed_at: String,
    pub source: Source,
    pub registry: String,
    /// The URL that was fetched (git clone URL or tarball URL).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// The resolved git commit hash at install time (if source was git).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit: Option<String>,
    /// Hex-encoded hash of the downloaded tarball (if source was tarball).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
    /// Hash algorithm used (e.g. "sha256").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash_algorithm: Option<String>,
}
