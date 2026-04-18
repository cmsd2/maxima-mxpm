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
    yes: bool,
    format: OutputFormat,
    config: &Config,
) -> Result<(), MxpmError> {
    if let Some(source_path) = path {
        run_local(
            package,
            source_path,
            reinstall,
            editable,
            yes,
            format,
            config,
        )
    } else {
        let name = package.ok_or_else(|| {
            MxpmError::Io(std::io::Error::other(
                "package name is required for registry installs",
            ))
        })?;
        run_registry(name, reinstall, yes, format, config).await
    }
}

fn run_local(
    package: Option<&str>,
    source_path: &str,
    reinstall: bool,
    editable: bool,
    yes: bool,
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
    if crate::install::is_installed(name, config)? {
        if !reinstall && !confirm_reinstall(name, yes, format)? {
            return Ok(());
        }
        crate::install::remove_package(name, config)?;
    }

    let mode = if editable { "editable" } else { "copy" };
    if format == OutputFormat::Human {
        eprintln!("Installing {name} from local path ({mode})...");
    }

    let metadata = crate::install::install_local_package(name, source_dir, editable, config)?;

    if format == OutputFormat::Human {
        maybe_install_quicklisp_deps(name, yes, config)?;
    }

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
    yes: bool,
    format: OutputFormat,
    config: &Config,
) -> Result<(), MxpmError> {
    let registries_config = config.effective_registries();
    let cache_dir = paths::cache_dir();
    let ttl = config.cache_ttl_duration();

    let registries = registry::load_registries(&registries_config, &cache_dir, ttl).await?;
    let (entry, registry_name) = registry::resolve_package(name, &registries)?;

    // Handle already-installed case
    if crate::install::is_installed(name, config)? {
        if !reinstall && !confirm_reinstall(name, yes, format)? {
            return Ok(());
        }
        crate::install::remove_package(name, config)?;
    }

    if format == OutputFormat::Human {
        eprintln!("Found {name} in registry '{registry_name}'");
    }

    let entry = entry.clone();
    let registry_name = registry_name.to_string();
    let metadata = crate::install::install_package(name, &entry, &registry_name, config).await?;

    if format == OutputFormat::Human {
        maybe_install_quicklisp_deps(name, yes, config)?;
    }

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

fn maybe_install_quicklisp_deps(name: &str, yes: bool, config: &Config) -> Result<(), MxpmError> {
    let userdir = paths::maxima_userdir(config)?;
    let manifest = match manifest::load_manifest(&userdir.join(name)) {
        Some(m) => m,
        None => return Ok(()),
    };
    let systems = match manifest.lisp.and_then(|l| l.quicklisp_systems) {
        Some(s) if !s.is_empty() => s,
        _ => return Ok(()),
    };

    let systems_str = systems.join(", ");

    use crate::quicklisp::{DetectResult, QuicklispSetup};

    match QuicklispSetup::detect() {
        DetectResult::Ready(ql) => {
            eprintln!();
            eprintln!("  CL dependencies needed: {systems_str}");

            let proceed = if yes {
                true
            } else {
                eprint!("  Install via Quicklisp now? [Y/n] ");
                let mut input = String::new();
                std::io::stdin().read_line(&mut input).unwrap_or(0);
                let trimmed = input.trim().to_lowercase();
                trimmed.is_empty() || trimmed == "y" || trimmed == "yes"
            };

            if proceed {
                eprintln!(
                    "  Installing CL dependencies (this may take a few minutes on first run)..."
                );
                ql.install_systems(&systems, config.sbcl_dynamic_space_size())?;
                eprintln!("  CL dependencies installed.");
            } else {
                print_quicklisp_preinstall_hint(&systems, name);
            }
        }
        DetectResult::NoQuicklisp => {
            eprintln!();
            eprintln!("  This package requires Quicklisp (SBCL).");
            eprintln!("  To set up Quicklisp:");
            eprintln!("    mxpm setup quicklisp");
            eprintln!();
            print_quicklisp_preinstall_hint(&systems, name);
        }
        DetectResult::NoSbcl => {
            eprintln!();
            eprintln!("  This package requires SBCL with Quicklisp.");
            eprintln!("  Install SBCL first:");
            eprintln!("    macOS:  brew install sbcl");
            eprintln!("    Linux:  apt install sbcl");
            eprintln!();
            eprintln!("  Then run: mxpm setup quicklisp");
        }
    }
    Ok(())
}

fn confirm_reinstall(name: &str, yes: bool, format: OutputFormat) -> Result<bool, MxpmError> {
    if format != OutputFormat::Human {
        return Err(MxpmError::AlreadyInstalled {
            name: name.to_string(),
        });
    }
    if yes {
        return Ok(true);
    }
    eprint!("Package '{name}' is already installed. Reinstall? [y/N] ");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).unwrap_or(0);
    let trimmed = input.trim().to_lowercase();
    Ok(trimmed == "y" || trimmed == "yes")
}

fn print_quicklisp_preinstall_hint(systems: &[String], name: &str) {
    let ql_args = systems
        .iter()
        .map(|s| format!(":{s}"))
        .collect::<Vec<_>>()
        .join(" ");
    eprintln!("  To pre-install CL dependencies:");
    eprintln!("    sbcl --eval '(ql:quickload (list {ql_args}))' --quit");
    eprintln!();
    eprintln!("  Or they will be installed automatically on first load(\"{name}\").");
}
