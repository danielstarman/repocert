use std::ffi::OsStr;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use tempfile::NamedTempFile;

use super::{CertificationKey, CertificationRecord, StorageError};
use crate::git::resolve_git_common_dir;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CertificationStore {
    common_dir: PathBuf,
    root_dir: PathBuf,
}

impl CertificationStore {
    pub fn open(repo_root: &Path) -> Result<Self, StorageError> {
        let common_dir = resolve_git_common_dir(repo_root)?;
        let root_dir = common_dir.join("repocert").join("certifications");
        Ok(Self {
            common_dir,
            root_dir,
        })
    }

    pub fn common_dir(&self) -> &Path {
        &self.common_dir
    }

    pub fn root_dir(&self) -> &Path {
        &self.root_dir
    }

    pub fn read(
        &self,
        key: &CertificationKey,
    ) -> Result<Option<CertificationRecord>, StorageError> {
        let path = self.record_path(key)?;
        if !path.exists() {
            return Ok(None);
        }

        let bytes = fs::read(&path).map_err(|source| StorageError::Io {
            path: path.clone(),
            source,
        })?;
        let record: CertificationRecord =
            serde_json::from_slice(&bytes).map_err(|source| StorageError::Json {
                path: path.clone(),
                source,
            })?;
        if record.key != *key {
            return Err(StorageError::InvalidStoredRecordKey { path });
        }
        Ok(Some(record))
    }

    pub fn write(&self, record: &CertificationRecord) -> Result<(), StorageError> {
        let directory = self.commit_dir(&record.key.commit)?;
        fs::create_dir_all(&directory).map_err(|source| StorageError::Io {
            path: directory.clone(),
            source,
        })?;

        let path = self.record_path(&record.key)?;
        let bytes =
            serde_json::to_vec_pretty(record).expect("certification records should serialize");

        let mut temp_file =
            NamedTempFile::new_in(&directory).map_err(|source| StorageError::Io {
                path: directory.clone(),
                source,
            })?;
        temp_file
            .write_all(&bytes)
            .map_err(|source| StorageError::Io {
                path: path.clone(),
                source,
            })?;
        temp_file
            .write_all(b"\n")
            .map_err(|source| StorageError::Io {
                path: path.clone(),
                source,
            })?;
        temp_file.flush().map_err(|source| StorageError::Io {
            path: path.clone(),
            source,
        })?;

        temp_file
            .persist(&path)
            .map_err(|error| StorageError::Persist {
                path: path.clone(),
                source: error.error,
            })?;
        Ok(())
    }

    pub fn list_for_commit(&self, commit: &str) -> Result<Vec<CertificationRecord>, StorageError> {
        let directory = self.commit_dir(commit)?;
        if !directory.exists() {
            return Ok(Vec::new());
        }

        let mut entries = fs::read_dir(&directory)
            .map_err(|source| StorageError::Io {
                path: directory.clone(),
                source,
            })?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|source| StorageError::Io {
                path: directory.clone(),
                source,
            })?;
        entries.sort_by_key(|entry| entry.file_name());

        let mut records = Vec::new();
        for entry in entries {
            let path = entry.path();
            if path.extension() != Some(OsStr::new("json")) {
                continue;
            }
            decode_profile_name(&path)?;

            let bytes = fs::read(&path).map_err(|source| StorageError::Io {
                path: path.clone(),
                source,
            })?;
            let record: CertificationRecord =
                serde_json::from_slice(&bytes).map_err(|source| StorageError::Json {
                    path: path.clone(),
                    source,
                })?;
            let expected_key = CertificationKey {
                commit: commit.to_string(),
                profile: decode_profile_name(&path)?,
            };
            if record.key != expected_key {
                return Err(StorageError::InvalidStoredRecordKey { path });
            }
            records.push(record);
        }

        records.sort_by(|left, right| left.key.profile.cmp(&right.key.profile));
        Ok(records)
    }

    fn commit_dir(&self, commit: &str) -> Result<PathBuf, StorageError> {
        validate_commit_id(commit)?;
        Ok(self.root_dir.join(commit))
    }

    fn record_path(&self, key: &CertificationKey) -> Result<PathBuf, StorageError> {
        let directory = self.commit_dir(&key.commit)?;
        let profile = encode_profile_name(&key.profile);
        Ok(directory.join(format!("{profile}.json")))
    }
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

fn decode_profile_name(path: &Path) -> Result<String, StorageError> {
    let stem = path.file_stem().and_then(OsStr::to_str).ok_or_else(|| {
        StorageError::InvalidStoredProfileName {
            path: path.to_path_buf(),
        }
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
