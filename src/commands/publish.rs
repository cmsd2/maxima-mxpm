use std::io::{self, Write};
use std::process::Command;

use serde::Serialize;

use crate::config::Config;
use crate::errors::MxpmError;
use crate::index::{PackageEntry, Source};
use crate::manifest;
use crate::output::{self, OutputFormat};

/// The upstream index repo (owner/name), derived from the default registry URL.
const INDEX_REPO: &str = "cmsd2/maxima-package-index";

#[derive(Serialize)]
struct PublishResult {
    pr_url: String,
    package: String,
    version: String,
    #[serde(rename = "ref")]
    git_ref: String,
}

/// Normalise a git remote URL to an HTTPS `.git` URL.
///
/// Handles SSH (`git@host:owner/repo.git`) and HTTPS forms.
fn normalise_remote_url(url: &str) -> String {
    let url = url.trim();

    // SSH: git@github.com:owner/repo.git -> https://github.com/owner/repo.git
    if let Some(rest) = url.strip_prefix("git@")
        && let Some((host, path)) = rest.split_once(':')
    {
        let path = path.strip_suffix(".git").unwrap_or(path);
        return format!("https://{host}/{path}.git");
    }

    // Already HTTPS — ensure it ends with .git
    let url = url.strip_suffix('/').unwrap_or(url);
    if url.ends_with(".git") {
        url.to_string()
    } else {
        format!("{url}.git")
    }
}

/// Get the GitHub username of the authenticated `gh` user.
fn gh_username() -> Result<String, MxpmError> {
    let output = Command::new("gh")
        .args(["api", "user", "--jq", ".login"])
        .output()
        .map_err(|e| {
            if e.kind() == io::ErrorKind::NotFound {
                MxpmError::GhNotFound
            } else {
                MxpmError::PublishFailed {
                    message: format!("failed to run gh: {e}"),
                }
            }
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(MxpmError::PublishFailed {
            message: format!("gh auth failed — run `gh auth login` first\n{stderr}"),
        });
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Run `gh` with the given arguments, returning stdout on success.
fn gh(args: &[&str]) -> Result<String, MxpmError> {
    let output = Command::new("gh").args(args).output().map_err(|e| {
        if e.kind() == io::ErrorKind::NotFound {
            MxpmError::GhNotFound
        } else {
            MxpmError::PublishFailed {
                message: format!("failed to run gh: {e}"),
            }
        }
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(MxpmError::PublishFailed {
            message: format!("gh {} failed: {}", args.join(" "), stderr.trim()),
        });
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn run(
    tag: Option<&str>,
    git_ref: Option<&str>,
    yes: bool,
    format: OutputFormat,
    _config: &Config,
) -> Result<(), MxpmError> {
    // 1. Read manifest
    let manifest_path = std::env::current_dir()?.join("manifest.toml");
    let manifest_contents =
        std::fs::read_to_string(&manifest_path).map_err(|_| MxpmError::ManifestNotFound {
            path: manifest_path.display().to_string(),
        })?;
    let manifest =
        manifest::parse_manifest(&manifest_contents).map_err(|e| MxpmError::PublishFailed {
            message: format!("failed to parse manifest.toml: {e}"),
        })?;
    let pkg = &manifest.package;

    // 2. Validate: repository field is required
    let _repository = pkg
        .repository
        .as_deref()
        .ok_or_else(|| MxpmError::PublishFailed {
            message: "manifest.toml must have a 'repository' field for publishing".to_string(),
        })?;

    // 3. Open git repo
    let repo = git2::Repository::open(".").map_err(|_| MxpmError::NotGitRepo)?;

    // 4. Resolve commit hash
    let commit_hash = if let Some(hash) = git_ref {
        // Validate 40-char hex
        if hash.len() != 40 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(MxpmError::PublishFailed {
                message: format!("--ref must be a 40-character hex commit hash, got: {hash}"),
            });
        }
        hash.to_string()
    } else if let Some(tag_name) = tag {
        // Resolve tag to commit hash
        let obj = repo
            .revparse_single(&format!("{tag_name}^{{commit}}"))
            .map_err(|e| MxpmError::PublishFailed {
                message: format!("failed to resolve tag '{tag_name}': {e}"),
            })?;
        obj.id().to_string()
    } else {
        // HEAD
        let head = repo.head().map_err(|e| MxpmError::PublishFailed {
            message: format!("failed to read HEAD: {e}"),
        })?;
        let commit = head
            .peel_to_commit()
            .map_err(|e| MxpmError::PublishFailed {
                message: format!("HEAD does not point to a commit: {e}"),
            })?;
        commit.id().to_string()
    };

    // 5. Get remote URL
    let remote = repo
        .find_remote("origin")
        .map_err(|_| MxpmError::NoGitRemote)?;
    let remote_url = remote.url().ok_or_else(|| MxpmError::PublishFailed {
        message: "origin remote URL is not valid UTF-8".to_string(),
    })?;
    let source_url = normalise_remote_url(remote_url);

    // 6. Build PackageEntry
    let entry = PackageEntry {
        description: pkg.description.clone(),
        repository: source_url.clone(),
        source: Source::Git {
            url: source_url.clone(),
            git_ref: commit_hash.clone(),
            subdir: None,
        },
        homepage: pkg.homepage.clone(),
        keywords: pkg.keywords.clone(),
        license: Some(pkg.license.clone()),
        authors: pkg.authors.as_ref().map(|a| a.names.clone()),
    };

    // 7. Show summary and confirm
    let short_hash = &commit_hash[..12];
    if matches!(format, OutputFormat::Human) {
        eprintln!("Publishing to {INDEX_REPO}:");
        eprintln!("  Package:    {}", pkg.name);
        eprintln!("  Version:    {}", pkg.version);
        eprintln!("  Commit:     {short_hash}");
        eprintln!("  Source:     {source_url}");
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

    // 8. Check gh auth
    if matches!(format, OutputFormat::Human) {
        eprintln!("Checking GitHub authentication...");
    }
    let gh_user = gh_username()?;

    // 9. Fork index repo (idempotent)
    if matches!(format, OutputFormat::Human) {
        eprintln!("Forking {INDEX_REPO} (if needed)...");
    }
    // --clone=false: don't clone locally, just ensure fork exists
    let _ = gh(&["repo", "fork", INDEX_REPO, "--clone=false"]);

    // 10. Clone fork to tempdir
    let tmp = tempfile::TempDir::new()?;
    let clone_dir = tmp.path().join("maxima-package-index");
    let fork_repo = format!("{gh_user}/maxima-package-index");

    if matches!(format, OutputFormat::Human) {
        eprintln!("Cloning {fork_repo}...");
    }
    gh(&[
        "repo",
        "clone",
        &fork_repo,
        clone_dir.to_str().unwrap(),
        "--",
        "--depth=1",
    ])?;

    // 11. Create branch
    let branch_name = format!("publish/{}-{short_hash}", pkg.name);
    let git_in_clone = |args: &[&str]| -> Result<String, MxpmError> {
        let output = Command::new("git")
            .args(args)
            .current_dir(&clone_dir)
            .output()
            .map_err(|e| MxpmError::PublishFailed {
                message: format!("git failed: {e}"),
            })?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(MxpmError::PublishFailed {
                message: format!("git {} failed: {}", args.join(" "), stderr.trim()),
            });
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    };

    // Sync fork with upstream before branching
    git_in_clone(&[
        "remote",
        "add",
        "upstream",
        &format!("https://github.com/{INDEX_REPO}.git"),
    ])?;
    // Determine the default branch of the upstream repo
    let upstream_head = git_in_clone(&["remote", "show", "upstream"])?;
    let default_branch = upstream_head
        .lines()
        .find_map(|line| line.trim().strip_prefix("HEAD branch:"))
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "master".to_string());
    git_in_clone(&["fetch", "upstream", &default_branch])?;
    git_in_clone(&["reset", "--hard", &format!("upstream/{default_branch}")])?;

    git_in_clone(&["checkout", "-b", &branch_name])?;

    // 12. Read index.json, update, write back
    //
    // Parse as serde_json::Value to preserve existing key order, field order,
    // and indentation. Only the new/updated entry is touched.
    let index_path = clone_dir.join("index.json");

    let index_contents =
        std::fs::read_to_string(&index_path).map_err(|e| MxpmError::PublishFailed {
            message: format!("failed to read index.json from clone: {e}"),
        })?;

    let mut index: serde_json::Value =
        serde_json::from_str(&index_contents).map_err(|e| MxpmError::PublishFailed {
            message: format!("failed to parse index.json: {e}"),
        })?;

    let packages = index
        .get_mut("packages")
        .and_then(|v| v.as_object_mut())
        .ok_or_else(|| MxpmError::PublishFailed {
            message: "index.json missing 'packages' object".to_string(),
        })?;

    let is_update = packages.contains_key(&pkg.name);

    // Serialize just the new entry
    let entry_value = serde_json::to_value(&entry).map_err(|e| MxpmError::PublishFailed {
        message: format!("failed to serialize package entry: {e}"),
    })?;
    packages.insert(pkg.name.clone(), entry_value);

    // serde_json::Map is backed by BTreeMap, so keys are sorted at every level
    let json = serde_json::to_string_pretty(&index).map_err(|e| MxpmError::PublishFailed {
        message: format!("failed to serialize index.json: {e}"),
    })?;
    // Ensure trailing newline
    std::fs::write(&index_path, format!("{json}\n"))?;

    // 13. Commit and push
    let action = if is_update { "Update" } else { "Add" };
    let commit_msg = format!("{action} {} {}", pkg.name, pkg.version);

    git_in_clone(&["add", "index.json"])?;
    git_in_clone(&["commit", "-m", &commit_msg])?;
    git_in_clone(&["push", "--force", "-u", "origin", &branch_name])?;

    // 14. Create PR or find existing one
    let head_ref = format!("{gh_user}:{branch_name}");

    // Check for an existing open PR from this branch
    // Note: gh pr list --head wants just the branch name, not user:branch
    let existing_pr = gh(&[
        "pr",
        "list",
        "--repo",
        INDEX_REPO,
        "--head",
        &branch_name,
        "--state",
        "open",
        "--json",
        "url",
        "--jq",
        ".[0].url",
    ]);

    let pr_url = match existing_pr {
        Ok(url) if !url.is_empty() => {
            if matches!(format, OutputFormat::Human) {
                eprintln!("Updated existing pull request.");
            }
            url
        }
        _ => {
            if matches!(format, OutputFormat::Human) {
                eprintln!("Creating pull request...");
            }

            let pr_title = format!("{action} {} {}", pkg.name, pkg.version);
            let pr_body = format!(
                "## {action} `{name}` {version}\n\n\
                 - **Source**: {source_url}\n\
                 - **Commit**: `{commit_hash}`\n\n\
                 Submitted via `mxpm publish`.",
                name = pkg.name,
                version = pkg.version,
            );

            gh(&[
                "pr", "create", "--repo", INDEX_REPO, "--head", &head_ref, "--title", &pr_title,
                "--body", &pr_body,
            ])?
        }
    };

    match format {
        OutputFormat::Json => {
            output::print_json(&PublishResult {
                pr_url,
                package: pkg.name.clone(),
                version: pkg.version.clone(),
                git_ref: commit_hash,
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
    use super::*;

    #[test]
    fn normalise_ssh_url() {
        assert_eq!(
            normalise_remote_url("git@github.com:user/repo.git"),
            "https://github.com/user/repo.git"
        );
    }

    #[test]
    fn normalise_ssh_url_without_git_suffix() {
        assert_eq!(
            normalise_remote_url("git@github.com:user/repo"),
            "https://github.com/user/repo.git"
        );
    }

    #[test]
    fn normalise_https_url() {
        assert_eq!(
            normalise_remote_url("https://github.com/user/repo"),
            "https://github.com/user/repo.git"
        );
    }

    #[test]
    fn normalise_https_url_with_git_suffix() {
        assert_eq!(
            normalise_remote_url("https://github.com/user/repo.git"),
            "https://github.com/user/repo.git"
        );
    }

    #[test]
    fn normalise_https_url_trailing_slash() {
        assert_eq!(
            normalise_remote_url("https://github.com/user/repo/"),
            "https://github.com/user/repo.git"
        );
    }

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
        // Find positions within the alpha entry
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
