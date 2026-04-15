use serde::Serialize;

use crate::config::Config;
use crate::errors::MxpmError;
use crate::output::{self, OutputFormat};
use crate::paths;
use crate::registry;

#[derive(Serialize)]
struct IndexUpdateResult {
    registries: Vec<RegistrySummary>,
}

#[derive(Serialize)]
struct RegistrySummary {
    name: String,
    package_count: usize,
}

pub async fn update(format: OutputFormat, config: &Config) -> Result<(), MxpmError> {
    let registries_config = config.effective_registries();
    let cache_dir = paths::cache_dir();

    let registries = registry::force_refresh(&registries_config, &cache_dir).await?;

    match format {
        OutputFormat::Json => {
            let result = IndexUpdateResult {
                registries: registries
                    .iter()
                    .map(|r| RegistrySummary {
                        name: r.name.clone(),
                        package_count: r.index.packages.len(),
                    })
                    .collect(),
            };
            output::print_json(&result)?;
        }
        OutputFormat::Human => {
            for reg in &registries {
                let count = reg.index.packages.len();
                eprintln!("Registry '{}': {} packages", reg.name, count);
            }
            eprintln!("Index updated.");
        }
    }

    Ok(())
}
