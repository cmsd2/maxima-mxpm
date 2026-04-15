use assert_cmd::Command;
use predicates::prelude::*;

fn mxpm() -> Command {
    Command::cargo_bin("mxpm").unwrap()
}

#[test]
fn help_flag() {
    mxpm()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Maxima Package Manager"))
        .stdout(predicate::str::contains("install"))
        .stdout(predicate::str::contains("search"))
        .stdout(predicate::str::contains("list"));
}

#[test]
fn version_flag() {
    mxpm()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::starts_with("mxpm "));
}

#[test]
fn install_help() {
    mxpm()
        .args(["install", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Install a package"));
}

#[test]
fn search_help() {
    mxpm()
        .args(["search", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Search for packages"));
}

#[test]
fn list_empty_with_temp_userdir() {
    let dir = tempfile::tempdir().unwrap();
    mxpm()
        .args(["list"])
        .env("MAXIMA_USERDIR", dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("No packages installed"));
}

#[test]
fn list_json_empty() {
    let dir = tempfile::tempdir().unwrap();
    mxpm()
        .args(["--json", "list"])
        .env("MAXIMA_USERDIR", dir.path())
        .assert()
        .success()
        .stdout(predicate::str::starts_with("["));
}

#[test]
fn remove_not_installed() {
    let dir = tempfile::tempdir().unwrap();
    mxpm()
        .args(["remove", "nonexistent", "--yes"])
        .env("MAXIMA_USERDIR", dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("not installed"));
}

#[test]
fn unknown_subcommand() {
    mxpm().arg("frobnicate").assert().failure();
}

#[test]
fn install_missing_package_arg() {
    mxpm().arg("install").assert().failure();
}
