use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum GitCommonDirError {
    #[error("failed to run git while resolving the git common dir")]
    Io {
        #[source]
        source: io::Error,
    },
    #[error("path {repo_root:?} is not inside a git repository")]
    NotGitRepository { repo_root: PathBuf },
    #[error("git did not return a common dir for {repo_root:?}")]
    MissingCommonDirOutput { repo_root: PathBuf },
    #[error("git common dir {path:?} does not exist or is not a directory")]
    MissingCommonDir { path: PathBuf },
    #[error("git failed while resolving the common dir: {message}")]
    CommandFailed { message: String },
}

#[derive(Debug, Error)]
pub enum GitDirError {
    #[error("failed to run git while resolving the git dir")]
    Io {
        #[source]
        source: io::Error,
    },
    #[error("path {repo_root:?} is not inside a git repository")]
    NotGitRepository { repo_root: PathBuf },
    #[error("git did not return a git dir for {repo_root:?}")]
    MissingGitDirOutput { repo_root: PathBuf },
    #[error("git dir {path:?} does not exist or is not a directory")]
    MissingGitDir { path: PathBuf },
    #[error("git failed while resolving the git dir: {message}")]
    CommandFailed { message: String },
}

pub(crate) fn resolve_git_common_dir(repo_root: &Path) -> Result<PathBuf, GitCommonDirError> {
    let output = Command::new("git")
        .current_dir(repo_root)
        .args(["rev-parse", "--git-common-dir"])
        .output()
        .map_err(|source| GitCommonDirError::Io { source })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let message = stderr.trim().to_string();
        if message.contains("not a git repository") {
            return Err(GitCommonDirError::NotGitRepository {
                repo_root: repo_root.to_path_buf(),
            });
        }

        return Err(GitCommonDirError::CommandFailed { message });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let common_dir = stdout.trim();
    if common_dir.is_empty() {
        return Err(GitCommonDirError::MissingCommonDirOutput {
            repo_root: repo_root.to_path_buf(),
        });
    }

    let path = PathBuf::from(common_dir);
    let path = if path.is_absolute() {
        path
    } else {
        repo_root.join(path)
    };
    let path = path
        .canonicalize()
        .map_err(|_| GitCommonDirError::MissingCommonDir { path: path.clone() })?;
    if !path.is_dir() {
        return Err(GitCommonDirError::MissingCommonDir { path });
    }

    Ok(path)
}

pub(crate) fn resolve_git_dir(repo_root: &Path) -> Result<PathBuf, GitDirError> {
    let output = Command::new("git")
        .current_dir(repo_root)
        .args(["rev-parse", "--git-dir"])
        .output()
        .map_err(|source| GitDirError::Io { source })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let message = stderr.trim().to_string();
        if message.contains("not a git repository") {
            return Err(GitDirError::NotGitRepository {
                repo_root: repo_root.to_path_buf(),
            });
        }

        return Err(GitDirError::CommandFailed { message });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let git_dir = stdout.trim();
    if git_dir.is_empty() {
        return Err(GitDirError::MissingGitDirOutput {
            repo_root: repo_root.to_path_buf(),
        });
    }

    let path = PathBuf::from(git_dir);
    let path = if path.is_absolute() {
        path
    } else {
        repo_root.join(path)
    };
    let path = path
        .canonicalize()
        .map_err(|_| GitDirError::MissingGitDir { path: path.clone() })?;
    if !path.is_dir() {
        return Err(GitDirError::MissingGitDir { path });
    }

    Ok(path)
}
