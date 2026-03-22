use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::process::Command;

use crate::config::Contract;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct GitStatusSnapshot {
    entries: BTreeMap<String, String>,
}

impl GitStatusSnapshot {
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

pub(crate) fn capture_snapshot(
    repo_root: &Path,
    pathspecs: &[String],
) -> Result<GitStatusSnapshot, String> {
    let mut command = Command::new("git");
    command.current_dir(repo_root);
    command.args([
        "--no-optional-locks",
        "status",
        "--porcelain=v1",
        "-z",
        "--untracked-files=all",
        "--",
    ]);
    command.args(pathspecs);

    let output = command
        .output()
        .map_err(|error| format!("failed to run git status: {error}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let message = stderr.trim();
        return Err(if message.is_empty() {
            format!(
                "git status failed with exit code {:?}",
                output.status.code()
            )
        } else {
            format!("git status failed: {message}")
        });
    }

    Ok(parse_porcelain_v1_z(&output.stdout))
}

fn parse_porcelain_v1_z(output: &[u8]) -> GitStatusSnapshot {
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

        if status.contains('R') || status.contains('C') {
            if let Some(secondary) = fields.next() {
                let path = String::from_utf8_lossy(secondary).into_owned();
                entries.insert(path, status.clone());
            }
        }
    }

    GitStatusSnapshot { entries }
}
