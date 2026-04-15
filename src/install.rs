use std::path::Path;

use crate::config::Config;
use crate::errors::MxpmError;
use crate::index::PackageEntry;
use crate::manifest;
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
            let short_ref = if git_ref.len() == 40 { &git_ref[..12] } else { git_ref };
            eprintln!("Cloning {url} ({short_ref})...");
        }
        crate::index::Source::Tarball { url, .. } => {
            eprintln!("Downloading {url}...");
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

    // Write install metadata
    let metadata = InstallMetadata {
        name: name.to_string(),
        version,
        installed_at: chrono::Utc::now().to_rfc3339(),
        source: entry.source.clone(),
        registry: registry_name.to_string(),
        url: Some(download_result.url),
        commit: download_result.commit,
        hash: download_result.hash,
        hash_algorithm: download_result.hash_algorithm,
    };
    let metadata_json = serde_json::to_string_pretty(&metadata)
        .map_err(|e| MxpmError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;
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

/// Read install metadata from an installed package.
pub fn read_install_metadata(package_dir: &Path) -> Result<InstallMetadata, MxpmError> {
    let metadata_path = package_dir.join(".mxpm.json");
    let contents = std::fs::read_to_string(&metadata_path).map_err(MxpmError::Io)?;
    let metadata: InstallMetadata = serde_json::from_str(&contents)
        .map_err(|e| MxpmError::Io(std::io::Error::new(std::io::ErrorKind::InvalidData, e)))?;
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
            if metadata_path.exists() {
                if let Ok(metadata) = read_install_metadata(&entry.path()) {
                    packages.push(metadata);
                }
            }
        }
    }

    packages.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(packages)
}

/// Remove an installed package.
pub fn remove_package(name: &str, config: &Config) -> Result<(), MxpmError> {
    let package_dir = crate::paths::package_dir(config, name)?;
    let metadata_path = package_dir.join(".mxpm.json");

    if !metadata_path.exists() {
        return Err(MxpmError::NotInstalled {
            name: name.to_string(),
        });
    }

    std::fs::remove_dir_all(&package_dir).map_err(MxpmError::Io)?;
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

fn score_match(name: &str, entry: &PackageEntry, query: &str) -> u32 {
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
