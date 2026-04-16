use std::path::Path;
use std::process::Command;

use serde::Serialize;

use crate::config::Config;
use crate::errors::MxpmError;
use crate::install;
use crate::manifest;
use crate::output::{self, OutputFormat};
use crate::paths;

#[derive(Serialize)]
struct TestFileResult {
    package: String,
    file: String,
    attempted: u32,
    passed: u32,
    failed: u32,
    success: bool,
}

#[derive(Serialize)]
struct TestSummary {
    results: Vec<TestFileResult>,
    total_attempted: u32,
    total_passed: u32,
    total_failed: u32,
    success: bool,
}

/// Discover test files for a package directory.
///
/// 1. If `manifest.toml` has a `[test]` section, use `test.files`
/// 2. Otherwise, glob for `rtest_*.mac` files
fn discover_test_files(pkg_dir: &Path) -> Vec<String> {
    // Try manifest first
    let manifest_path = pkg_dir.join("manifest.toml");
    if manifest_path.exists()
        && let Ok(contents) = std::fs::read_to_string(&manifest_path)
        && let Ok(m) = manifest::parse_manifest(&contents)
        && let Some(test_info) = m.test
    {
        return test_info.files;
    }

    // Fallback: scan for rtest_*.mac files
    let Ok(entries) = std::fs::read_dir(pkg_dir) else {
        return Vec::new();
    };
    let mut files: Vec<String> = entries
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            if name.starts_with("rtest_") && name.ends_with(".mac") {
                Some(name)
            } else {
                None
            }
        })
        .collect();
    files.sort();
    files
}

/// Parse Maxima batch test output for the summary line.
///
/// Maxima 5.47+: `M/N tests passed`
/// Older Maxima:  `N problems attempted; M correct.`
fn parse_test_output(output: &str) -> Option<(u32, u32)> {
    for line in output.lines().rev() {
        let trimmed = line.trim();

        // Modern format: "M/N tests passed"
        if let Some(rest) = trimmed.strip_suffix("tests passed") {
            let rest = rest.trim();
            if let Some((passed_str, total_str)) = rest.split_once('/')
                && let (Ok(passed), Ok(total)) = (
                    passed_str.trim().parse::<u32>(),
                    total_str.trim().parse::<u32>(),
                )
            {
                return Some((total, passed));
            }
        }

        // Legacy format: "N problems attempted; M correct."
        let trimmed_dot = trimmed.trim_end_matches('.');
        if let Some(rest) = trimmed_dot.strip_suffix("correct") {
            let rest = rest.trim().trim_end_matches(';').trim();
            if let Some(idx) = rest.find("problems attempted") {
                let attempted_str = rest[..idx].trim();
                let correct_str = rest[idx + "problems attempted".len()..].trim();
                let correct_str = correct_str.trim_start_matches(';').trim();
                if let (Ok(attempted), Ok(correct)) =
                    (attempted_str.parse::<u32>(), correct_str.parse::<u32>())
                {
                    return Some((attempted, correct));
                }
            }
        }
    }
    None
}

/// Run a single test file for a package.
fn run_test_file(
    maxima_bin: &Path,
    package: &str,
    test_file: &str,
    format: OutputFormat,
) -> Result<TestFileResult, MxpmError> {
    if matches!(format, OutputFormat::Human) {
        eprintln!("  {test_file}...");
    }

    let batch_string = format!("load(\"{package}\"); batch(\"{test_file}\", test);",);

    let result = Command::new(maxima_bin)
        .arg("--batch-string")
        .arg(&batch_string)
        .output();

    let output = match result {
        Ok(o) => o,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(MxpmError::MaximaNotFound);
        }
        Err(e) => {
            return Err(MxpmError::TestFailed {
                package: package.to_string(),
                message: format!("failed to execute maxima: {e}"),
            });
        }
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}\n{stderr}");

    if let Some((attempted, correct)) = parse_test_output(&combined) {
        let failed = attempted - correct;
        let success = failed == 0;

        if matches!(format, OutputFormat::Human) {
            if success {
                eprintln!("  {test_file}: {correct}/{attempted} passed");
            } else {
                eprintln!("  {test_file}: {correct}/{attempted} passed ({failed} FAILED)");
            }
        }

        Ok(TestFileResult {
            package: package.to_string(),
            file: test_file.to_string(),
            attempted,
            passed: correct,
            failed,
            success,
        })
    } else if !output.status.success() {
        let msg = if !stderr.is_empty() {
            stderr.trim().to_string()
        } else {
            format!("maxima exited with {}", output.status)
        };
        Err(MxpmError::TestFailed {
            package: package.to_string(),
            message: msg,
        })
    } else {
        // Maxima succeeded but no test summary found — treat as error
        Err(MxpmError::TestFailed {
            package: package.to_string(),
            message: format!("no test summary found in output for {test_file}"),
        })
    }
}

/// Run tests for a single package or all installed packages.
///
/// Returns `Ok(true)` if all tests pass, `Ok(false)` if any fail.
pub fn run(
    package: Option<&str>,
    format: OutputFormat,
    config: &Config,
) -> Result<bool, MxpmError> {
    let maxima_bin = paths::maxima_bin(config);

    let packages: Vec<String> = match package {
        Some(name) => {
            let pkg_dir = paths::package_dir(config, name)?;
            if !pkg_dir.exists() {
                return Err(MxpmError::NotInstalled {
                    name: name.to_string(),
                });
            }
            vec![name.to_string()]
        }
        None => {
            let installed = install::list_installed(config)?;
            if installed.is_empty() {
                if matches!(format, OutputFormat::Human) {
                    eprintln!("No packages installed.");
                }
                return Ok(true);
            }
            installed.into_iter().map(|m| m.name).collect()
        }
    };

    let mut all_results = Vec::new();

    for pkg_name in &packages {
        let pkg_dir = paths::package_dir(config, pkg_name)?;
        let test_files = discover_test_files(&pkg_dir);

        if test_files.is_empty() {
            if matches!(format, OutputFormat::Human) {
                eprintln!("{pkg_name}: no test files found");
            }
            continue;
        }

        if matches!(format, OutputFormat::Human) {
            eprintln!("Testing {pkg_name}...");
        }

        for test_file in &test_files {
            match run_test_file(&maxima_bin, pkg_name, test_file, format) {
                Ok(result) => all_results.push(result),
                Err(e) => {
                    if matches!(format, OutputFormat::Human) {
                        eprintln!("  {test_file}: ERROR — {e}");
                    }
                    all_results.push(TestFileResult {
                        package: pkg_name.clone(),
                        file: test_file.clone(),
                        attempted: 0,
                        passed: 0,
                        failed: 0,
                        success: false,
                    });
                }
            }
        }
    }

    let total_attempted: u32 = all_results.iter().map(|r| r.attempted).sum();
    let total_passed: u32 = all_results.iter().map(|r| r.passed).sum();
    let total_failed: u32 = all_results.iter().map(|r| r.failed).sum();
    let success = all_results.iter().all(|r| r.success);

    match format {
        OutputFormat::Json => {
            output::print_json(&TestSummary {
                results: all_results,
                total_attempted,
                total_passed,
                total_failed,
                success,
            })?;
        }
        OutputFormat::Human => {
            if total_attempted == 0 && all_results.is_empty() {
                // No tests at all — already reported per-package
            } else if success {
                eprintln!("All tests passed.");
            } else {
                eprintln!(
                    "Tests failed: {total_passed}/{total_attempted} passed, {total_failed} failed."
                );
            }
        }
    }

    Ok(success)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_modern_format() {
        let output = "
(%i1) load(\"hello-world\")
(%o1) hello-world.mac
(%i2) batch(\"rtest_hello-world.mac\",test)
2/2 tests passed
(%o2) rtest_hello-world.mac
";
        let (attempted, correct) = parse_test_output(output).unwrap();
        assert_eq!(attempted, 2);
        assert_eq!(correct, 2);
    }

    #[test]
    fn parse_modern_format_with_failures() {
        let output = "1/2 tests passed\n";
        let (attempted, correct) = parse_test_output(output).unwrap();
        assert_eq!(attempted, 2);
        assert_eq!(correct, 1);
    }

    #[test]
    fn parse_legacy_format() {
        let output = " 5 problems attempted; 3 correct.\n";
        let (attempted, correct) = parse_test_output(output).unwrap();
        assert_eq!(attempted, 5);
        assert_eq!(correct, 3);
    }

    #[test]
    fn parse_no_summary() {
        let output = "Some random maxima output\nno test summary here\n";
        assert!(parse_test_output(output).is_none());
    }

    #[test]
    fn discover_from_manifest() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join("manifest.toml"),
            r#"
[package]
name = "test-pkg"
version = "1.0.0"
description = "Test"
license = "MIT"
entry = "test-pkg.mac"

[test]
files = ["rtest_test-pkg.mac"]
"#,
        )
        .unwrap();

        let files = discover_test_files(tmp.path());
        assert_eq!(files, vec!["rtest_test-pkg.mac"]);
    }

    #[test]
    fn discover_fallback_glob() {
        let tmp = tempfile::TempDir::new().unwrap();
        // No manifest — create rtest files
        std::fs::write(tmp.path().join("rtest_foo.mac"), "/* test */").unwrap();
        std::fs::write(tmp.path().join("rtest_bar.mac"), "/* test */").unwrap();
        std::fs::write(tmp.path().join("other.mac"), "/* not a test */").unwrap();

        let mut files = discover_test_files(tmp.path());
        files.sort();
        assert_eq!(files, vec!["rtest_bar.mac", "rtest_foo.mac"]);
    }

    #[test]
    fn discover_no_tests() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(tmp.path().join("something.mac"), "/* not a test */").unwrap();

        let files = discover_test_files(tmp.path());
        assert!(files.is_empty());
    }
}
