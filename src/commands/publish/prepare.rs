//! Prepare a publish operation: read manifest, resolve commit, build entry.

use crate::errors::MxpmError;
use crate::index::{PackageEntry, Source};
use crate::manifest;

use super::gh::normalise_remote_url;

/// All the information needed to publish a package, gathered before confirmation.
pub(super) struct PreparedPublish {
    pub package_name: String,
    pub version: String,
    pub commit_hash: String,
    pub source_url: String,
    pub entry: PackageEntry,
}

/// Read manifest, validate fields, resolve the commit hash, and build the index entry.
pub(super) fn prepare_publish(
    tag: Option<&str>,
    git_ref: Option<&str>,
) -> Result<PreparedPublish, MxpmError> {
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
    let commit_hash = resolve_commit(&repo, tag, git_ref)?;

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

    Ok(PreparedPublish {
        package_name: pkg.name.clone(),
        version: pkg.version.clone(),
        commit_hash,
        source_url,
        entry,
    })
}

/// Resolve the commit hash from --ref, --tag, or HEAD.
fn resolve_commit(
    repo: &git2::Repository,
    tag: Option<&str>,
    git_ref: Option<&str>,
) -> Result<String, MxpmError> {
    if let Some(hash) = git_ref {
        // Validate 40-char hex
        if hash.len() != 40 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(MxpmError::PublishFailed {
                message: format!("--ref must be a 40-character hex commit hash, got: {hash}"),
            });
        }
        Ok(hash.to_string())
    } else if let Some(tag_name) = tag {
        // Resolve tag to commit hash
        let obj = repo
            .revparse_single(&format!("{tag_name}^{{commit}}"))
            .map_err(|e| MxpmError::PublishFailed {
                message: format!("failed to resolve tag '{tag_name}': {e}"),
            })?;
        Ok(obj.id().to_string())
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
        Ok(commit.id().to_string())
    }
}
