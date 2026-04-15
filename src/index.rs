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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_fixture_index() {
        let contents = include_str!("../tests/fixtures/index.json");
        let index: PackageIndex = serde_json::from_str(contents).unwrap();
        assert_eq!(index.version, 1);
        assert_eq!(index.packages.len(), 3);
    }

    #[test]
    fn parse_git_source() {
        let json = r#"{"type":"git","url":"https://example.com/repo.git","ref":"abc123"}"#;
        let source: Source = serde_json::from_str(json).unwrap();
        match source {
            Source::Git {
                url,
                git_ref,
                subdir,
            } => {
                assert_eq!(url, "https://example.com/repo.git");
                assert_eq!(git_ref, "abc123");
                assert!(subdir.is_none());
            }
            _ => panic!("expected git source"),
        }
    }

    #[test]
    fn parse_git_source_with_subdir() {
        let json = r#"{"type":"git","url":"https://example.com/repo.git","ref":"abc123","subdir":"pkg/foo"}"#;
        let source: Source = serde_json::from_str(json).unwrap();
        assert_eq!(source.subdir(), Some("pkg/foo"));
    }

    #[test]
    fn parse_tarball_source() {
        let json = r#"{"type":"tarball","url":"https://example.com/pkg.tar.gz"}"#;
        let source: Source = serde_json::from_str(json).unwrap();
        match source {
            Source::Tarball {
                url,
                hash,
                hash_algorithm,
            } => {
                assert_eq!(url, "https://example.com/pkg.tar.gz");
                assert!(hash.is_none());
                assert!(hash_algorithm.is_none());
            }
            _ => panic!("expected tarball source"),
        }
    }

    #[test]
    fn parse_tarball_source_with_hash() {
        let json = r#"{"type":"tarball","url":"https://example.com/pkg.tar.gz","hash":"abcdef","hash_algorithm":"sha256"}"#;
        let source: Source = serde_json::from_str(json).unwrap();
        match source {
            Source::Tarball {
                hash,
                hash_algorithm,
                ..
            } => {
                assert_eq!(hash.unwrap(), "abcdef");
                assert_eq!(hash_algorithm.unwrap(), "sha256");
            }
            _ => panic!("expected tarball source"),
        }
    }

    #[test]
    fn source_equality() {
        let a = Source::Git {
            url: "https://example.com/repo.git".into(),
            git_ref: "abc123".into(),
            subdir: None,
        };
        let b = Source::Git {
            url: "https://example.com/repo.git".into(),
            git_ref: "abc123".into(),
            subdir: None,
        };
        let c = Source::Git {
            url: "https://example.com/repo.git".into(),
            git_ref: "def456".into(),
            subdir: None,
        };
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn roundtrip_serialize() {
        let source = Source::Git {
            url: "https://example.com/repo.git".into(),
            git_ref: "abc123".into(),
            subdir: Some("sub/dir".into()),
        };
        let json = serde_json::to_string(&source).unwrap();
        let parsed: Source = serde_json::from_str(&json).unwrap();
        assert_eq!(source, parsed);
    }

    #[test]
    fn rejects_unsupported_version() {
        let json = r#"{"version":99,"packages":{}}"#;
        let index: PackageIndex = serde_json::from_str(json).unwrap();
        assert_eq!(index.version, 99);
    }

    #[test]
    fn empty_packages() {
        let json = r#"{"version":1,"packages":{}}"#;
        let index: PackageIndex = serde_json::from_str(json).unwrap();
        assert!(index.packages.is_empty());
    }
}
