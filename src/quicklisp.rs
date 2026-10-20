//! Quicklisp/SBCL detection and dependency installation.

use std::path::PathBuf;
use std::process::Command;

use crate::errors::MxpmError;

/// Result of probing for SBCL and Quicklisp.
pub enum DetectResult {
    /// Both SBCL and Quicklisp are available.
    Ready(QuicklispSetup),
    /// SBCL found but Quicklisp is not installed.
    NoQuicklisp,
    /// SBCL not found in PATH.
    NoSbcl,
}

/// Detected SBCL + Quicklisp installation.
pub struct QuicklispSetup {
    pub sbcl_path: PathBuf,
    pub quicklisp_init: PathBuf,
}

impl QuicklispSetup {
    /// Detect SBCL and Quicklisp on the system.
    pub fn detect() -> DetectResult {
        let sbcl = match which("sbcl") {
            Some(p) => p,
            None => return DetectResult::NoSbcl,
        };
        let ql_init = dirs::home_dir()
            .map(|h| h.join("quicklisp/setup.lisp"))
            .filter(|p| p.exists());
        match ql_init {
            Some(init) => DetectResult::Ready(Self {
                sbcl_path: sbcl,
                quicklisp_init: init,
            }),
            None => DetectResult::NoQuicklisp,
        }
    }

    /// Run ql:quickload for the given system names.
    pub fn install_systems(
        &self,
        systems: &[String],
        dynamic_space_size: u32,
    ) -> Result<(), MxpmError> {
        let ql_args = systems
            .iter()
            .map(|s| format!(":{s}"))
            .collect::<Vec<_>>()
            .join(" ");
        let eval = format!("(ql:quickload (list {}) :silent t)", ql_args);
        let output = Command::new(&self.sbcl_path)
            .arg("--dynamic-space-size")
            .arg(dynamic_space_size.to_string())
            .arg("--non-interactive")
            .arg("--load")
            .arg(&self.quicklisp_init)
            .arg("--eval")
            .arg(&eval)
            .output()
            .map_err(MxpmError::Io)?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(MxpmError::QuicklispFailed {
                message: stderr.to_string(),
            });
        }
        Ok(())
    }
}

/// Find an executable on `PATH`, like the `which` command.
///
/// On Windows, executables carry an extension (`sbcl.exe`), so each `PATH`
/// directory is tried with every extension in `PATHEXT`.
pub fn which(name: &str) -> Option<PathBuf> {
    let paths = std::env::var_os("PATH")?;
    which_in(name, &paths, &executable_extensions())
}

/// [`which`] with an explicit search path and list of extensions to try.
pub fn which_in(name: &str, paths: &std::ffi::OsStr, extensions: &[String]) -> Option<PathBuf> {
    std::env::split_paths(paths).find_map(|dir| {
        extensions
            .iter()
            .map(|ext| dir.join(format!("{name}{ext}")))
            .find(|p| p.is_file())
    })
}

/// Suffixes that make a file name executable: none on Unix, `PATHEXT` on
/// Windows (`.COM;.EXE;.BAT;.CMD` if unset).
fn executable_extensions() -> Vec<String> {
    if cfg!(windows) {
        let pathext = std::env::var("PATHEXT").unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".into());
        pathext
            .split(';')
            .filter(|e| !e.is_empty())
            .map(|e| e.to_ascii_lowercase())
            .collect()
    } else {
        vec![String::new()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exts(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn finds_executable_with_extension() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("sbcl.exe"), "").unwrap();
        let found = which_in("sbcl", dir.path().as_os_str(), &exts(&[".com", ".exe"]));
        assert_eq!(found, Some(dir.path().join("sbcl.exe")));
    }

    #[test]
    fn finds_bare_name_when_no_extension_is_needed() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("sbcl"), "").unwrap();
        let found = which_in("sbcl", dir.path().as_os_str(), &exts(&[""]));
        assert_eq!(found, Some(dir.path().join("sbcl")));
    }

    #[test]
    fn searches_path_directories_in_order() {
        let first = tempfile::tempdir().unwrap();
        let second = tempfile::tempdir().unwrap();
        std::fs::write(first.path().join("sbcl.exe"), "").unwrap();
        std::fs::write(second.path().join("sbcl.exe"), "").unwrap();
        let paths = std::env::join_paths([second.path(), first.path()]).unwrap();
        let found = which_in("sbcl", &paths, &exts(&[".exe"]));
        assert_eq!(found, Some(second.path().join("sbcl.exe")));
    }

    #[test]
    fn ignores_directories_and_missing_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("sbcl")).unwrap();
        assert_eq!(which_in("sbcl", dir.path().as_os_str(), &exts(&[""])), None);
        assert_eq!(
            which_in("sbcl", dir.path().as_os_str(), &exts(&[".exe"])),
            None
        );
    }
}
