//! Maxima directory resolution.

use std::path::PathBuf;

/// Resolve the Maxima user directory.
///
/// Resolution order:
/// 1. `$MAXIMA_USERDIR` environment variable
/// 2. `~/.maxima/` (Unix) or `%USERPROFILE%/maxima/` (Windows)
pub fn maxima_userdir() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("MAXIMA_USERDIR") {
        return Some(PathBuf::from(dir));
    }
    dirs::home_dir().map(|h| {
        #[cfg(windows)]
        {
            h.join("maxima")
        }
        #[cfg(not(windows))]
        {
            h.join(".maxima")
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maxima_userdir_from_env() {
        unsafe {
            std::env::set_var("MAXIMA_USERDIR", "/tmp/test-maxima-core");
        }
        let dir = maxima_userdir().unwrap();
        assert_eq!(dir, PathBuf::from("/tmp/test-maxima-core"));
        unsafe {
            std::env::remove_var("MAXIMA_USERDIR");
        }
    }
}
