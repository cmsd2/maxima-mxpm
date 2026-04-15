use crate::config::Config;
use crate::errors::MxpmError;
use crate::index::Source;
use crate::output::{self, OutputFormat};
use crate::paths;
use crate::registry;

pub async fn run(
    name: &str,
    reinstall: bool,
    format: OutputFormat,
    config: &Config,
) -> Result<(), MxpmError> {
    let registries_config = config.effective_registries();
    let cache_dir = paths::cache_dir();
    let ttl = config.cache_ttl_duration();

    let registries = registry::load_registries(&registries_config, &cache_dir, ttl).await?;
    let (entry, registry_name) = registry::resolve_package(name, &registries)?;

    // Handle already-installed case
    if !reinstall && crate::install::is_installed(name, config)? {
        return Err(MxpmError::AlreadyInstalled {
            name: name.to_string(),
        });
    }

    // If reinstalling, remove existing first
    if reinstall {
        if crate::install::is_installed(name, config)? {
            crate::install::remove_package(name, config)?;
        }
    }

    if format == OutputFormat::Human {
        eprintln!("Found {name} in registry '{registry_name}'");
    }

    let entry = entry.clone();
    let registry_name = registry_name.to_string();
    let metadata =
        crate::install::install_package(name, &entry, &registry_name, config).await?;

    match format {
        OutputFormat::Json => output::print_json(&metadata)?,
        OutputFormat::Human => {
            match &metadata.source {
                Source::Git { git_ref, .. } => {
                    let short = if git_ref.len() >= 12 { &git_ref[..12] } else { git_ref };
                    eprintln!("Commit:  {short}");
                }
                Source::Tarball { hash, hash_algorithm, .. } => {
                    if let Some(h) = hash {
                        let algo = hash_algorithm.as_deref().unwrap_or("sha256");
                        eprintln!("Hash:    {algo}:{}", &h[..16.min(h.len())]);
                    }
                }
            }
            if let Some(ref version) = metadata.version {
                eprintln!("Version: {version}");
            }
            eprintln!("Done.");
            println!("Use: load(\"{name}\");");
        }
    }

    Ok(())
}
