use serde::Serialize;

use crate::config::Config;
use crate::errors::MxpmError;
use crate::install;
use crate::output::{self, OutputFormat};
use crate::paths;
use crate::registry;

use super::outdated;

#[derive(Serialize)]
struct UpgradeResult {
    upgraded: Vec<UpgradedPackage>,
}

#[derive(Serialize)]
struct UpgradedPackage {
    name: String,
    old_ref: Option<String>,
    new_ref: Option<String>,
}

pub async fn run(
    package: Option<&str>,
    yes: bool,
    format: OutputFormat,
    config: &Config,
) -> Result<(), MxpmError> {
    let outdated_list = outdated::find_outdated(config).await?;

    let to_upgrade: Vec<&outdated::OutdatedPackage> = match package {
        Some(name) => {
            let found = outdated_list.iter().find(|p| p.name == name);
            match found {
                Some(p) => vec![p],
                None => {
                    if !install::is_installed(name, config)? {
                        return Err(MxpmError::NotInstalled {
                            name: name.to_string(),
                        });
                    }
                    if format == OutputFormat::Human {
                        println!("{name} is already up to date.");
                    }
                    return Ok(());
                }
            }
        }
        None => outdated_list.iter().collect(),
    };

    if to_upgrade.is_empty() {
        match format {
            OutputFormat::Json => output::print_json(&UpgradeResult {
                upgraded: Vec::new(),
            })?,
            OutputFormat::Human => println!("All packages are up to date."),
        }
        return Ok(());
    }

    // Confirm unless --yes or --json
    if !yes && format == OutputFormat::Human {
        let names: Vec<&str> = to_upgrade.iter().map(|p| p.name.as_str()).collect();
        eprint!("Upgrade {}? [y/N] ", names.join(", "));
        let mut input = String::new();
        std::io::stdin()
            .read_line(&mut input)
            .map_err(MxpmError::Io)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            eprintln!("Cancelled.");
            return Ok(());
        }
    }

    let registries_config = config.effective_registries();
    let cache_dir = paths::cache_dir();
    let ttl = config.cache_ttl_duration();
    let registries = registry::load_registries(&registries_config, &cache_dir, ttl).await?;

    let mut upgraded = Vec::new();

    for pkg in &to_upgrade {
        let (entry, registry_name) = registry::resolve_package(&pkg.name, &registries)?;

        if format == OutputFormat::Human {
            eprintln!("Upgrading {}...", pkg.name);
        }

        // Remove old version
        install::remove_package(&pkg.name, config)?;

        // Install new version
        let entry = entry.clone();
        let registry_name = registry_name.to_string();
        let metadata =
            install::install_package(&pkg.name, &entry, &registry_name, config).await?;

        upgraded.push(UpgradedPackage {
            name: pkg.name.clone(),
            old_ref: pkg.installed_ref.clone(),
            new_ref: pkg.registry_ref.clone(),
        });

        if format == OutputFormat::Human {
            let version_str = metadata.version.as_deref().unwrap_or("-");
            eprintln!("Upgraded {} to version {version_str}", pkg.name);
        }
    }

    match format {
        OutputFormat::Json => output::print_json(&UpgradeResult { upgraded })?,
        OutputFormat::Human => {
            eprintln!("Done.");
        }
    }

    Ok(())
}
