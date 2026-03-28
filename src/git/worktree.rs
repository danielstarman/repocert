use std::collections::{BTreeMap, BTreeSet};
use std::io;
use std::path::Path;
use std::process::Command;

use thiserror::Error;

use crate::config::Contract;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct GitWorktreeSnapshot {
    entries: BTreeMap<String, String>,
}

impl GitWorktreeSnapshot {
    pub(crate) fn is_clean(&self) -> bool {
        self.entries.is_empty()
    }

    pub(crate) fn paths(&self) -> Vec<String> {
        self.entries.keys().cloned().collect()
    }

    pub(crate) fn changed_paths(&self, other: &Self) -> Vec<String> {
        let mut paths = BTreeSet::new();
        paths.extend(self.entries.keys().cloned());
        paths.extend(other.entries.keys().cloned());

        paths
            .into_iter()
            .filter(|path| self.entries.get(path) != other.entries.get(path))
            .collect()
    }
}

#[derive(Debug, Error)]
pub enum GitWorktreeError {
    #[error("failed to run git status")]
    Io {
        #[source]
        source: io::Error,
    },
    #[error("git status failed: {message}")]
    CommandFailed { message: String },
}

pub(crate) fn protected_pathspecs(contract: &Contract) -> Vec<String> {
    let mut pathspecs = vec![contract.built_in_protected_dir.as_str().to_string()];
    pathspecs.extend(
        contract
            .declared_protected_paths
            .iter()
            .map(|path| path.as_str().to_string()),
    );
    pathspecs
}

pub(crate) fn capture_pathspec_snapshot(
    repo_root: &Path,
    pathspecs: &[String],
) -> Result<GitWorktreeSnapshot, GitWorktreeError> {
    let mut command = Command::new("git");
    command.current_dir(repo_root);
    command.args([
        "--no-optional-locks",
        "status",
        "--porcelain=v1",
        "-z",
        "--untracked-files=all",
    ]);
    if !pathspecs.is_empty() {
        command.arg("--");
        command.args(pathspecs);
    }

    let output = command
        .output()
        .map_err(|source| GitWorktreeError::Io { source })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let message = stderr.trim();
        let message = if message.is_empty() {
            format!(
                "git status failed with exit code {:?}",
                output.status.code()
            )
        } else {
            message.to_string()
        };
        return Err(GitWorktreeError::CommandFailed { message });
    }

    Ok(parse_porcelain_v1_z(&output.stdout))
}

pub(crate) fn capture_worktree_snapshot(
    repo_root: &Path,
) -> Result<GitWorktreeSnapshot, GitWorktreeError> {
    capture_pathspec_snapshot(repo_root, &[])
}

fn parse_porcelain_v1_z(output: &[u8]) -> GitWorktreeSnapshot {
    let mut entries = BTreeMap::new();
    let mut fields = output
        .split(|byte| *byte == 0)
        .filter(|field| !field.is_empty());

    while let Some(field) = fields.next() {
        if field.len() < 4 {
            continue;
        }

        let status = String::from_utf8_lossy(&field[..2]).into_owned();
        let path = String::from_utf8_lossy(&field[3..]).into_owned();
        entries.insert(path, status.clone());

        if (status.contains('R') || status.contains('C'))
            && let Some(secondary) = fields.next()
        {
            let path = String::from_utf8_lossy(secondary).into_owned();
            entries.insert(path, status.clone());
        }
    }

    GitWorktreeSnapshot { entries }
}
