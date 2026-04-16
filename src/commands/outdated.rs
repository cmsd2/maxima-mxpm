use serde::Serialize;

use crate::config::Config;
use crate::errors::MxpmError;
use crate::index::Source;
use crate::install;
use crate::output::{self, OutputFormat};
use crate::paths;
use crate::registry;

#[derive(Debug, Serialize)]
pub struct OutdatedPackage {
    pub name: String,
    pub installed_ref: Option<String>,
    pub registry_ref: Option<String>,
    pub registry: String,
}

/// Find packages whose installed source differs from the registry.
pub async fn find_outdated(config: &Config) -> Result<Vec<OutdatedPackage>, MxpmError> {
    let registries_config = config.effective_registries();
    let cache_dir = paths::cache_dir();
    let ttl = config.cache_ttl_duration();

    let registries = registry::load_registries(&registries_config, &cache_dir, ttl).await?;
    let installed = install::list_installed(config)?;

    let mut outdated = Vec::new();

    for pkg in &installed {
        if let Ok((entry, reg_name)) = registry::resolve_package(&pkg.name, &registries)
            && source_changed(&pkg.source, &entry.source)
        {
            outdated.push(OutdatedPackage {
                name: pkg.name.clone(),
                installed_ref: source_ref(&pkg.source),
                registry_ref: source_ref(&entry.source),
                registry: reg_name.to_string(),
            });
        }
    }

    outdated.sort_by_key(|o| o.name.clone());
    Ok(outdated)
}

pub async fn run(format: OutputFormat, config: &Config) -> Result<(), MxpmError> {
    let outdated = find_outdated(config).await?;

    match format {
        OutputFormat::Json => output::print_json(&outdated)?,
        OutputFormat::Human => {
            if outdated.is_empty() {
                println!("All packages are up to date.");
                return Ok(());
            }

            let name_w = outdated.iter().map(|p| p.name.len()).max().unwrap_or(0);

            for pkg in &outdated {
                let installed = short_ref(pkg.installed_ref.as_deref());
                let available = short_ref(pkg.registry_ref.as_deref());
                println!("{:<name_w$}  {} -> {}", pkg.name, installed, available);
            }
        }
    }

    Ok(())
}

/// Check if the registry source indicates a newer version than what's installed.
/// Compares only the identifying fields (ref for git, url for tarball),
/// ignoring computed metadata like hash that may differ.
fn source_changed(installed: &Source, registry: &Source) -> bool {
    match (installed, registry) {
        (
            Source::Git {
                git_ref: installed_ref,
                ..
            },
            Source::Git {
                git_ref: registry_ref,
                ..
            },
        ) => installed_ref != registry_ref,
        (
            Source::Tarball {
                url: installed_url, ..
            },
            Source::Tarball {
                url: registry_url, ..
            },
        ) => installed_url != registry_url,
        // Source type changed entirely
        _ => true,
    }
}

fn source_ref(source: &Source) -> Option<String> {
    match source {
        Source::Git { git_ref, .. } => Some(git_ref.clone()),
        Source::Tarball { url, .. } => Some(url.clone()),
        Source::Local { path, .. } => Some(path.clone()),
    }
}

fn short_ref(r: Option<&str>) -> String {
    match r {
        Some(s) if s.len() == 40 && s.chars().all(|c| c.is_ascii_hexdigit()) => s[..12].to_string(),
        Some(s) => s.to_string(),
        None => "-".to_string(),
    }
}
