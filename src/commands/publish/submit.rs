//! Submit a package entry to the index: fork, clone, update, push, and open PR.

use std::path::Path;
use std::process::Command;

use crate::errors::MxpmError;
use crate::output::OutputFormat;

use super::gh::{self, INDEX_REPO};
use super::prepare::PreparedPublish;

/// Fork the index repo, update index.json, push, and create/find a PR.
///
/// Returns the PR URL on success.
pub(super) fn submit_to_index(
    prepared: &PreparedPublish,
    format: OutputFormat,
) -> Result<String, MxpmError> {
    let short_hash = &prepared.commit_hash[..12];

    // 1. Check gh auth
    if matches!(format, OutputFormat::Human) {
        eprintln!("Checking GitHub authentication...");
    }
    let gh_user = gh::gh_username()?;

    // 2. Fork index repo (idempotent)
    if matches!(format, OutputFormat::Human) {
        eprintln!("Forking {INDEX_REPO} (if needed)...");
    }
    let _ = gh::gh(&["repo", "fork", INDEX_REPO, "--clone=false"]);

    // 3. Clone fork to tempdir
    let tmp = tempfile::TempDir::new()?;
    let clone_dir = tmp.path().join("maxima-package-index");
    let fork_repo = format!("{gh_user}/maxima-package-index");

    if matches!(format, OutputFormat::Human) {
        eprintln!("Cloning {fork_repo}...");
    }
    gh::gh(&[
        "repo",
        "clone",
        &fork_repo,
        clone_dir.to_str().unwrap(),
        "--",
        "--depth=1",
    ])?;

    // 4. Create branch
    let branch_name = format!("publish/{}-{short_hash}", prepared.package_name);

    // Sync fork with upstream before branching
    git_in_dir(
        &clone_dir,
        &[
            "remote",
            "add",
            "upstream",
            &format!("https://github.com/{INDEX_REPO}.git"),
        ],
    )?;
    let upstream_head = git_in_dir(&clone_dir, &["remote", "show", "upstream"])?;
    let default_branch = upstream_head
        .lines()
        .find_map(|line| line.trim().strip_prefix("HEAD branch:"))
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "master".to_string());
    git_in_dir(&clone_dir, &["fetch", "upstream", &default_branch])?;
    git_in_dir(
        &clone_dir,
        &["reset", "--hard", &format!("upstream/{default_branch}")],
    )?;
    git_in_dir(&clone_dir, &["checkout", "-b", &branch_name])?;

    // 5. Update index.json
    let is_update = update_index_json(&clone_dir, prepared)?;

    // 6. Commit and push
    let action = if is_update { "Update" } else { "Add" };
    let commit_msg = format!("{action} {} {}", prepared.package_name, prepared.version);

    git_in_dir(&clone_dir, &["add", "index.json"])?;
    git_in_dir(&clone_dir, &["commit", "-m", &commit_msg])?;
    git_in_dir(
        &clone_dir,
        &["push", "--force", "-u", "origin", &branch_name],
    )?;

    // 7. Create PR or find existing one
    let pr_url = create_or_find_pr(&gh_user, &branch_name, action, prepared, format)?;

    Ok(pr_url)
}

/// Update index.json in the cloned directory with the new package entry.
///
/// Returns `true` if this is an update to an existing package, `false` if new.
fn update_index_json(clone_dir: &Path, prepared: &PreparedPublish) -> Result<bool, MxpmError> {
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

    let is_update = packages.contains_key(&prepared.package_name);

    let entry_value =
        serde_json::to_value(&prepared.entry).map_err(|e| MxpmError::PublishFailed {
            message: format!("failed to serialize package entry: {e}"),
        })?;
    packages.insert(prepared.package_name.clone(), entry_value);

    // serde_json::Map is backed by BTreeMap, so keys are sorted at every level
    let json = serde_json::to_string_pretty(&index).map_err(|e| MxpmError::PublishFailed {
        message: format!("failed to serialize index.json: {e}"),
    })?;
    std::fs::write(&index_path, format!("{json}\n"))?;

    Ok(is_update)
}

/// Create a new PR or find an existing open one for this branch.
fn create_or_find_pr(
    gh_user: &str,
    branch_name: &str,
    action: &str,
    prepared: &PreparedPublish,
    format: OutputFormat,
) -> Result<String, MxpmError> {
    let head_ref = format!("{gh_user}:{branch_name}");

    // Check for an existing open PR from this branch
    let existing_pr = gh::gh(&[
        "pr",
        "list",
        "--repo",
        INDEX_REPO,
        "--head",
        branch_name,
        "--state",
        "open",
        "--json",
        "url",
        "--jq",
        ".[0].url",
    ]);

    match existing_pr {
        Ok(url) if !url.is_empty() => {
            if matches!(format, OutputFormat::Human) {
                eprintln!("Updated existing pull request.");
            }
            Ok(url)
        }
        _ => {
            if matches!(format, OutputFormat::Human) {
                eprintln!("Creating pull request...");
            }

            let pr_title = format!("{action} {} {}", prepared.package_name, prepared.version);
            let pr_body = format!(
                "## {action} `{name}` {version}\n\n\
                 - **Source**: {source_url}\n\
                 - **Commit**: `{commit_hash}`\n\n\
                 Submitted via `mxpm publish`.",
                name = prepared.package_name,
                version = prepared.version,
                source_url = prepared.source_url,
                commit_hash = prepared.commit_hash,
            );

            gh::gh(&[
                "pr", "create", "--repo", INDEX_REPO, "--head", &head_ref, "--title", &pr_title,
                "--body", &pr_body,
            ])
        }
    }
}

/// Run a git command in the given directory, returning stdout.
fn git_in_dir(dir: &Path, args: &[&str]) -> Result<String, MxpmError> {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
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
}
