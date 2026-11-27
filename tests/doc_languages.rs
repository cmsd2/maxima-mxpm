//! Documentation in other languages: `doc build --lang` and
//! `doc generate-core-docs --lang`.

use std::fs;
use std::path::Path;

use assert_cmd::Command;
use predicates::prelude::*;

fn mxpm() -> Command {
    Command::cargo_bin("mxpm").unwrap()
}

fn fixture(path: &str) -> String {
    format!("{}/tests/fixtures/{path}", env!("CARGO_MANIFEST_DIR"))
}

fn json(path: &Path) -> serde_json::Value {
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

/// Whether the tools generate-core-docs runs are installed; the tests that
/// need them say so and pass without them, as on CI
fn have(tools: &[&str]) -> bool {
    let missing: Vec<&&str> = tools
        .iter()
        .filter(|t| {
            std::process::Command::new(t)
                .arg("--version")
                .output()
                .is_err()
        })
        .collect();
    if !missing.is_empty() {
        eprintln!("skipping: {missing:?} not installed");
    }
    missing.is_empty()
}

/// A package with English documentation and a Spanish translation beside it
fn package_with_translation(dir: &Path) {
    fs::create_dir_all(dir.join("doc")).unwrap();
    fs::write(
        dir.join("manifest.toml"),
        "[package]\nname = \"testpkg\"\nversion = \"0.1.0\"\ndescription = \"A test\"\nlicense = \"MIT\"\nentry = \"testpkg.mac\"\ndoc = \"doc/testpkg.md\"\n",
    )
    .unwrap();
    fs::copy(fixture("doc/testpkg.md"), dir.join("doc/testpkg.md")).unwrap();
    fs::write(
        dir.join("doc/testpkg.es.md"),
        "# testpkg\n\n## Definiciones para testpkg\n\n### Function: hello (name)\n\nDevuelve un saludo para name.\n",
    )
    .unwrap();
}

#[test]
fn a_translated_build_writes_only_its_index() {
    let dir = tempfile::tempdir().unwrap();
    package_with_translation(dir.path());
    mxpm()
        .current_dir(dir.path())
        .args(["doc", "build", "--lang", "es"])
        .assert()
        .success();
    let index = json(&dir.path().join("doc/testpkg-doc-index.es.json"));
    assert_eq!(index["language"], "es");
    assert_eq!(index["package"], "testpkg");
    assert_eq!(
        index["symbols"]["hello"]["summary"],
        "Devuelve un saludo para name."
    );
    // Maxima's own help stays English: no .info, no Lisp index
    assert!(!dir.path().join("testpkg.info").exists());
    assert!(!dir.path().join("testpkg-index.lisp").exists());
    assert!(!dir.path().join("doc/testpkg-doc-index.json").exists());
}

#[test]
fn a_translated_build_takes_a_file_and_a_slim_path() {
    let dir = tempfile::tempdir().unwrap();
    package_with_translation(dir.path());
    let slim = dir.path().join("slim.json");
    mxpm()
        .current_dir(dir.path())
        .args([
            "doc",
            "build",
            "doc/testpkg.es.md",
            "--lang",
            "es",
            "--slim",
        ])
        .arg(&slim)
        .assert()
        .success();
    assert!(dir.path().join("doc/testpkg-doc-index.es.json").exists());
    let slim = json(&slim);
    assert_eq!(slim["language"], "es");
    assert_eq!(slim["symbols"]["hello"]["body_md"], "");
}

#[test]
fn a_language_without_a_manual_is_an_error() {
    let out = tempfile::tempdir().unwrap();
    mxpm()
        .args(["doc", "generate-core-docs", "--maxima-src"])
        .arg(fixture("maxima-src"))
        .arg("--output")
        .arg(out.path())
        .args(["--lang", "fr"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No Maxima manual in fr"));
}

/// The English output, with the output directory as `@OUT@`, equals
/// what mxpm wrote before languages
#[test]
fn english_core_docs_are_unchanged() {
    if !have(&["makeinfo", "pandoc"]) {
        return;
    }
    let out = tempfile::tempdir().unwrap();
    mxpm()
        .args(["doc", "generate-core-docs", "--maxima-src"])
        .arg(fixture("maxima-src"))
        .arg("--output")
        .arg(out.path())
        .assert()
        .success();
    for file in [
        "manifest.toml",
        "maxima_core_docs.mac",
        "doc/maxima-core-docs.md",
        "doc/calculus-integration.md",
        "doc/maxima-core-docs-doc-index.json",
        "doc/maxima-core-docs-slim-doc-index.json",
    ] {
        let got = fs::read_to_string(out.path().join(file))
            .unwrap()
            .replace(&out.path().to_string_lossy().to_string(), "@OUT@");
        let want = fs::read_to_string(fixture(&format!("core-docs-en/{file}"))).unwrap();
        assert_eq!(got, want, "{file}");
    }
}

#[test]
fn spanish_core_docs_are_a_language_pack() {
    if !have(&["makeinfo"]) {
        return;
    }
    let out = tempfile::tempdir().unwrap();
    mxpm()
        .args(["doc", "generate-core-docs", "--maxima-src"])
        .arg(fixture("maxima-src"))
        .arg("--output")
        .arg(out.path())
        .args(["--lang", "es"])
        .assert()
        .success();
    let index = json(&out.path().join("doc/maxima-core-docs-doc-index.es.json"));
    assert_eq!(index["language"], "es");
    assert_eq!(index["package"], "maxima-core-docs-es");
    let integrate = &index["symbols"]["integrate"];
    assert!(
        integrate["summary"]
            .as_str()
            .unwrap()
            .starts_with("Calcula simbólicamente"),
        "{integrate}"
    );
    // The English tree's macros let the Spanish manual's categories through
    assert_eq!(integrate["category"], "Calculus");
    // English-only symbols aren't in the translation
    assert!(index["symbols"].get("defint").is_none());
    let slim = json(
        &out.path()
            .join("doc/maxima-core-docs-slim-doc-index.es.json"),
    );
    assert_eq!(slim["language"], "es");
    let manifest = fs::read_to_string(out.path().join("manifest.toml")).unwrap();
    assert!(
        manifest.contains("name = \"maxima-core-docs-es\""),
        "{manifest}"
    );
    assert!(!out.path().join("maxima-core-docs.info").exists());
}

/// German, in the fixture, has what stopped Maxima's German, Japanese and
/// Russian manuals: a template naming the version `@VERSION@`, a reference
/// to a node only the English manual has, a chapter only the English
/// manual has, and an end-of-sentence `@.`
#[test]
fn a_translation_with_errors_still_builds() {
    if !have(&["makeinfo"]) {
        return;
    }
    let out = tempfile::tempdir().unwrap();
    mxpm()
        .args(["doc", "generate-core-docs", "--maxima-src"])
        .arg(fixture("maxima-src"))
        .arg("--output")
        .arg(out.path())
        .args(["--lang", "de"])
        .assert()
        .success()
        .stderr(predicate::str::contains("kept going (--force)"))
        .stderr(predicate::str::contains("nonexistent node `defint'"));
    let index = json(&out.path().join("doc/maxima-core-docs-doc-index.de.json"));
    let integrate = &index["symbols"]["integrate"];
    assert!(
        integrate["summary"]
            .as_str()
            .unwrap()
            .starts_with("Berechnet das Integral von expr mit Maxima."),
        "{integrate}"
    );
    // The English-only chapter isn't passed off as German
    assert!(index["symbols"].get("limit").is_none(), "{index}");
}
