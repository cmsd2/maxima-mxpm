use crate::config::Config;
use crate::errors::MxpmError;
use crate::output::OutputFormat;
use crate::quicklisp::{self, DetectResult, QuicklispSetup};

pub fn quicklisp(yes: bool, format: OutputFormat, config: &Config) -> Result<(), MxpmError> {
    match QuicklispSetup::detect() {
        DetectResult::Ready(ql) => {
            if format == OutputFormat::Human {
                eprintln!(
                    "Quicklisp is already installed at {}",
                    ql.quicklisp_init.display()
                );
            }
            return Ok(());
        }
        DetectResult::NoSbcl => {
            return Err(MxpmError::Io(std::io::Error::other(
                "SBCL not found in PATH. Install SBCL first:\n  \
                 macOS:  brew install sbcl\n  \
                 Linux:  apt install sbcl  (or your distro's package manager)",
            )));
        }
        DetectResult::NoQuicklisp => {}
    }

    let sbcl = quicklisp::which("sbcl").expect("sbcl already confirmed present");

    if format == OutputFormat::Human {
        eprintln!("Setting up Quicklisp...");
    }

    // Download quicklisp.lisp to a temp dir
    let tmp_dir = tempfile::tempdir().map_err(MxpmError::Io)?;
    let ql_lisp = tmp_dir.path().join("quicklisp.lisp");

    let output = std::process::Command::new("curl")
        .arg("-sS")
        .arg("-o")
        .arg(&ql_lisp)
        .arg("https://beta.quicklisp.org/quicklisp.lisp")
        .output()
        .map_err(MxpmError::Io)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(MxpmError::Io(std::io::Error::other(format!(
            "failed to download quicklisp.lisp: {stderr}"
        ))));
    }

    if format == OutputFormat::Human {
        eprintln!("  Downloaded quicklisp.lisp");
    }

    if !yes {
        eprint!("  Install Quicklisp to ~/quicklisp/? [Y/n] ");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap_or(0);
        let trimmed = input.trim().to_lowercase();
        if !(trimmed.is_empty() || trimmed == "y" || trimmed == "yes") {
            eprintln!("  Aborted.");
            return Ok(());
        }
    }

    // Run SBCL to install Quicklisp.
    let output = std::process::Command::new(&sbcl)
        .arg("--dynamic-space-size")
        .arg(config.sbcl_dynamic_space_size().to_string())
        .arg("--non-interactive")
        .arg("--load")
        .arg(&ql_lisp)
        .arg("--eval")
        .arg("(quicklisp-quickstart:install)")
        .output()
        .map_err(MxpmError::Io)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(MxpmError::QuicklispFailed {
            message: stderr.to_string(),
        });
    }

    if format == OutputFormat::Human {
        eprintln!("  Quicklisp installed to ~/quicklisp/");
        eprintln!("Done.");
    }

    Ok(())
}
