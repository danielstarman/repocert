use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum GitHooksPathError {
    #[error("failed to run git while reading or writing core.hooksPath")]
    Io {
        #[source]
        source: io::Error,
    },
    #[error("path {repo_root:?} is not inside a git repository")]
    NotGitRepository { repo_root: PathBuf },
    #[error("git config failed: {message}")]
    CommandFailed { message: String },
}

pub(crate) fn read_local_core_hooks_path(
    repo_root: &Path,
) -> Result<Option<String>, GitHooksPathError> {
    read_core_hooks_path_with_args(repo_root, &["config", "--local", "--get", "core.hooksPath"])
}

pub(crate) fn read_worktree_core_hooks_path(
    repo_root: &Path,
) -> Result<Option<String>, GitHooksPathError> {
    read_core_hooks_path_with_args(
        repo_root,
        &["config", "--worktree", "--get", "core.hooksPath"],
    )
}

fn read_core_hooks_path_with_args(
    repo_root: &Path,
    args: &[&str],
) -> Result<Option<String>, GitHooksPathError> {
    let output = Command::new("git")
        .current_dir(repo_root)
        .args(args)
        .output()
        .map_err(|source| GitHooksPathError::Io { source })?;

    if output.status.success() {
        let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if value.is_empty() {
            Ok(None)
        } else {
            Ok(Some(value))
        }
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let message = stderr.trim().to_string();
        if message.contains("not in a git directory") || message.contains("not a git repository") {
            return Err(GitHooksPathError::NotGitRepository {
                repo_root: repo_root.to_path_buf(),
            });
        }
        if output.status.code() == Some(1) && message.is_empty() {
            return Ok(None);
        }

        Err(GitHooksPathError::CommandFailed { message })
    }
}

pub(crate) fn enable_worktree_config(repo_root: &Path) -> Result<bool, GitHooksPathError> {
    if read_worktree_config_flag(repo_root, "extensions.worktreeConfig")?.as_deref() == Some("true")
    {
        return Ok(false);
    }

    run_git_config(
        repo_root,
        &["config", "--local", "extensions.worktreeConfig", "true"],
    )?;
    Ok(true)
}

pub(crate) fn unset_local_core_hooks_path(repo_root: &Path) -> Result<bool, GitHooksPathError> {
    if read_local_core_hooks_path(repo_root)?.is_none() {
        return Ok(false);
    }

    run_git_config(
        repo_root,
        &["config", "--local", "--unset-all", "core.hooksPath"],
    )?;
    Ok(true)
}

pub(crate) fn write_worktree_core_hooks_path(
    repo_root: &Path,
    hooks_path: &str,
) -> Result<(), GitHooksPathError> {
    run_git_config(
        repo_root,
        &["config", "--worktree", "core.hooksPath", hooks_path],
    )
}

fn read_worktree_config_flag(
    repo_root: &Path,
    key: &str,
) -> Result<Option<String>, GitHooksPathError> {
    let output = Command::new("git")
        .current_dir(repo_root)
        .args(["config", "--local", "--get", key])
        .output()
        .map_err(|source| GitHooksPathError::Io { source })?;

    if output.status.success() {
        let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if value.is_empty() {
            Ok(None)
        } else {
            Ok(Some(value))
        }
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let message = stderr.trim().to_string();
        if message.contains("not in a git directory") || message.contains("not a git repository") {
            return Err(GitHooksPathError::NotGitRepository {
                repo_root: repo_root.to_path_buf(),
            });
        }
        if output.status.code() == Some(1) && message.is_empty() {
            return Ok(None);
        }

        Err(GitHooksPathError::CommandFailed { message })
    }
}

fn run_git_config(repo_root: &Path, args: &[&str]) -> Result<(), GitHooksPathError> {
    let output = Command::new("git")
        .current_dir(repo_root)
        .args(args)
        .output()
        .map_err(|source| GitHooksPathError::Io { source })?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let message = stderr.trim().to_string();
        if message.contains("not in a git directory") || message.contains("not a git repository") {
            return Err(GitHooksPathError::NotGitRepository {
                repo_root: repo_root.to_path_buf(),
            });
        }

        Err(GitHooksPathError::CommandFailed { message })
    }
}
