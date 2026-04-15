use comfy_table::{ContentArrangement, Table};
use serde::Serialize;

use crate::config::Config;
use crate::errors::MxpmError;
use crate::install;
use crate::output::{self, OutputFormat};
use crate::paths;
use crate::registry;

#[derive(Serialize)]
struct SearchResult {
    name: String,
    description: String,
    registry: String,
}

pub async fn run(query: &str, format: OutputFormat, config: &Config) -> Result<(), MxpmError> {
    let registries_config = config.effective_registries();
    let cache_dir = paths::cache_dir();
    let ttl = config.cache_ttl_duration();

    let registries = registry::load_registries(&registries_config, &cache_dir, ttl).await?;
    let results = install::search_packages(query, &registries);

    match format {
        OutputFormat::Json => {
            let json_results: Vec<SearchResult> = results
                .iter()
                .map(|(name, entry, registry, _score)| SearchResult {
                    name: name.to_string(),
                    description: entry.description.clone(),
                    registry: registry.to_string(),
                })
                .collect();
            output::print_json(&json_results)?;
        }
        OutputFormat::Human => {
            if results.is_empty() {
                println!("No packages found matching \"{query}\".");
                return Ok(());
            }

            let mut table = Table::new();
            table.set_content_arrangement(ContentArrangement::Dynamic);
            table.set_header(vec!["NAME", "DESCRIPTION"]);

            for (name, entry, _registry, _score) in &results {
                table.add_row(vec![*name, entry.description.as_str()]);
            }

            println!("{table}");
        }
    }

    Ok(())
}
