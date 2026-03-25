use std::ffi::OsStr;
use std::fs;
use std::io::Write;
use std::path::Path;

use tempfile::NamedTempFile;

use crate::certification::{
    CertificationKey, CertificationPayload, CertificationRecord, SignedCertificationRecord,
    StorageError,
};

use super::layout;

pub(super) fn read_record(
    path: &Path,
    expected_key: &CertificationKey,
) -> Result<CertificationRecord, StorageError> {
    let bytes = fs::read(path).map_err(|source| StorageError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let record = parse_record(path, &bytes)?;
    if record.key() != expected_key {
        return Err(StorageError::InvalidStoredRecordKey {
            path: path.to_path_buf(),
        });
    }
    Ok(record)
}

pub(super) fn write_record(
    directory: &Path,
    record: &CertificationRecord,
) -> Result<(), StorageError> {
    fs::create_dir_all(directory).map_err(|source| StorageError::Io {
        path: directory.to_path_buf(),
        source,
    })?;

    let path = layout::record_path(
        directory
            .parent()
            .expect("commit directory should have a parent"),
        record.key(),
    )?;
    let bytes = serialize_record(record);

    let mut temp_file = NamedTempFile::new_in(directory).map_err(|source| StorageError::Io {
        path: directory.to_path_buf(),
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
            path,
            source: error.error,
        })?;
    Ok(())
}

pub(super) fn list_commit_records(
    directory: &Path,
    commit: &str,
) -> Result<Vec<CertificationRecord>, StorageError> {
    let mut entries = fs::read_dir(directory)
        .map_err(|source| StorageError::Io {
            path: directory.to_path_buf(),
            source,
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|source| StorageError::Io {
            path: directory.to_path_buf(),
            source,
        })?;
    entries.sort_by_key(|entry| entry.file_name());

    entries
        .into_iter()
        .filter(|entry| entry.path().extension() == Some(OsStr::new("json")))
        .map(|entry| {
            let path = entry.path();
            let expected_key = CertificationKey {
                commit: commit.to_string(),
                profile: layout::decode_profile_name(&path)?,
            };
            read_record(&path, &expected_key)
        })
        .collect()
}

pub(super) fn list_profile_records(
    root_dir: &Path,
    profile: &str,
) -> Result<Vec<CertificationRecord>, StorageError> {
    let mut commits = fs::read_dir(root_dir)
        .map_err(|source| StorageError::Io {
            path: root_dir.to_path_buf(),
            source,
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|source| StorageError::Io {
            path: root_dir.to_path_buf(),
            source,
        })?;
    commits.sort_by_key(|entry| entry.file_name());

    let mut records = Vec::new();
    for commit_entry in commits {
        let commit = commit_entry.file_name().to_string_lossy().into_owned();
        if !commit_entry.path().is_dir() {
            continue;
        }

        let key = CertificationKey {
            commit,
            profile: profile.to_string(),
        };
        let path = layout::record_path(root_dir, &key)?;
        if path.exists() {
            records.push(read_record(&path, &key)?);
        }
    }

    Ok(records)
}

fn parse_record(path: &Path, bytes: &[u8]) -> Result<CertificationRecord, StorageError> {
    if let Ok(record) = serde_json::from_slice::<SignedCertificationRecord>(bytes) {
        return Ok(CertificationRecord::Signed(record));
    }

    let payload: CertificationPayload =
        serde_json::from_slice(bytes).map_err(|source| StorageError::Json {
            path: path.to_path_buf(),
            source,
        })?;
    Ok(CertificationRecord::Legacy(payload))
}

fn serialize_record(record: &CertificationRecord) -> Vec<u8> {
    match record {
        CertificationRecord::Legacy(payload) => serde_json::to_vec_pretty(payload)
            .expect("legacy certification records should serialize"),
        CertificationRecord::Signed(record) => serde_json::to_vec_pretty(record)
            .expect("signed certification records should serialize"),
    }
}
