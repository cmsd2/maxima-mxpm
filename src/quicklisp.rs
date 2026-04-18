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

pub fn which(name: &str) -> Option<PathBuf> {
    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths)
            .map(|dir| dir.join(name))
            .find(|p| p.exists())
    })
}
