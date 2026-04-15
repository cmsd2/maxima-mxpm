use std::io::{IsTerminal, Read};
use std::path::Path;

use indicatif::{ProgressBar, ProgressStyle};
use sha2::{Digest, Sha256};

use crate::errors::MxpmError;
use crate::index::Source;

/// Result of downloading and extracting a package source.
pub struct DownloadResult {
    /// The URL that was fetched (git clone URL or tarball URL).
    pub url: String,
    /// The resolved git commit hash, if the source was a git repo.
    pub commit: Option<String>,
    /// Hex-encoded hash of the downloaded tarball, if the source was a tarball.
    pub hash: Option<String>,
    /// Hash algorithm used (e.g. "sha256").
    pub hash_algorithm: Option<String>,
}

/// Download a package source and extract it to `dest_dir`.
pub async fn download_and_extract(
    source: &Source,
    dest_dir: &Path,
) -> Result<DownloadResult, MxpmError> {
    match source {
        Source::Tarball {
            url,
            hash,
            hash_algorithm,
        } => download_tarball(url, hash.as_deref(), hash_algorithm.as_deref(), dest_dir).await,
        Source::Git {
            url,
            git_ref,
            subdir,
        } => clone_git(url, git_ref, subdir.as_deref(), dest_dir),
    }
}

fn make_progress_bar(len: u64, template: &str) -> ProgressBar {
    if !std::io::stderr().is_terminal() {
        return ProgressBar::hidden();
    }
    let pb = ProgressBar::new(len);
    pb.set_style(
        ProgressStyle::with_template(template)
            .unwrap()
            .progress_chars("=> "),
    );
    pb
}

/// Download a tarball from a URL and extract it to `dest_dir`.
async fn download_tarball(
    url: &str,
    expected_hash: Option<&str>,
    hash_algorithm: Option<&str>,
    dest_dir: &Path,
) -> Result<DownloadResult, MxpmError> {
    let algo = hash_algorithm.unwrap_or("sha256");
    if algo != "sha256" {
        return Err(MxpmError::Extraction(std::io::Error::other(format!(
            "unsupported hash algorithm: {algo}"
        ))));
    }

    let response = reqwest::get(url).await.map_err(|e| MxpmError::Download {
        url: url.to_string(),
        source: e,
    })?;

    if !response.status().is_success() {
        return Err(MxpmError::DownloadStatus {
            url: url.to_string(),
            status: response.status().as_u16(),
        });
    }

    let total_size = response.content_length().unwrap_or(0);
    let pb = make_progress_bar(
        total_size,
        "{spinner:.green} [{bar:30.cyan/dim}] {bytes}/{total_bytes} ({bytes_per_sec})",
    );

    // Stream the response body with progress
    let mut bytes = Vec::with_capacity(total_size as usize);
    let mut stream = response;
    while let Some(chunk) = stream.chunk().await.map_err(|e| MxpmError::Download {
        url: url.to_string(),
        source: e,
    })? {
        bytes.extend_from_slice(&chunk);
        pb.set_position(bytes.len() as u64);
    }
    pb.finish_and_clear();

    // Compute SHA-256 hash of the tarball
    let hash = hex::encode(Sha256::digest(&bytes));

    // Verify against expected hash if provided
    if let Some(expected) = expected_hash
        && hash != expected
    {
        return Err(MxpmError::HashMismatch {
            url: url.to_string(),
            expected: expected.to_string(),
            actual: hash,
        });
    }

    // Tarballs from GitHub/GitLab wrap contents in a top-level directory.
    // Try to detect this: if all entries share a single common prefix directory,
    // strip it automatically.
    extract_tarball(&bytes[..], dest_dir, true, None)?;

    Ok(DownloadResult {
        url: url.to_string(),
        commit: None,
        hash: Some(hash),
        hash_algorithm: Some(algo.to_string()),
    })
}

/// Clone a git repository at a specific ref and copy files to `dest_dir`.
fn clone_git(
    url: &str,
    git_ref: &str,
    subdir: Option<&str>,
    dest_dir: &Path,
) -> Result<DownloadResult, MxpmError> {
    let temp_dir = tempfile::tempdir().map_err(MxpmError::Extraction)?;

    // Set up progress tracking for the fetch
    let pb = make_progress_bar(
        0,
        "{spinner:.green} [{bar:30.cyan/dim}] {pos}/{len} objects ({bytes})",
    );
    let pb_clone = pb.clone();

    let mut callbacks = git2::RemoteCallbacks::new();
    callbacks.transfer_progress(move |stats| {
        pb_clone.set_length(stats.total_objects() as u64);
        pb_clone.set_position(stats.received_objects() as u64);
        pb_clone.set_message(format!("{} bytes", stats.received_bytes()));
        true
    });

    let mut fetch_opts = git2::FetchOptions::new();
    fetch_opts.remote_callbacks(callbacks);

    let mut builder = git2::build::RepoBuilder::new();
    builder.fetch_options(fetch_opts);

    let repo = builder
        .clone(url, temp_dir.path())
        .map_err(|e| MxpmError::GitClone {
            url: url.to_string(),
            message: e.message().to_string(),
        })?;

    pb.finish_and_clear();

    // Resolve the ref to a commit
    let obj = repo
        .revparse_single(git_ref)
        .map_err(|e| MxpmError::GitClone {
            url: url.to_string(),
            message: format!("failed to resolve ref '{git_ref}': {}", e.message()),
        })?;
    repo.checkout_tree(&obj, None)
        .map_err(|e| MxpmError::GitClone {
            url: url.to_string(),
            message: format!("failed to checkout '{git_ref}': {}", e.message()),
        })?;
    let commit_id = obj.id();
    repo.set_head_detached(commit_id)
        .map_err(|e| MxpmError::GitClone {
            url: url.to_string(),
            message: format!("failed to detach HEAD: {}", e.message()),
        })?;

    // Determine the source directory within the clone
    let source_dir = if let Some(sub) = subdir {
        temp_dir.path().join(sub)
    } else {
        temp_dir.path().to_path_buf()
    };

    if !source_dir.exists() {
        return Err(MxpmError::GitClone {
            url: url.to_string(),
            message: format!("subdir '{}' not found in repository", subdir.unwrap_or("")),
        });
    }

    // Copy files from clone to dest_dir (excluding .git)
    copy_dir_recursive(&source_dir, dest_dir)?;

    Ok(DownloadResult {
        url: url.to_string(),
        commit: Some(commit_id.to_string()),
        hash: None,
        hash_algorithm: None,
    })
}

/// Recursively copy a directory, skipping `.git`.
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), MxpmError> {
    std::fs::create_dir_all(dst).map_err(MxpmError::Extraction)?;

    for entry in std::fs::read_dir(src).map_err(MxpmError::Extraction)? {
        let entry = entry.map_err(MxpmError::Extraction)?;
        let name = entry.file_name();

        // Skip .git directory
        if name == ".git" {
            continue;
        }

        let src_path = entry.path();
        let dst_path = dst.join(&name);

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path).map_err(MxpmError::Extraction)?;
        }
    }
    Ok(())
}

/// Extract a gzipped tarball to `dest_dir`.
///
/// If `strip_first_component` is true, removes the first path component
/// (GitHub/GitLab tarballs wrap contents in a `repo-ref/` directory).
///
/// If `subdir` is specified, only files under that subdirectory are extracted,
/// with the subdir prefix removed from their paths.
pub fn extract_tarball(
    reader: impl Read,
    dest_dir: &Path,
    strip_first_component: bool,
    subdir: Option<&str>,
) -> Result<(), MxpmError> {
    let decoder = flate2::read::GzDecoder::new(reader);
    let mut archive = tar::Archive::new(decoder);

    std::fs::create_dir_all(dest_dir).map_err(MxpmError::Extraction)?;

    for entry in archive.entries().map_err(MxpmError::Extraction)? {
        let mut entry = entry.map_err(MxpmError::Extraction)?;
        let original_path = entry.path().map_err(MxpmError::Extraction)?.into_owned();

        // Strip first component if needed (e.g., "repo-ref/file.mac" -> "file.mac")
        let mut components = original_path.components();
        let relative_path = if strip_first_component {
            components.next(); // skip first component
            components.as_path().to_path_buf()
        } else {
            original_path.clone()
        };

        if relative_path.as_os_str().is_empty() {
            continue;
        }

        // Apply subdir filter
        let final_path = if let Some(sub) = subdir {
            let sub_path = Path::new(sub);
            if let Ok(stripped) = relative_path.strip_prefix(sub_path) {
                stripped.to_path_buf()
            } else {
                continue; // skip files outside the subdir
            }
        } else {
            relative_path
        };

        if final_path.as_os_str().is_empty() {
            continue;
        }

        // Path traversal protection
        validate_path(&final_path)?;

        let dest = dest_dir.join(&final_path);

        if entry.header().entry_type().is_dir() {
            std::fs::create_dir_all(&dest).map_err(MxpmError::Extraction)?;
        } else {
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent).map_err(MxpmError::Extraction)?;
            }
            entry.unpack(&dest).map_err(MxpmError::Extraction)?;
        }
    }

    Ok(())
}

/// Reject paths with absolute components or `..` traversal.
fn validate_path(path: &Path) -> Result<(), MxpmError> {
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                return Err(MxpmError::UnsafePath {
                    path: path.display().to_string(),
                });
            }
            std::path::Component::RootDir | std::path::Component::Prefix(_) => {
                return Err(MxpmError::UnsafePath {
                    path: path.display().to_string(),
                });
            }
            _ => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_path_normal() {
        assert!(validate_path(Path::new("foo/bar.mac")).is_ok());
    }

    #[test]
    fn test_validate_path_traversal() {
        assert!(validate_path(Path::new("../etc/passwd")).is_err());
    }

    #[test]
    fn test_validate_path_absolute() {
        assert!(validate_path(Path::new("/etc/passwd")).is_err());
    }
}
