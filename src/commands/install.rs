use std::path::Path;

use crate::config::Config;
use crate::errors::MxpmError;
use crate::index::Source;
use crate::manifest;
use crate::output::{self, OutputFormat};
use crate::paths;
use crate::registry;

pub async fn run(
    package: Option<&str>,
    reinstall: bool,
    path: Option<&str>,
    editable: bool,
    format: OutputFormat,
    config: &Config,
) -> Result<(), MxpmError> {
    if let Some(source_path) = path {
        run_local(package, source_path, reinstall, editable, format, config)
    } else {
        let name = package.ok_or_else(|| {
            MxpmError::Io(std::io::Error::other(
                "package name is required for registry installs",
            ))
        })?;
        run_registry(name, reinstall, format, config).await
    }
}

fn run_local(
    package: Option<&str>,
    source_path: &str,
    reinstall: bool,
    editable: bool,
    format: OutputFormat,
    config: &Config,
) -> Result<(), MxpmError> {
    let source_dir = Path::new(source_path);

    // Read manifest to get package name
    let manifest_path = source_dir.join("manifest.toml");
    if !manifest_path.exists() {
        return Err(MxpmError::ManifestNotFound {
            path: source_dir.display().to_string(),
        });
    }
    let manifest_contents = std::fs::read_to_string(&manifest_path).map_err(MxpmError::Io)?;
    let m = manifest::parse_manifest(&manifest_contents)
        .map_err(|e| MxpmError::Io(std::io::Error::other(e)))?;

    let name = package.unwrap_or(&m.package.name);

    // Handle already-installed case
    if !reinstall && crate::install::is_installed(name, config)? {
        return Err(MxpmError::AlreadyInstalled {
            name: name.to_string(),
        });
    }
    if reinstall && crate::install::is_installed(name, config)? {
        crate::install::remove_package(name, config)?;
    }

    let mode = if editable { "editable" } else { "copy" };
    if format == OutputFormat::Human {
        eprintln!("Installing {name} from local path ({mode})...");
    }

    let metadata = crate::install::install_local_package(name, source_dir, editable, config)?;

    match format {
        OutputFormat::Json => output::print_json(&metadata)?,
        OutputFormat::Human => {
            if let Some(ref version) = metadata.version {
                eprintln!("Version: {version}");
            }
            eprintln!("Done.");
            println!("Use: load(\"{name}\");");
        }
    }

    Ok(())
}

async fn run_registry(
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
    if reinstall && crate::install::is_installed(name, config)? {
        crate::install::remove_package(name, config)?;
    }

    if format == OutputFormat::Human {
        eprintln!("Found {name} in registry '{registry_name}'");
    }

    let entry = entry.clone();
    let registry_name = registry_name.to_string();
    let metadata = crate::install::install_package(name, &entry, &registry_name, config).await?;

    match format {
        OutputFormat::Json => output::print_json(&metadata)?,
        OutputFormat::Human => {
            match &metadata.source {
                Source::Git { git_ref, .. } => {
                    let short = if git_ref.len() >= 12 {
                        &git_ref[..12]
                    } else {
                        git_ref
                    };
                    eprintln!("Commit:  {short}");
                }
                Source::Tarball {
                    hash,
                    hash_algorithm,
                    ..
                } => {
                    if let Some(h) = hash {
                        let algo = hash_algorithm.as_deref().unwrap_or("sha256");
                        eprintln!("Hash:    {algo}:{}", &h[..16.min(h.len())]);
                    }
                }
                Source::Local { .. } => {}
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
