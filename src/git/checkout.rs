use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use thiserror::Error;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct GitCheckoutState {
    pub head_ref: Option<String>,
    pub is_primary_checkout: bool,
}

#[derive(Debug, Error)]
pub enum GitCheckoutError {
    #[error("failed to run git while inspecting checkout state")]
    Io {
        #[source]
        source: io::Error,
    },
    #[error("path {repo_root:?} is not inside a git repository")]
    NotGitRepository { repo_root: PathBuf },
    #[error("git failed while inspecting checkout state: {message}")]
    CommandFailed { message: String },
}

pub(crate) fn inspect_checkout(repo_root: &Path) -> Result<GitCheckoutState, GitCheckoutError> {
    let git_dir = resolve_path(repo_root, &run_git(repo_root, &["rev-parse", "--git-dir"])?);
    let git_common_dir = resolve_path(
        repo_root,
        &run_git(repo_root, &["rev-parse", "--git-common-dir"])?,
    );
    let head_ref = resolve_head_ref(repo_root)?;

    Ok(GitCheckoutState {
        head_ref,
        is_primary_checkout: git_dir == git_common_dir,
    })
}

fn resolve_head_ref(repo_root: &Path) -> Result<Option<String>, GitCheckoutError> {
    let output = Command::new("git")
        .current_dir(repo_root)
        .args(["symbolic-ref", "-q", "HEAD"])
        .output()
        .map_err(|source| GitCheckoutError::Io { source })?;

    if output.status.success() {
        let head_ref = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if head_ref.is_empty() {
            Ok(None)
        } else {
            Ok(Some(head_ref))
        }
    } else {
        let message = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if message.contains("not a git repository") {
            return Err(GitCheckoutError::NotGitRepository {
                repo_root: repo_root.to_path_buf(),
            });
        }
        if output.status.code() == Some(1) && message.is_empty() {
            return Ok(None);
        }

        Err(GitCheckoutError::CommandFailed { message })
    }
}

fn run_git(repo_root: &Path, args: &[&str]) -> Result<String, GitCheckoutError> {
    let output = Command::new("git")
        .current_dir(repo_root)
        .args(args)
        .output()
        .map_err(|source| GitCheckoutError::Io { source })?;

    if !output.status.success() {
        let message = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if message.contains("not a git repository") {
            return Err(GitCheckoutError::NotGitRepository {
                repo_root: repo_root.to_path_buf(),
            });
        }
        return Err(GitCheckoutError::CommandFailed { message });
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn resolve_path(repo_root: &Path, raw: &str) -> PathBuf {
    let path = PathBuf::from(raw);
    let path = if path.is_absolute() {
        path
    } else {
        repo_root.join(path)
    };

    path.canonicalize().unwrap_or(path)
}
