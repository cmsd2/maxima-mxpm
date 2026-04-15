use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::time::SystemTime;

use crate::config::RegistryConfig;
use crate::errors::MxpmError;
use crate::index::{PackageEntry, PackageIndex};

/// A loaded registry with its cached index.
#[derive(Debug)]
pub struct Registry {
    pub name: String,
    pub url: String,
    pub index: PackageIndex,
}

/// Fetch and cache an index from a registry URL.
pub async fn fetch_index(
    url: &str,
    cache_dir: &Path,
    ttl: std::time::Duration,
) -> Result<PackageIndex, MxpmError> {
    std::fs::create_dir_all(cache_dir).map_err(MxpmError::Io)?;

    let cache_file = cache_dir.join(format!("index_{}.json", url_hash(url)));

    // Check cache freshness
    if let Ok(metadata) = std::fs::metadata(&cache_file)
        && let Ok(modified) = metadata.modified()
        && SystemTime::now()
            .duration_since(modified)
            .unwrap_or(ttl + std::time::Duration::from_secs(1))
            < ttl
        && let Ok(contents) = std::fs::read_to_string(&cache_file)
        && let Ok(index) = serde_json::from_str::<PackageIndex>(&contents)
    {
        return validate_index_version(index);
    }

    // Fetch fresh copy
    let body = reqwest::get(url)
        .await
        .map_err(|e| MxpmError::IndexFetch {
            url: url.to_string(),
            source: e,
        })?
        .text()
        .await
        .map_err(|e| MxpmError::IndexFetch {
            url: url.to_string(),
            source: e,
        })?;

    // Cache it
    let _ = std::fs::write(&cache_file, &body);

    let index: PackageIndex = serde_json::from_str(&body)?;
    validate_index_version(index)
}

/// Force-refresh all registry caches.
pub async fn force_refresh(
    registries: &[RegistryConfig],
    cache_dir: &Path,
) -> Result<Vec<Registry>, MxpmError> {
    let zero = std::time::Duration::from_secs(0);
    load_registries(registries, cache_dir, zero).await
}

/// Load all registries, using cached indexes where fresh enough.
pub async fn load_registries(
    registries: &[RegistryConfig],
    cache_dir: &Path,
    ttl: std::time::Duration,
) -> Result<Vec<Registry>, MxpmError> {
    let mut result = Vec::new();
    for r in registries {
        let index = fetch_index(&r.url, cache_dir, ttl).await?;
        result.push(Registry {
            name: r.name.clone(),
            url: r.url.clone(),
            index,
        });
    }
    Ok(result)
}

/// Resolve a package name across registries (first match wins).
pub fn resolve_package<'a>(
    name: &str,
    registries: &'a [Registry],
) -> Result<(&'a PackageEntry, &'a str), MxpmError> {
    for registry in registries {
        if let Some(entry) = registry.index.packages.get(name) {
            return Ok((entry, &registry.name));
        }
    }
    Err(MxpmError::PackageNotFound {
        name: name.to_string(),
    })
}

fn validate_index_version(index: PackageIndex) -> Result<PackageIndex, MxpmError> {
    if index.version != 1 {
        return Err(MxpmError::UnsupportedIndexVersion {
            version: index.version,
        });
    }
    Ok(index)
}

fn url_hash(url: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    url.hash(&mut hasher);
    hasher.finish()
}
