use std::fs;
use std::path::Path;

use assert_cmd::Command;
use predicates::prelude::*;

fn mxpm() -> Command {
    Command::cargo_bin("mxpm").unwrap()
}

/// Path to the fixture directory.
fn fixtures_dir() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/doc"))
}

#[test]
fn doc_index_matches_expected_output() {
    let info_path = fixtures_dir().join("testpkg.info");
    let expected = fs::read_to_string(fixtures_dir().join("testpkg-index.lisp"))
        .unwrap()
        .replace("\r\n", "\n");

    let output = mxpm()
        .args(["doc", "index", info_path.to_str().unwrap(), "-o", "-"])
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(stdout, expected);
}

#[test]
fn doc_index_generates_lisp() {
    let info_path = fixtures_dir().join("testpkg.info");

    mxpm()
        .args(["doc", "index", info_path.to_str().unwrap(), "-o", "-"])
        .assert()
        .success()
        .stdout(predicate::str::contains("(in-package :cl-info)"))
        .stdout(predicate::str::contains("deffn-defvr-pairs"))
        .stdout(predicate::str::contains("section-pairs"))
        .stdout(predicate::str::contains("load-info-hashtables"))
        .stdout(predicate::str::contains("\"hello\""))
        .stdout(predicate::str::contains("\"greeting\""))
        .stdout(predicate::str::contains("\"testpkg.info\""))
        .stdout(predicate::str::contains("\"Definitions for testpkg\""));
}

#[test]
fn doc_index_output_file() {
    let info_path = fixtures_dir().join("testpkg.info");
    let dir = tempfile::tempdir().unwrap();
    let output_path = dir.path().join("testpkg-index.lisp");

    mxpm()
        .args([
            "doc",
            "index",
            info_path.to_str().unwrap(),
            "-o",
            output_path.to_str().unwrap(),
        ])
        .assert()
        .success();

    let content = fs::read_to_string(&output_path).unwrap();
    assert!(content.contains("(in-package :cl-info)"));
    assert!(content.contains("\"hello\""));
}

#[test]
fn doc_index_install_path_flag() {
    let info_path = fixtures_dir().join("testpkg.info");

    mxpm()
        .args([
            "doc",
            "index",
            info_path.to_str().unwrap(),
            "--install-path",
            "/usr/share/info/",
            "-o",
            "-",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("#p\"/usr/share/info/\""));
}

#[test]
fn doc_index_default_uses_maxima_load_pathname() {
    let info_path = fixtures_dir().join("testpkg.info");

    mxpm()
        .args(["doc", "index", info_path.to_str().unwrap(), "-o", "-"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "(maxima::maxima-load-pathname-directory)",
        ));
}

#[test]
fn doc_index_entries_sorted() {
    let info_path = fixtures_dir().join("testpkg.info");

    let output = mxpm()
        .args(["doc", "index", info_path.to_str().unwrap(), "-o", "-"])
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    let greeting_pos = stdout.find("\"greeting\"").expect("greeting not found");
    let hello_pos = stdout.find("\"hello\"").expect("hello not found");
    assert!(
        greeting_pos < hello_pos,
        "entries should be sorted alphabetically"
    );
}

#[test]
fn doc_index_default_output_path() {
    let dir = tempfile::tempdir().unwrap();
    let info_path = dir.path().join("testpkg.info");
    fs::copy(fixtures_dir().join("testpkg.info"), &info_path).unwrap();

    mxpm()
        .args(["doc", "index", info_path.to_str().unwrap()])
        .assert()
        .success();

    // Default output: <stem>-index.lisp next to the .info file
    let output_path = dir.path().join("testpkg-index.lisp");
    assert!(
        output_path.exists(),
        "default output file should be created"
    );
    let content = fs::read_to_string(&output_path).unwrap();
    assert!(content.contains("(in-package :cl-info)"));
}

#[test]
fn doc_index_missing_file_errors() {
    mxpm()
        .args(["doc", "index", "/nonexistent/path.info"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("info file not found"));
}

#[test]
fn doc_index_empty_info_warns() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("empty.info");

    // Minimal info file with no index or sections
    fs::write(
        &path,
        "This is empty.info, produced by makeinfo version 7.1 from\n\
         empty.texi.\n\n\
         \x1f\n\
         File: empty.info,  Node: Top,  Prev: (dir),  Up: (dir)\n\n\
         Nothing here.\n\n\
         \x1f\nTag Table:\n\
         \x1f\nEnd Tag Table\n",
    )
    .unwrap();

    mxpm()
        .args(["doc", "index", path.to_str().unwrap()])
        .assert()
        .success()
        .stderr(predicate::str::contains("empty index"));
}

/// Test that `mxpm doc index` can handle .texi input by invoking makeinfo.
/// Ignored by default because it requires makeinfo to be installed.
#[test]
#[ignore]
fn doc_index_from_texi() {
    // Copy the fixture .texi to a temp dir so makeinfo output doesn't pollute fixtures
    let dir = tempfile::tempdir().unwrap();
    let source = fixtures_dir().join("testpkg.texi");
    let dest = dir.path().join("testpkg.texi");
    fs::copy(&source, &dest).unwrap();

    let expected = fs::read_to_string(fixtures_dir().join("testpkg-index.lisp"))
        .unwrap()
        .replace("\r\n", "\n");

    mxpm()
        .args(["doc", "index", dest.to_str().unwrap()])
        .assert()
        .success();

    // Verify .info file was created in the temp dir
    assert!(dir.path().join("testpkg.info").exists());

    // Verify index file was created with expected content
    let index_path = dir.path().join("testpkg-index.lisp");
    let content = fs::read_to_string(&index_path).unwrap();
    assert_eq!(content, expected);
}

/// Verify that a .texi file without makeinfo on PATH gives a clear error.
#[test]
#[ignore]
fn doc_index_texi_without_makeinfo() {
    let dir = tempfile::tempdir().unwrap();
    let texi_path = dir.path().join("test.texi");
    fs::write(&texi_path, "\\input texinfo\n@bye\n").unwrap();

    mxpm()
        .args(["doc", "index", texi_path.to_str().unwrap()])
        .env("PATH", "/nonexistent")
        .assert()
        .failure()
        .stderr(predicate::str::contains("makeinfo"));
}

// ── mxpm doc build ──────────────────────────────────────────────────

#[test]
fn doc_build_rejects_non_texi() {
    mxpm()
        .args(["doc", "build", "foo.info"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("expected a .texi"));
}

#[test]
fn doc_build_missing_file() {
    mxpm()
        .args(["doc", "build", "/nonexistent/pkg.texi"])
        .assert()
        .failure();
}

#[test]
#[ignore]
fn doc_build_generates_info_and_index() {
    let dir = tempfile::tempdir().unwrap();
    let source = fixtures_dir().join("testpkg.texi");
    let dest = dir.path().join("testpkg.texi");
    fs::copy(&source, &dest).unwrap();

    mxpm()
        .args(["doc", "build", dest.to_str().unwrap()])
        .assert()
        .success()
        .stderr(predicate::str::contains("Wrote"));

    assert!(dir.path().join("testpkg.info").exists());
    assert!(dir.path().join("testpkg-index.lisp").exists());

    let index = fs::read_to_string(dir.path().join("testpkg-index.lisp")).unwrap();
    assert!(index.contains("(in-package :cl-info)"));
    assert!(index.contains("\"hello\""));
}

#[test]
#[ignore]
fn doc_build_output_dir() {
    let src_dir = tempfile::tempdir().unwrap();
    let out_dir = tempfile::tempdir().unwrap();
    let source = fixtures_dir().join("testpkg.texi");
    let dest = src_dir.path().join("testpkg.texi");
    fs::copy(&source, &dest).unwrap();

    mxpm()
        .args([
            "doc",
            "build",
            dest.to_str().unwrap(),
            "-o",
            out_dir.path().to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(out_dir.path().join("testpkg.info").exists());
    assert!(out_dir.path().join("testpkg-index.lisp").exists());
}

#[test]
#[ignore]
fn doc_build_with_xml() {
    let dir = tempfile::tempdir().unwrap();
    let source = fixtures_dir().join("testpkg.texi");
    let dest = dir.path().join("testpkg.texi");
    fs::copy(&source, &dest).unwrap();

    mxpm()
        .args(["doc", "build", dest.to_str().unwrap(), "--xml"])
        .assert()
        .success();

    assert!(dir.path().join("testpkg.xml").exists());
    let xml = fs::read_to_string(dir.path().join("testpkg.xml")).unwrap();
    assert!(xml.contains("<texinfo"));
}

#[test]
#[ignore]
fn doc_build_texi_with_mdbook_not_yet() {
    let dir = tempfile::tempdir().unwrap();
    let source = fixtures_dir().join("testpkg.texi");
    let dest = dir.path().join("testpkg.texi");
    fs::copy(&source, &dest).unwrap();

    mxpm()
        .args(["doc", "build", dest.to_str().unwrap(), "--mdbook"])
        .assert()
        .success()
        .stderr(predicate::str::contains("not yet implemented"));
}

// ── mxpm doc build (markdown input) ─────────────────────────────────

#[test]
#[ignore]
fn doc_build_from_markdown() {
    let dir = tempfile::tempdir().unwrap();
    let source = fixtures_dir().join("testpkg.md");
    let dest = dir.path().join("testpkg.md");
    fs::copy(&source, &dest).unwrap();

    mxpm()
        .args(["doc", "build", dest.to_str().unwrap()])
        .assert()
        .success();

    // Should produce .texi, .info, and -index.lisp
    assert!(dir.path().join("testpkg.texi").exists());
    assert!(dir.path().join("testpkg.info").exists());
    assert!(dir.path().join("testpkg-index.lisp").exists());

    let index = fs::read_to_string(dir.path().join("testpkg-index.lisp")).unwrap();
    assert!(index.contains("\"hello\""), "index should contain hello");
    assert!(
        index.contains("\"greeting\""),
        "index should contain greeting"
    );
}

#[test]
#[ignore]
fn doc_build_markdown_with_mdbook() {
    let dir = tempfile::tempdir().unwrap();
    let source = fixtures_dir().join("testpkg.md");
    let dest = dir.path().join("testpkg.md");
    fs::copy(&source, &dest).unwrap();

    mxpm()
        .args(["doc", "build", dest.to_str().unwrap(), "--mdbook"])
        .assert()
        .success();

    // mdBook structure
    assert!(dir.path().join("book/book.toml").exists());
    assert!(dir.path().join("book/src/SUMMARY.md").exists());

    let summary = fs::read_to_string(dir.path().join("book/src/SUMMARY.md")).unwrap();
    assert!(summary.contains("testpkg"));
}

#[test]
fn doc_build_rejects_info_file() {
    mxpm()
        .args(["doc", "build", "something.info"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("expected a .texi"));
}

// ── mxpm doc build (manifest-driven) ────────────────────────────────

#[test]
fn doc_build_no_manifest_no_file() {
    let dir = tempfile::tempdir().unwrap();

    mxpm()
        .args(["doc", "build"])
        .current_dir(dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("manifest.toml"));
}

#[test]
fn doc_build_manifest_no_doc_field() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("manifest.toml"),
        r#"[package]
name = "test-pkg"
version = "0.1.0"
description = "A test"
license = "MIT"
entry = "test.mac"
"#,
    )
    .unwrap();

    mxpm()
        .args(["doc", "build"])
        .current_dir(dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("no doc path"));
}

#[test]
#[ignore]
fn doc_build_from_manifest() {
    let dir = tempfile::tempdir().unwrap();

    // Create manifest
    fs::write(
        dir.path().join("manifest.toml"),
        r#"[package]
name = "test-pkg"
version = "0.1.0"
description = "A test"
license = "MIT"
entry = "test-pkg.mac"
doc = "doc/test-pkg.md"
"#,
    )
    .unwrap();

    // Create doc source
    let doc_dir = dir.path().join("doc");
    fs::create_dir(&doc_dir).unwrap();
    fs::copy(
        fixtures_dir().join("testpkg.md"),
        doc_dir.join("test-pkg.md"),
    )
    .unwrap();

    mxpm()
        .args(["doc", "build"])
        .current_dir(dir.path())
        .assert()
        .success();

    // .info and -index.lisp should be in the package root
    assert!(
        dir.path().join("test-pkg.info").exists(),
        ".info should be in package root"
    );
    assert!(
        dir.path().join("test-pkg-index.lisp").exists(),
        "-index.lisp should be in package root"
    );

    // .texi intermediate should be in doc/
    assert!(
        doc_dir.join("test-pkg.texi").exists(),
        ".texi should be next to the .md source"
    );

    let index = fs::read_to_string(dir.path().join("test-pkg-index.lisp")).unwrap();
    assert!(index.contains("(in-package :cl-info)"));
}

#[test]
#[ignore]
fn doc_build_explicit_file_finds_manifest() {
    let dir = tempfile::tempdir().unwrap();

    // Create manifest at package root
    fs::write(
        dir.path().join("manifest.toml"),
        r#"[package]
name = "test-pkg"
version = "0.1.0"
description = "A test"
license = "MIT"
entry = "test-pkg.mac"
doc = "doc/test-pkg.md"
"#,
    )
    .unwrap();

    // Create doc source in subdirectory
    let doc_dir = dir.path().join("doc");
    fs::create_dir(&doc_dir).unwrap();
    fs::copy(
        fixtures_dir().join("testpkg.md"),
        doc_dir.join("test-pkg.md"),
    )
    .unwrap();

    // Run with explicit file path (not manifest-driven)
    mxpm()
        .args([
            "doc",
            "build",
            doc_dir.join("test-pkg.md").to_str().unwrap(),
        ])
        .current_dir(dir.path())
        .assert()
        .success();

    // .info and -index.lisp should be in the package root (where manifest.toml is)
    assert!(
        dir.path().join("test-pkg.info").exists(),
        ".info should be in package root, not doc/"
    );
    assert!(
        dir.path().join("test-pkg-index.lisp").exists(),
        "-index.lisp should be in package root, not doc/"
    );
}

// ── mxpm doc watch / serve ──────────────────────────────────────────

#[test]
fn doc_watch_no_manifest_no_file() {
    let dir = tempfile::tempdir().unwrap();

    mxpm()
        .args(["doc", "watch"])
        .current_dir(dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("manifest.toml"));
}

#[test]
fn doc_serve_rejects_texi() {
    let dir = tempfile::tempdir().unwrap();
    let texi = dir.path().join("test.texi");
    fs::write(&texi, "\\input texinfo\n@bye\n").unwrap();

    mxpm()
        .args(["doc", "serve", texi.to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("requires a .md source"));
}

#[test]
fn doc_serve_no_manifest_no_file() {
    let dir = tempfile::tempdir().unwrap();

    mxpm()
        .args(["doc", "serve"])
        .current_dir(dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("manifest.toml"));
}
