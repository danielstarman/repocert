use std::path::{Path, PathBuf};

use crate::certification::hex;
use crate::certification::{CertificationKey, StorageError};

pub(super) fn commit_dir(root_dir: &Path, commit: &str) -> Result<PathBuf, StorageError> {
    validate_commit_id(commit)?;
    Ok(root_dir.join(commit))
}

pub(super) fn record_path(
    root_dir: &Path,
    key: &CertificationKey,
) -> Result<PathBuf, StorageError> {
    let directory = commit_dir(root_dir, &key.commit)?;
    let profile = encode_profile_name(&key.profile);
    Ok(directory.join(format!("{profile}.json")))
}

pub(super) fn decode_profile_name(path: &Path) -> Result<String, StorageError> {
    let stem = path
        .file_stem()
        .and_then(std::ffi::OsStr::to_str)
        .ok_or_else(|| StorageError::InvalidStoredProfileName {
            path: path.to_path_buf(),
        })?;

    let bytes = hex::decode(stem).ok_or_else(|| StorageError::InvalidStoredProfileName {
        path: path.to_path_buf(),
    })?;

    String::from_utf8(bytes).map_err(|_| StorageError::InvalidStoredProfileName {
        path: path.to_path_buf(),
    })
}

fn validate_commit_id(commit: &str) -> Result<(), StorageError> {
    if commit.is_empty() || !commit.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(StorageError::InvalidCommitId {
            commit: commit.to_string(),
        });
    }

    Ok(())
}

fn encode_profile_name(profile: &str) -> String {
    hex::encode(profile.as_bytes())
}
