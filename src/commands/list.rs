use crate::config::Config;
use crate::errors::MxpmError;
use crate::install;
use crate::output::{self, OutputFormat};

pub fn run(format: OutputFormat, config: &Config) -> Result<(), MxpmError> {
    let packages = install::list_installed(config)?;

    match format {
        OutputFormat::Json => output::print_json(&packages)?,
        OutputFormat::Human => {
            if packages.is_empty() {
                println!("No packages installed.");
                return Ok(());
            }

            let name_w = packages.iter().map(|p| p.name.len()).max().unwrap_or(0);
            let ver_w = packages
                .iter()
                .map(|p| p.version.as_deref().unwrap_or("-").len())
                .max()
                .unwrap_or(0);

            for pkg in &packages {
                let version = pkg.version.as_deref().unwrap_or("-");
                let date = pkg
                    .installed_at
                    .split('T')
                    .next()
                    .unwrap_or(&pkg.installed_at);
                println!("{:<name_w$}  {:<ver_w$}  {date}", pkg.name, version,);
            }
        }
    }

    Ok(())
}
