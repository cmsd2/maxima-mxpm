use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct PackageIndex {
    pub version: u32,
    pub packages: HashMap<String, PackageEntry>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PackageEntry {
    pub description: String,
    pub repository: String,
    pub source: Source,
    pub homepage: Option<String>,
    pub keywords: Option<Vec<String>>,
    pub license: Option<String>,
    pub authors: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Source {
    Tarball {
        url: String,
        /// Expected SHA-256 hash of the tarball (hex-encoded).
        #[serde(skip_serializing_if = "Option::is_none")]
        hash: Option<String>,
        /// Hash algorithm (defaults to "sha256").
        #[serde(skip_serializing_if = "Option::is_none")]
        hash_algorithm: Option<String>,
    },
    Git {
        url: String,
        #[serde(rename = "ref")]
        git_ref: String,
        subdir: Option<String>,
    },
}

impl Source {
    /// Return the subdir filter, if any.
    pub fn subdir(&self) -> Option<&str> {
        match self {
            Source::Git { subdir, .. } => subdir.as_deref(),
            Source::Tarball { .. } => None,
        }
    }
}
