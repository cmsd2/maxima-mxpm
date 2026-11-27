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

fn write_manifest(dir: &std::path::Path, version: &str) {
    std::fs::write(
        dir.join("manifest.toml"),
        format!(
            "[package]\nname = \"demo\"\nversion = \"{version}\"\ndescription = \"A demo\"\nlicense = \"MIT\"\nentry = \"demo.mac\"\n"
        ),
    )
    .unwrap();
}

#[test]
fn list_editable_reads_current_manifest_version() {
    let userdir = tempfile::tempdir().unwrap();
    let src = tempfile::tempdir().unwrap();
    write_manifest(src.path(), "0.1.0");
    std::fs::write(src.path().join("demo.mac"), "/* demo */\n").unwrap();

    mxpm()
        .args(["install", "--path"])
        .arg(src.path())
        .args(["--editable", "--yes"])
        .env("MAXIMA_USERDIR", userdir.path())
        .assert()
        .success();

    // Editing the working copy after install changes what list reports
    write_manifest(src.path(), "0.2.0");
    mxpm()
        .args(["list", "--json"])
        .env("MAXIMA_USERDIR", userdir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("\"version\": \"0.2.0\""))
        .stdout(predicate::str::contains("0.1.0").not());
}
