use serde::Serialize;

use crate::config::Config;
use crate::errors::MxpmError;
use crate::install;
use crate::output::{self, OutputFormat};
use crate::paths;

#[derive(Serialize)]
struct RemoveResult {
    name: String,
    removed: bool,
}

pub fn run(name: &str, yes: bool, format: OutputFormat, config: &Config) -> Result<(), MxpmError> {
    let package_dir = paths::package_dir(config, name)?;

    if !install::is_installed(name, config)? {
        return Err(MxpmError::NotInstalled {
            name: name.to_string(),
        });
    }

    // In JSON mode, skip interactive confirmation (treat as --yes)
    if !yes && format == OutputFormat::Human {
        eprint!("Remove {name} from {}? [y/N] ", package_dir.display());
        let mut input = String::new();
        std::io::stdin()
            .read_line(&mut input)
            .map_err(MxpmError::Io)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            eprintln!("Cancelled.");
            return Ok(());
        }
    }

    install::remove_package(name, config)?;

    match format {
        OutputFormat::Json => output::print_json(&RemoveResult {
            name: name.to_string(),
            removed: true,
        })?,
        OutputFormat::Human => {
            eprintln!("Removed {name}.");
        }
    }

    Ok(())
}
