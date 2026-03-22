use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum GitHeadError {
    #[error("failed to run git while resolving HEAD")]
    Io {
        #[source]
        source: io::Error,
    },
    #[error("path {repo_root:?} is not inside a git repository")]
    NotGitRepository { repo_root: PathBuf },
    #[error("git failed while resolving HEAD: {message}")]
    CommandFailed { message: String },
    #[error("git returned an invalid HEAD commit {commit:?}")]
    InvalidHeadCommit { commit: String },
}

pub(crate) fn resolve_head_commit(repo_root: &Path) -> Result<String, GitHeadError> {
    let output = Command::new("git")
        .current_dir(repo_root)
        .args(["rev-parse", "HEAD"])
        .output()
        .map_err(|source| GitHeadError::Io { source })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let message = stderr.trim().to_string();
        if message.contains("not a git repository") {
            return Err(GitHeadError::NotGitRepository {
                repo_root: repo_root.to_path_buf(),
            });
        }

        return Err(GitHeadError::CommandFailed { message });
    }

    let commit = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if commit.is_empty() || !commit.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(GitHeadError::InvalidHeadCommit { commit });
    }

    Ok(commit)
}
