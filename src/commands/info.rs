use serde::Serialize;

use crate::config::Config;
use crate::errors::MxpmError;
use crate::install;
use crate::output::{self, OutputFormat};
use crate::paths;
use crate::registry;

#[derive(Serialize)]
struct PackageInfo {
    name: String,
    description: String,
    authors: Option<Vec<String>>,
    license: Option<String>,
    repository: String,
    homepage: Option<String>,
    keywords: Option<Vec<String>>,
    registry: String,
    installed: Option<InstalledInfo>,
}

#[derive(Serialize)]
struct InstalledInfo {
    version: Option<String>,
    installed_at: String,
}

pub async fn run(
    name: &str,
    format: OutputFormat,
    config: &Config,
) -> Result<(), MxpmError> {
    let registries_config = config.effective_registries();
    let cache_dir = paths::cache_dir();
    let ttl = config.cache_ttl_duration();

    let registries = registry::load_registries(&registries_config, &cache_dir, ttl).await?;
    let (entry, registry_name) = registry::resolve_package(name, &registries)?;

    let installed = match install::is_installed(name, config) {
        Ok(true) => {
            let pkg_dir = paths::package_dir(config, name)?;
            install::read_install_metadata(&pkg_dir).ok().map(|m| InstalledInfo {
                version: m.version,
                installed_at: m.installed_at,
            })
        }
        _ => None,
    };

    match format {
        OutputFormat::Json => {
            let info = PackageInfo {
                name: name.to_string(),
                description: entry.description.clone(),
                authors: entry.authors.clone(),
                license: entry.license.clone(),
                repository: entry.repository.clone(),
                homepage: entry.homepage.clone(),
                keywords: entry.keywords.clone(),
                registry: registry_name.to_string(),
                installed,
            };
            output::print_json(&info)?;
        }
        OutputFormat::Human => {
            println!("Name:        {name}");
            println!("Description: {}", entry.description);
            if let Some(ref authors) = entry.authors {
                println!("Authors:     {}", authors.join(", "));
            }
            if let Some(ref license) = entry.license {
                println!("License:     {license}");
            }
            println!("Repository:  {}", entry.repository);
            if let Some(ref homepage) = entry.homepage {
                println!("Homepage:    {homepage}");
            }
            if let Some(ref keywords) = entry.keywords {
                println!("Keywords:    {}", keywords.join(", "));
            }
            println!("Registry:    {registry_name}");
            match installed {
                Some(info) => {
                    let version = info.version.as_deref().unwrap_or("-");
                    let date = info
                        .installed_at
                        .split('T')
                        .next()
                        .unwrap_or(&info.installed_at);
                    println!("Status:      installed (version {version}, {date})");
                }
                None => {
                    println!("Status:      not installed");
                }
            }
        }
    }

    Ok(())
}
