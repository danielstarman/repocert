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

pub(crate) fn read_core_hooks_path(repo_root: &Path) -> Result<Option<String>, GitHooksPathError> {
    let output = Command::new("git")
        .current_dir(repo_root)
        .args(["config", "--local", "--get", "core.hooksPath"])
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

pub(crate) fn write_core_hooks_path(
    repo_root: &Path,
    hooks_path: &Path,
) -> Result<(), GitHooksPathError> {
    let hooks_path = hooks_path.display().to_string();
    let output = Command::new("git")
        .current_dir(repo_root)
        .args(["config", "--local", "core.hooksPath", &hooks_path])
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
