use comfy_table::{ContentArrangement, Table};

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

            let mut table = Table::new();
            table.set_content_arrangement(ContentArrangement::Dynamic);
            table.set_header(vec!["NAME", "VERSION", "INSTALLED"]);

            for pkg in &packages {
                let version = pkg.version.as_deref().unwrap_or("-");
                let date = pkg
                    .installed_at
                    .split('T')
                    .next()
                    .unwrap_or(&pkg.installed_at);
                table.add_row(vec![&pkg.name, version, date]);
            }

            println!("{table}");
        }
    }

    Ok(())
}
