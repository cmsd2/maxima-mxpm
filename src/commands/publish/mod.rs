//! Publish a package to the community index via GitHub PR.

mod gh;
mod prepare;
mod submit;

use std::io::{self, Write};

use serde::Serialize;

use crate::config::Config;
use crate::errors::MxpmError;
use crate::output::{self, OutputFormat};

#[derive(Serialize)]
struct PublishResult {
    pr_url: String,
    package: String,
    version: String,
    #[serde(rename = "ref")]
    git_ref: String,
}

pub fn run(
    tag: Option<&str>,
    git_ref: Option<&str>,
    yes: bool,
    format: OutputFormat,
    _config: &Config,
) -> Result<(), MxpmError> {
    let prepared = prepare::prepare_publish(tag, git_ref)?;
    let short_hash = &prepared.commit_hash[..12];

    // Show summary and confirm
    if matches!(format, OutputFormat::Human) {
        eprintln!("Publishing to {}:", gh::INDEX_REPO);
        eprintln!("  Package:    {}", prepared.package_name);
        eprintln!("  Version:    {}", prepared.version);
        eprintln!("  Commit:     {short_hash}");
        eprintln!("  Source:     {}", prepared.source_url);
        eprintln!();
    }

    if !yes && matches!(format, OutputFormat::Human) {
        eprint!("Continue? [y/N] ");
        io::stderr().flush()?;
        let mut answer = String::new();
        io::stdin().read_line(&mut answer)?;
        if !answer.trim().eq_ignore_ascii_case("y") {
            eprintln!("Aborted.");
            return Ok(());
        }
    }

    let pr_url = submit::submit_to_index(&prepared, format)?;

    match format {
        OutputFormat::Json => {
            output::print_json(&PublishResult {
                pr_url,
                package: prepared.package_name,
                version: prepared.version,
                git_ref: prepared.commit_hash,
            })?;
        }
        OutputFormat::Human => {
            eprintln!("Done!");
            println!("{pr_url}");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn index_roundtrip_sorts_keys() {
        // Simulate an index.json with unsorted package keys and unsorted fields
        let input = r#"{
  "version": 1,
  "packages": {
    "zebra": {
      "source": {"type": "git", "url": "https://example.com/z.git", "ref": "aaaa"},
      "repository": "https://example.com/z",
      "description": "Z package"
    },
    "alpha": {
      "repository": "https://example.com/a",
      "description": "A package",
      "source": {"ref": "bbbb", "type": "git", "url": "https://example.com/a.git"}
    }
  }
}"#;

        let index: serde_json::Value = serde_json::from_str(input).unwrap();
        let output = serde_json::to_string_pretty(&index).unwrap();

        // Top-level keys sorted: "packages" before "version"
        let packages_pos = output.find("\"packages\"").unwrap();
        let version_pos = output.find("\"version\"").unwrap();
        assert!(
            packages_pos < version_pos,
            "top-level keys should be sorted"
        );

        // Package names sorted: "alpha" before "zebra"
        let alpha_pos = output.find("\"alpha\"").unwrap();
        let zebra_pos = output.find("\"zebra\"").unwrap();
        assert!(alpha_pos < zebra_pos, "package names should be sorted");

        // Fields within a package sorted: "description" before "repository" before "source"
        let alpha_section = &output[alpha_pos..];
        let desc_pos = alpha_section.find("\"description\"").unwrap();
        let repo_pos = alpha_section.find("\"repository\"").unwrap();
        let source_pos = alpha_section.find("\"source\"").unwrap();
        assert!(desc_pos < repo_pos, "description before repository");
        assert!(repo_pos < source_pos, "repository before source");

        // Source fields sorted: "ref" before "type" before "url"
        let source_section = &alpha_section[source_pos..];
        let ref_pos = source_section.find("\"ref\"").unwrap();
        let type_pos = source_section.find("\"type\"").unwrap();
        let url_pos = source_section.find("\"url\"").unwrap();
        assert!(ref_pos < type_pos, "ref before type");
        assert!(type_pos < url_pos, "type before url");
    }
}
