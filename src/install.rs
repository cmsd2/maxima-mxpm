use std::path::Path;

use crate::config::Config;
use crate::errors::MxpmError;
use crate::index::{PackageEntry, Source};
use crate::manifest;
use crate::source::DownloadResult;
use crate::types::InstallMetadata;

/// Install a package to the Maxima user directory.
///
/// Returns the install metadata on success.
pub async fn install_package(
    name: &str,
    entry: &PackageEntry,
    registry_name: &str,
    config: &Config,
) -> Result<InstallMetadata, MxpmError> {
    let userdir = crate::paths::maxima_userdir(config)?;
    let package_dir = userdir.join(name);
    let metadata_path = package_dir.join(".mxpm.json");

    // Check if already installed
    if metadata_path.exists() {
        return Err(MxpmError::AlreadyInstalled {
            name: name.to_string(),
        });
    }

    // Ensure the userdir exists
    std::fs::create_dir_all(&userdir).map_err(MxpmError::Io)?;

    // Download to a staging directory for atomic install
    let staging_dir = userdir.join(".mxpm_staging");
    let staging_pkg = staging_dir.join(name);
    if staging_pkg.exists() {
        std::fs::remove_dir_all(&staging_pkg).map_err(MxpmError::Io)?;
    }

    match &entry.source {
        crate::index::Source::Git { url, git_ref, .. } => {
            let short_ref = if git_ref.len() == 40 {
                &git_ref[..12]
            } else {
                git_ref
            };
            eprintln!("Cloning {url} ({short_ref})...");
        }
        crate::index::Source::Tarball { url, .. } => {
            eprintln!("Downloading {url}...");
        }
        crate::index::Source::Local { .. } => {
            // Local sources are handled by install_local_package
            unreachable!("install_package should not be called with a Local source");
        }
    }
    let download_result = crate::source::download_and_extract(&entry.source, &staging_pkg).await?;

    // Try to read manifest.toml for version info
    let version = read_version_from_staging(&staging_pkg);

    // Move from staging to final location
    eprintln!("Installing to {}/", package_dir.display());
    if package_dir.exists() {
        std::fs::remove_dir_all(&package_dir).map_err(MxpmError::Io)?;
    }
    std::fs::rename(&staging_pkg, &package_dir).map_err(MxpmError::Io)?;

    // Clean up staging dir if empty
    let _ = std::fs::remove_dir(&staging_dir);

    // Build a resolved source that reflects what was actually installed
    let resolved_source = resolve_source(&entry.source, &download_result);

    // Write install metadata
    let metadata = InstallMetadata {
        name: name.to_string(),
        version,
        installed_at: chrono::Utc::now().to_rfc3339(),
        source: resolved_source,
        registry: registry_name.to_string(),
    };
    let metadata_json = serde_json::to_string_pretty(&metadata)
        .map_err(|e| MxpmError::Io(std::io::Error::other(e)))?;
    std::fs::write(package_dir.join(".mxpm.json"), metadata_json).map_err(MxpmError::Io)?;

    Ok(metadata)
}

/// Read version from manifest.toml in a staging directory, if present.
fn read_version_from_staging(staging_dir: &Path) -> Option<String> {
    let manifest_path = staging_dir.join("manifest.toml");
    let contents = std::fs::read_to_string(manifest_path).ok()?;
    let m = manifest::parse_manifest(&contents).ok()?;
    Some(m.package.version)
}

/// Build a resolved Source from the index entry and actual download result.
/// For git: replaces ref with the resolved commit hash.
/// For tarball: fills in the computed hash.
fn resolve_source(original: &Source, result: &DownloadResult) -> Source {
    match original {
        Source::Git { url, subdir, .. } => Source::Git {
            url: url.clone(),
            git_ref: result.commit.clone().unwrap_or_default(),
            subdir: subdir.clone(),
        },
        Source::Tarball { url, .. } => Source::Tarball {
            url: url.clone(),
            hash: result.hash.clone(),
            hash_algorithm: result.hash_algorithm.clone(),
        },
        Source::Local { .. } => original.clone(),
    }
}

/// Install a package from a local directory.
///
/// In copy mode, the source directory is copied to `~/.maxima/<name>/`.
/// In editable mode, a symlink is created from `~/.maxima/<name>/` to the source directory.
pub fn install_local_package(
    name: &str,
    source_path: &Path,
    editable: bool,
    config: &Config,
) -> Result<InstallMetadata, MxpmError> {
    let abs_source = source_path.canonicalize().map_err(MxpmError::Io)?;

    // Read manifest to get version
    let manifest_path = abs_source.join("manifest.toml");
    if !manifest_path.exists() {
        return Err(MxpmError::ManifestNotFound {
            path: abs_source.display().to_string(),
        });
    }
    let manifest_contents = std::fs::read_to_string(&manifest_path).map_err(MxpmError::Io)?;
    let m = manifest::parse_manifest(&manifest_contents)
        .map_err(|e| MxpmError::Io(std::io::Error::other(e)))?;
    let version = Some(m.package.version);

    let userdir = crate::paths::maxima_userdir(config)?;
    let package_dir = userdir.join(name);

    // Check if already installed
    if package_dir.exists() {
        return Err(MxpmError::AlreadyInstalled {
            name: name.to_string(),
        });
    }

    std::fs::create_dir_all(&userdir).map_err(MxpmError::Io)?;

    let source = Source::Local {
        path: abs_source.display().to_string(),
        editable,
    };

    if editable {
        // Create symlink
        #[cfg(unix)]
        std::os::unix::fs::symlink(&abs_source, &package_dir).map_err(MxpmError::Io)?;
        #[cfg(windows)]
        std::os::windows::fs::symlink_dir(&abs_source, &package_dir).map_err(MxpmError::Io)?;

        eprintln!(
            "Linked {} -> {}",
            package_dir.display(),
            abs_source.display()
        );
    } else {
        // Copy
        eprintln!("Copying to {}...", package_dir.display());
        crate::source::copy_dir_recursive(&abs_source, &package_dir)?;
    }

    let metadata = InstallMetadata {
        name: name.to_string(),
        version,
        installed_at: chrono::Utc::now().to_rfc3339(),
        source,
        registry: "local".to_string(),
    };
    let metadata_json = serde_json::to_string_pretty(&metadata)
        .map_err(|e| MxpmError::Io(std::io::Error::other(e)))?;
    // For editable, this writes into the source dir (via the symlink).
    // For copy, this writes into the copied dir.
    std::fs::write(package_dir.join(".mxpm.json"), metadata_json).map_err(MxpmError::Io)?;

    Ok(metadata)
}

/// Read install metadata from an installed package.
pub fn read_install_metadata(package_dir: &Path) -> Result<InstallMetadata, MxpmError> {
    let metadata_path = package_dir.join(".mxpm.json");
    let contents = std::fs::read_to_string(&metadata_path).map_err(MxpmError::Io)?;
    let metadata: InstallMetadata =
        serde_json::from_str(&contents).map_err(|e| MxpmError::Io(std::io::Error::other(e)))?;
    Ok(metadata)
}

/// List all installed packages by scanning the Maxima user directory.
pub fn list_installed(config: &Config) -> Result<Vec<InstallMetadata>, MxpmError> {
    let userdir = crate::paths::maxima_userdir(config)?;
    let mut packages = Vec::new();

    if !userdir.exists() {
        return Ok(packages);
    }

    let entries = std::fs::read_dir(&userdir).map_err(MxpmError::Io)?;
    for entry in entries {
        let entry = entry.map_err(MxpmError::Io)?;
        if entry.path().is_dir() {
            let metadata_path = entry.path().join(".mxpm.json");
            if metadata_path.exists()
                && let Ok(metadata) = read_install_metadata(&entry.path())
            {
                packages.push(metadata);
            }
        }
    }

    packages.sort_by_key(|p| p.name.clone());
    Ok(packages)
}

/// Remove an installed package.
///
/// For editable (symlinked) packages: removes the symlink and cleans up
/// `.mxpm.json` from the source directory. For regular packages: removes
/// the entire package directory.
pub fn remove_package(name: &str, config: &Config) -> Result<(), MxpmError> {
    let package_dir = crate::paths::package_dir(config, name)?;
    let metadata_path = package_dir.join(".mxpm.json");

    if !metadata_path.exists() {
        return Err(MxpmError::NotInstalled {
            name: name.to_string(),
        });
    }

    // Check if this is an editable install (symlink)
    let is_symlink = package_dir
        .symlink_metadata()
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false);

    if is_symlink {
        // Remove .mxpm.json from the source directory (the symlink target)
        let _ = std::fs::remove_file(&metadata_path);
        // Remove the symlink itself
        std::fs::remove_file(&package_dir).map_err(MxpmError::Io)?;
    } else {
        std::fs::remove_dir_all(&package_dir).map_err(MxpmError::Io)?;
    }

    Ok(())
}

/// Check whether a package is currently installed.
pub fn is_installed(name: &str, config: &Config) -> Result<bool, MxpmError> {
    let package_dir = crate::paths::package_dir(config, name)?;
    Ok(package_dir.join(".mxpm.json").exists())
}

/// Search the index for packages matching a query string.
/// Returns (package_name, entry, registry_name, score) sorted by relevance.
pub fn search_packages<'a>(
    query: &str,
    registries: &'a [crate::registry::Registry],
) -> Vec<(&'a str, &'a PackageEntry, &'a str, u32)> {
    let query_lower = query.to_lowercase();
    let mut results: Vec<(&str, &PackageEntry, &str, u32)> = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for registry in registries {
        for (name, entry) in &registry.index.packages {
            if seen.contains(name.as_str()) {
                continue;
            }

            let score = score_match(name, entry, &query_lower);
            if score > 0 {
                seen.insert(name.as_str());
                results.push((name, entry, &registry.name, score));
            }
        }
    }

    results.sort_by(|a, b| b.3.cmp(&a.3).then(a.0.cmp(b.0)));
    results
}

/// Visible for testing.
pub fn score_match(name: &str, entry: &PackageEntry, query: &str) -> u32 {
    let name_lower = name.to_lowercase();
    let desc_lower = entry.description.to_lowercase();

    let mut score = 0u32;

    // Exact name match
    if name_lower == *query {
        score += 100;
    } else if name_lower.contains(query) {
        score += 50;
    }

    // Keyword match
    if let Some(keywords) = &entry.keywords {
        for kw in keywords {
            if kw.to_lowercase() == *query {
                score += 30;
            } else if kw.to_lowercase().contains(query) {
                score += 15;
            }
        }
    }

    // Description match
    if desc_lower.contains(query) {
        score += 10;
    }

    score
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::PackageEntry;

    fn make_entry(desc: &str, keywords: Option<Vec<&str>>) -> PackageEntry {
        PackageEntry {
            description: desc.to_string(),
            repository: "https://example.com".to_string(),
            source: Source::Git {
                url: "https://example.com/repo.git".into(),
                git_ref: "abc123".into(),
                subdir: None,
            },
            homepage: None,
            keywords: keywords.map(|kws| kws.into_iter().map(String::from).collect()),
            license: None,
            authors: None,
        }
    }

    #[test]
    fn exact_name_match_scores_highest() {
        let entry = make_entry("Some package", None);
        let score = score_match("diophantine", &entry, "diophantine");
        assert_eq!(score, 100);
    }

    #[test]
    fn partial_name_match() {
        let entry = make_entry("Some package", None);
        let score = score_match("diophantine", &entry, "diophan");
        assert_eq!(score, 50);
    }

    #[test]
    fn keyword_exact_match() {
        let entry = make_entry("Some package", Some(vec!["algebra", "math"]));
        let score = score_match("other-name", &entry, "algebra");
        assert_eq!(score, 30);
    }

    #[test]
    fn description_match() {
        let entry = make_entry("Solver for equations", None);
        let score = score_match("other-name", &entry, "equations");
        assert_eq!(score, 10);
    }

    #[test]
    fn no_match_scores_zero() {
        let entry = make_entry("Some package", Some(vec!["math"]));
        let score = score_match("other-name", &entry, "xyz");
        assert_eq!(score, 0);
    }

    #[test]
    fn case_insensitive_matching() {
        let entry = make_entry("Diophantine Solver", Some(vec!["Number-Theory"]));
        let score = score_match("Diophantine", &entry, "diophantine");
        assert!(score >= 100); // exact name + description
    }

    #[test]
    fn multiple_match_types_combine() {
        // Name contains query + description contains query
        let entry = make_entry("A test package", Some(vec!["test"]));
        let score = score_match("test-pkg", &entry, "test");
        // partial name (50) + exact keyword (30) + description (10)
        assert_eq!(score, 90);
    }

    #[test]
    fn resolve_source_git_uses_commit() {
        let original = Source::Git {
            url: "https://example.com/repo.git".into(),
            git_ref: "main".into(),
            subdir: Some("sub".into()),
        };
        let result = DownloadResult {
            url: "https://example.com/repo.git".into(),
            commit: Some("abc123def456".into()),
            hash: None,
            hash_algorithm: None,
        };
        let resolved = resolve_source(&original, &result);
        match resolved {
            Source::Git {
                git_ref, subdir, ..
            } => {
                assert_eq!(git_ref, "abc123def456");
                assert_eq!(subdir.unwrap(), "sub");
            }
            _ => panic!("expected git source"),
        }
    }

    #[test]
    fn resolve_source_tarball_adds_hash() {
        let original = Source::Tarball {
            url: "https://example.com/pkg.tar.gz".into(),
            hash: None,
            hash_algorithm: None,
        };
        let result = DownloadResult {
            url: "https://example.com/pkg.tar.gz".into(),
            commit: None,
            hash: Some("deadbeef".into()),
            hash_algorithm: Some("sha256".into()),
        };
        let resolved = resolve_source(&original, &result);
        match resolved {
            Source::Tarball {
                hash,
                hash_algorithm,
                ..
            } => {
                assert_eq!(hash.unwrap(), "deadbeef");
                assert_eq!(hash_algorithm.unwrap(), "sha256");
            }
            _ => panic!("expected tarball source"),
        }
    }

    #[test]
    fn list_installed_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let config = Config {
            maxima_userdir: Some(dir.path().to_path_buf()),
            ..Config::default()
        };
        let packages = list_installed(&config).unwrap();
        assert!(packages.is_empty());
    }

    #[test]
    fn list_installed_finds_packages() {
        let dir = tempfile::tempdir().unwrap();
        let pkg_dir = dir.path().join("test-pkg");
        std::fs::create_dir(&pkg_dir).unwrap();
        let metadata = InstallMetadata {
            name: "test-pkg".into(),
            version: Some("1.0.0".into()),
            installed_at: "2025-01-01T00:00:00Z".into(),
            source: Source::Git {
                url: "https://example.com".into(),
                git_ref: "abc123".into(),
                subdir: None,
            },
            registry: "community".into(),
        };
        let json = serde_json::to_string(&metadata).unwrap();
        std::fs::write(pkg_dir.join(".mxpm.json"), json).unwrap();

        let config = Config {
            maxima_userdir: Some(dir.path().to_path_buf()),
            ..Config::default()
        };
        let packages = list_installed(&config).unwrap();
        assert_eq!(packages.len(), 1);
        assert_eq!(packages[0].name, "test-pkg");
    }

    #[test]
    fn is_installed_detects_package() {
        let dir = tempfile::tempdir().unwrap();
        let pkg_dir = dir.path().join("foo");
        std::fs::create_dir(&pkg_dir).unwrap();
        std::fs::write(pkg_dir.join(".mxpm.json"), "{}").unwrap();

        let config = Config {
            maxima_userdir: Some(dir.path().to_path_buf()),
            ..Config::default()
        };
        // .mxpm.json exists but is invalid JSON for InstallMetadata,
        // but is_installed only checks file existence
        assert!(is_installed("foo", &config).unwrap());
        assert!(!is_installed("bar", &config).unwrap());
    }

    #[test]
    fn remove_package_deletes_dir() {
        let dir = tempfile::tempdir().unwrap();
        let pkg_dir = dir.path().join("foo");
        std::fs::create_dir(&pkg_dir).unwrap();
        std::fs::write(pkg_dir.join(".mxpm.json"), "{}").unwrap();
        std::fs::write(pkg_dir.join("foo.mac"), "/* code */").unwrap();

        let config = Config {
            maxima_userdir: Some(dir.path().to_path_buf()),
            ..Config::default()
        };
        remove_package("foo", &config).unwrap();
        assert!(!pkg_dir.exists());
    }
}
