use std::path::{Path, PathBuf};

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

    if stem.len() % 2 != 0 {
        return Err(StorageError::InvalidStoredProfileName {
            path: path.to_path_buf(),
        });
    }

    let mut bytes = Vec::with_capacity(stem.len() / 2);
    let mut chars = stem.bytes();
    while let (Some(high), Some(low)) = (chars.next(), chars.next()) {
        let high = hex_value(high).ok_or_else(|| StorageError::InvalidStoredProfileName {
            path: path.to_path_buf(),
        })?;
        let low = hex_value(low).ok_or_else(|| StorageError::InvalidStoredProfileName {
            path: path.to_path_buf(),
        })?;
        bytes.push((high << 4) | low);
    }

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
    let mut encoded = String::with_capacity(profile.len() * 2);
    for byte in profile.as_bytes() {
        encoded.push(nibble_to_hex(byte >> 4));
        encoded.push(nibble_to_hex(byte & 0x0f));
    }
    encoded
}

fn nibble_to_hex(value: u8) -> char {
    match value {
        0..=9 => char::from(b'0' + value),
        10..=15 => char::from(b'a' + (value - 10)),
        _ => unreachable!("nibbles must stay within 0..=15"),
    }
}

fn hex_value(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        _ => None,
    }
}
