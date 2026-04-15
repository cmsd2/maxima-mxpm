use std::path::PathBuf;

use serde::Deserialize;

/// Default URL for the community package index.
pub const DEFAULT_REGISTRY_URL: &str =
    "https://raw.githubusercontent.com/cmsd2/maxima-package-index/master/index.json";

#[derive(Debug, Deserialize, Default, Clone)]
pub struct Config {
    /// Override Maxima user directory.
    pub maxima_userdir: Option<PathBuf>,

    /// Override Maxima binary path.
    pub maxima_bin: Option<PathBuf>,

    /// Cache TTL in seconds (default: 3600).
    pub cache_ttl: Option<u64>,

    /// Package registries, searched in order.
    pub registries: Option<Vec<RegistryConfig>>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RegistryConfig {
    pub name: String,
    pub url: String,
}

impl Config {
    /// Load configuration from the platform-appropriate config file,
    /// with environment variable overrides.
    pub fn load() -> Self {
        let mut config = Self::load_from_file();
        config.apply_env_overrides();
        config
    }

    fn load_from_file() -> Self {
        let config_path = crate::paths::config_dir().join("config.toml");
        if config_path.exists() {
            match std::fs::read_to_string(&config_path) {
                Ok(contents) => match toml::from_str(&contents) {
                    Ok(config) => return config,
                    Err(e) => {
                        eprintln!("warning: failed to parse {}: {e}", config_path.display());
                    }
                },
                Err(e) => {
                    eprintln!("warning: failed to read {}: {e}", config_path.display());
                }
            }
        }
        Self::default()
    }

    fn apply_env_overrides(&mut self) {
        if let Ok(dir) = std::env::var("MAXIMA_USERDIR") {
            self.maxima_userdir = Some(PathBuf::from(dir));
        }
        if let Ok(bin) = std::env::var("MAXIMA_BIN") {
            self.maxima_bin = Some(PathBuf::from(bin));
        }
        if let Ok(url) = std::env::var("MXPM_REGISTRY_URL") {
            self.registries = Some(vec![RegistryConfig {
                name: "override".to_string(),
                url,
            }]);
        }
    }

    /// Return the list of registries to search, with the default community
    /// registry appended if not explicitly listed.
    pub fn effective_registries(&self) -> Vec<RegistryConfig> {
        let mut registries = self.registries.clone().unwrap_or_default();
        let has_community = registries.iter().any(|r| r.name == "community");
        if !has_community {
            registries.push(RegistryConfig {
                name: "community".to_string(),
                url: DEFAULT_REGISTRY_URL.to_string(),
            });
        }
        registries
    }

    /// Cache TTL as a Duration (default: 1 hour).
    pub fn cache_ttl_duration(&self) -> std::time::Duration {
        std::time::Duration::from_secs(self.cache_ttl.unwrap_or(3600))
    }
}
