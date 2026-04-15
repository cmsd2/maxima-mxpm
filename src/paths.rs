use std::path::PathBuf;

use crate::config::Config;
use crate::errors::MxpmError;

/// Determine the Maxima user directory.
///
/// Resolution order:
/// 1. Config file `maxima_userdir` setting
/// 2. `$MAXIMA_USERDIR` environment variable
/// 3. `~/.maxima/` (Unix) or `%USERPROFILE%/maxima/` (Windows)
pub fn maxima_userdir(config: &Config) -> Result<PathBuf, MxpmError> {
    if let Some(ref dir) = config.maxima_userdir {
        return Ok(dir.clone());
    }

    if let Ok(dir) = std::env::var("MAXIMA_USERDIR") {
        return Ok(PathBuf::from(dir));
    }

    if let Some(home) = dirs::home_dir() {
        #[cfg(windows)]
        let dir = home.join("maxima");
        #[cfg(not(windows))]
        let dir = home.join(".maxima");
        return Ok(dir);
    }

    Err(MxpmError::MaximaUserDirNotFound)
}

/// Directory where installed package files live: `<maxima_userdir>/<name>/`
pub fn package_dir(config: &Config, name: &str) -> Result<PathBuf, MxpmError> {
    Ok(maxima_userdir(config)?.join(name))
}

/// Platform-appropriate cache directory for mxpm.
pub fn cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("mxpm")
}

/// Platform-appropriate config directory for mxpm.
pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("mxpm")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maxima_userdir_from_env() {
        let config = Config::default();
        unsafe {
            std::env::set_var("MAXIMA_USERDIR", "/tmp/test-maxima");
        }
        let dir = maxima_userdir(&config).unwrap();
        assert_eq!(dir, PathBuf::from("/tmp/test-maxima"));
        unsafe {
            std::env::remove_var("MAXIMA_USERDIR");
        }
    }

    #[test]
    fn maxima_userdir_from_config() {
        let config = Config {
            maxima_userdir: Some(PathBuf::from("/custom/maxima")),
            ..Config::default()
        };
        let dir = maxima_userdir(&config).unwrap();
        assert_eq!(dir, PathBuf::from("/custom/maxima"));
    }
}
