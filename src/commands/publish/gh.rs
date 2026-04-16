//! GitHub CLI (`gh`) helpers.

use std::io;
use std::process::Command;

use crate::errors::MxpmError;

/// The upstream index repo (owner/name), derived from the default registry URL.
pub(super) const INDEX_REPO: &str = "cmsd2/maxima-package-index";

/// Normalise a git remote URL to an HTTPS `.git` URL.
///
/// Handles SSH (`git@host:owner/repo.git`) and HTTPS forms.
pub(super) fn normalise_remote_url(url: &str) -> String {
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
pub(super) fn gh_username() -> Result<String, MxpmError> {
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
pub(super) fn gh(args: &[&str]) -> Result<String, MxpmError> {
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
}
