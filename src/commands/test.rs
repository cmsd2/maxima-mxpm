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
    #[serde(skip_serializing_if = "Option::is_none")]
    failure_details: Option<String>,
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

/// Extract the failure portions of Maxima test output.
///
/// `batch(file, test)` prints a block per failing test:
///
/// ```text
/// ********************** Problem 3 ***************
/// Input:
/// <expr>
///
/// Result:
/// <actual>
///
/// This differed from the expected result:
/// <expected>
/// ```
///
/// When the test wrapper uses mxpm_run_tests to batch multiple sub-files,
/// the output contains several of these regions, each terminated by a
/// summary line.  We collect every such region -- the file name appears
/// inside each block ("rtest_foo.mac: Problem N (line L)"), so failure
/// attribution is preserved.
fn extract_failure_details(output: &str) -> Option<String> {
    let lines: Vec<&str> = output.lines().collect();
    let mut blocks: Vec<String> = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        if lines[i].contains("***") && lines[i].contains("Problem") {
            // Capture from this Problem banner up to (but not past) the NEXT
            // one.  A block ends either at the next Problem banner, at a
            // summary line, or at EOF.
            let start = i;
            i += 1;
            while i < lines.len() {
                let trimmed = lines[i].trim();
                let is_summary = trimmed.ends_with("tests passed")
                    || trimmed.trim_end_matches('.').ends_with("correct");
                let is_next_problem = lines[i].contains("***") && lines[i].contains("Problem");
                if is_next_problem {
                    break;
                }
                if is_summary {
                    i += 1;
                    break;
                }
                i += 1;
            }
            let block: String = lines[start..i.min(lines.len())].join("\n");
            // Keep only blocks that are actual failures.  Passing tests also
            // emit a Problem banner but end with "... Which was correct."
            // Failures include "This differed from the expected result:".
            if block.contains("This differed from the expected result") {
                blocks.push(block.trim_end().to_string());
            }
        } else {
            i += 1;
        }
    }

    if blocks.is_empty() {
        None
    } else {
        Some(blocks.join("\n\n"))
    }
}

/// Parse Maxima batch test output and sum all summary lines.
///
/// A single batch(f, test) call emits one summary line.  When a test wrapper
/// uses mxpm_run_tests to run multiple sub-files, the output contains one
/// summary per sub-file; we sum them so the reported total matches what
/// actually ran.
///
/// Modern Maxima (5.47+): `M/N tests passed`
/// Older Maxima:          `N problems attempted; M correct.`
///
/// Returns (total_attempted, total_correct) across all summary lines, or
/// None if no summary was found at all.
fn parse_test_output(output: &str) -> Option<(u32, u32)> {
    let mut total_attempted: u32 = 0;
    let mut total_correct: u32 = 0;
    let mut saw_any = false;

    for line in output.lines() {
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
                total_attempted += total;
                total_correct += passed;
                saw_any = true;
                continue;
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
                    total_attempted += attempted;
                    total_correct += correct;
                    saw_any = true;
                }
            }
        }
    }

    if saw_any {
        Some((total_attempted, total_correct))
    } else {
        None
    }
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
        let failure_details = if success {
            None
        } else {
            extract_failure_details(&combined)
        };

        if matches!(format, OutputFormat::Human) {
            if success {
                eprintln!("  {test_file}: {correct}/{attempted} passed");
            } else {
                eprintln!("  {test_file}: {correct}/{attempted} passed ({failed} FAILED)");
                if let Some(details) = &failure_details {
                    for line in details.lines() {
                        eprintln!("    {line}");
                    }
                }
            }
        }

        Ok(TestFileResult {
            package: package.to_string(),
            file: test_file.to_string(),
            attempted,
            passed: correct,
            failed,
            success,
            failure_details,
        })
    } else {
        // No test summary — either Maxima crashed or the test file failed to
        // produce one. Include the combined output so the user can diagnose.
        let tail = tail_lines(&combined, 40);
        let prefix = if !output.status.success() {
            format!("maxima exited with {}", output.status)
        } else {
            format!("no test summary found in output for {test_file}")
        };
        let message = if tail.is_empty() {
            prefix
        } else {
            format!("{prefix}\n---\n{tail}")
        };
        Err(MxpmError::TestFailed {
            package: package.to_string(),
            message,
        })
    }
}

/// Return the last `n` non-empty-suffix lines of `s` joined by newlines.
fn tail_lines(s: &str, n: usize) -> String {
    let trimmed = s.trim_end();
    if trimmed.is_empty() {
        return String::new();
    }
    let lines: Vec<&str> = trimmed.lines().collect();
    let start = lines.len().saturating_sub(n);
    lines[start..].join("\n")
}

/// Detect if the current directory is an editable (symlinked) package.
///
/// Scans installed packages for symlinks that point to the current directory.
fn detect_package_from_cwd(config: &Config) -> Option<String> {
    let cwd = std::env::current_dir().ok()?;
    let cwd = cwd.canonicalize().ok()?;
    let userdir = paths::maxima_userdir(config).ok()?;
    let entries = std::fs::read_dir(&userdir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_symlink()
            && let Ok(target) = std::fs::canonicalize(&path)
            && target == cwd
        {
            return Some(entry.file_name().to_string_lossy().into_owned());
        }
    }
    // Also check if we're in a directory with a manifest.toml (local package dir)
    let manifest_path = cwd.join("manifest.toml");
    if manifest_path.exists()
        && let Ok(contents) = std::fs::read_to_string(&manifest_path)
        && let Ok(m) = manifest::parse_manifest(&contents)
    {
        return Some(m.package.name);
    }
    None
}

/// Run tests for a single package or all installed packages.
///
/// Returns `Ok(true)` if all tests pass, `Ok(false)` if any fail.
pub fn run(
    package: Option<&str>,
    all: bool,
    format: OutputFormat,
    config: &Config,
) -> Result<bool, MxpmError> {
    let maxima_bin = paths::maxima_bin(config);

    let packages: Vec<String> = if let Some(name) = package {
        let pkg_dir = paths::package_dir(config, name)?;
        if !pkg_dir.exists() {
            return Err(MxpmError::NotInstalled {
                name: name.to_string(),
            });
        }
        vec![name.to_string()]
    } else if all {
        let installed = install::list_installed(config)?;
        if installed.is_empty() {
            if matches!(format, OutputFormat::Human) {
                eprintln!("No packages installed.");
            }
            return Ok(true);
        }
        installed.into_iter().map(|m| m.name).collect()
    } else if let Some(name) = detect_package_from_cwd(config) {
        if matches!(format, OutputFormat::Human) {
            eprintln!("Detected package: {name}");
        }
        vec![name]
    } else {
        return Err(MxpmError::Io(std::io::Error::other(
            "no package specified. Use: mxpm test <package>, or run from a package directory, or use --all",
        )));
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
                        failure_details: Some(e.to_string()),
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

    /// When a test wrapper uses mxpm_run_tests to batch multiple sub-files,
    /// Maxima emits one "M/N tests passed" summary per sub-file.  We must
    /// sum them -- picking only the last (the pre-harness behaviour) would
    /// drop everyone but the final file's count.
    #[test]
    fn parse_multi_summary_sums_across_batches() {
        let output = "
(%i1) load(\"numerics\")
(%i2) batch(\"rtest_numerics.mac\",test)
Testing tests/test_a.mac:
3/3 tests passed
Testing tests/test_b.mac:
2/5 tests passed
Testing tests/test_c.mac:
4/4 tests passed
(%o2) rtest_numerics.mac
";
        let (attempted, correct) = parse_test_output(output).unwrap();
        assert_eq!(attempted, 12);
        assert_eq!(correct, 9);
    }

    #[test]
    fn parse_legacy_format() {
        let output = " 5 problems attempted; 3 correct.\n";
        let (attempted, correct) = parse_test_output(output).unwrap();
        assert_eq!(attempted, 5);
        assert_eq!(correct, 3);
    }

    #[test]
    fn extract_failure_details_modern() {
        let output = "\
(%i1) load(\"foo\")
(%o1) foo.mac
(%i2) batch(\"rtest_foo.mac\",test)
Testing rtest_foo.mac:
********************** Problem 3 ***************
Input:
2 + 2

Result:
5

This differed from the expected result:
4
2/3 tests passed
(%o2) rtest_foo.mac
";
        let details = extract_failure_details(output).unwrap();
        assert!(details.starts_with("********************** Problem 3"));
        assert!(details.contains("2 + 2"));
        assert!(details.contains("This differed from the expected result"));
        assert!(details.trim_end().ends_with("2/3 tests passed"));
    }

    /// batch(f, test) emits a "Problem N" banner for EVERY test, passing
    /// or failing.  Only blocks containing "This differed from the expected
    /// result" are real failures; passing blocks end with "... Which was
    /// correct." and must be skipped in the report.
    #[test]
    fn extract_failure_details_skips_passing_banners() {
        let output = "\
********************** Problem 1 (line 1) **********************

Input:
1 + 1

Result:
2

... Which was correct.

********************** Problem 2 (line 2) **********************

Input:
1 + 1

Result:
2

This differed from the expected result:
3

1/2 tests passed
";
        let details = extract_failure_details(output).unwrap();
        assert!(details.contains("Problem 2"));
        assert!(!details.contains("Problem 1"));
        assert!(details.contains("This differed"));
    }

    /// With mxpm_run_tests batching multiple sub-files, failure blocks
    /// appear across several summary sections.  extract_failure_details
    /// must collect every block so failures in later files are not lost.
    #[test]
    fn extract_failure_details_across_multiple_summaries() {
        let output = "\
(%i2) batch(\"rtest_foo.mac\",test)
Testing tests/test_a.mac:
********************** test_a.mac: Problem 2 ***************
Input:
2 + 2

Result:
5

This differed from the expected result:
4
3/4 tests passed
Testing tests/test_b.mac:
********************** test_b.mac: Problem 1 ***************
Input:
foo(1)

Result:
wrong

This differed from the expected result:
right
1/2 tests passed
(%o2) rtest_foo.mac
";
        let details = extract_failure_details(output).unwrap();
        // Both failure blocks present, with their file-prefixed headers intact.
        assert!(details.contains("test_a.mac: Problem 2"));
        assert!(details.contains("test_b.mac: Problem 1"));
        assert!(details.contains("2 + 2"));
        assert!(details.contains("foo(1)"));
    }

    #[test]
    fn extract_failure_details_no_failure() {
        let output = "\
(%i2) batch(\"rtest_foo.mac\",test)
2/2 tests passed
(%o2) rtest_foo.mac
";
        assert!(extract_failure_details(output).is_none());
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

    /// The test harness shipped by `mxpm new` lives at `tests/test_harness.mac`.
    /// The fallback glob must not pick it up as a test file: it does not start
    /// with `rtest_`, and it lives in a subdirectory.  If this test ever
    /// starts failing, the template or the discovery rule has drifted.
    #[test]
    fn discover_fallback_excludes_test_harness() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::create_dir(tmp.path().join("tests")).unwrap();
        std::fs::write(tmp.path().join("rtest_foo.mac"), "/* test */").unwrap();
        std::fs::write(
            tmp.path().join("tests").join("test_harness.mac"),
            "/* harness */",
        )
        .unwrap();
        // A hypothetical future mistake: someone drops the harness at pkg root.
        // Still excluded because it does not start with `rtest_`.
        std::fs::write(tmp.path().join("test_harness.mac"), "/* harness */").unwrap();

        let files = discover_test_files(tmp.path());
        assert_eq!(files, vec!["rtest_foo.mac"]);
        assert!(!files.iter().any(|f| f.contains("test_harness")));
    }
}
