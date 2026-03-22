use repocert::certification::{
    CertificationKey, CertificationRecord, CertificationStore, ContractFingerprint, StorageError,
};
use std::fs;
use tempfile::TempDir;

use crate::{commit_all, init_git_repo, run_git, write_repo_file};

#[test]
fn certification_store_open_non_git_repo_returns_error() {
    // Arrange
    let repo = TempDir::new().unwrap();

    // Act
    let error = CertificationStore::open(repo.path()).unwrap_err();

    // Assert
    match error {
        StorageError::GitMetadata(_) => {}
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn certification_store_open_linked_worktree_uses_shared_common_dir() {
    // Arrange
    let repo = TempDir::new().unwrap();
    let worktree_parent = TempDir::new().unwrap();
    let worktree = worktree_parent.path().join("linked");
    init_git_repo(&repo);
    write_repo_file(&repo, "README.md", "repocert\n");
    commit_all(&repo, "initial");
    run_git(
        repo.path(),
        &["worktree", "add", "-q", worktree.to_str().unwrap()],
    );

    // Act
    let store = CertificationStore::open(&worktree).unwrap();

    // Assert
    assert_eq!(
        store.common_dir(),
        repo.path().join(".git").canonicalize().unwrap()
    );
    assert_eq!(
        store.root_dir(),
        repo.path()
            .join(".git/repocert/certifications")
            .canonicalize()
            .unwrap_or_else(|_| repo.path().join(".git/repocert/certifications"))
    );
}

#[test]
fn certification_store_write_then_read_returns_record() {
    // Arrange
    let repo = TempDir::new().unwrap();
    init_git_repo(&repo);
    let store = CertificationStore::open(repo.path()).unwrap();
    let record = CertificationRecord {
        key: CertificationKey {
            commit: "abc123".to_string(),
            profile: "default".to_string(),
        },
        contract_fingerprint: fingerprint(1),
    };

    // Act
    store.write(&record).unwrap();
    let loaded = store.read(&record.key).unwrap();

    // Assert
    assert_eq!(loaded, Some(record));
}

#[test]
fn certification_store_write_same_key_twice_updates_record() {
    // Arrange
    let repo = TempDir::new().unwrap();
    init_git_repo(&repo);
    let store = CertificationStore::open(repo.path()).unwrap();
    let key = CertificationKey {
        commit: "abc123".to_string(),
        profile: "default".to_string(),
    };
    let first = CertificationRecord {
        key: key.clone(),
        contract_fingerprint: fingerprint(1),
    };
    let second = CertificationRecord {
        key: key.clone(),
        contract_fingerprint: fingerprint(2),
    };

    // Act
    store.write(&first).unwrap();
    store.write(&second).unwrap();
    let loaded = store.read(&key).unwrap();

    // Assert
    assert_eq!(loaded, Some(second));
}

#[test]
fn certification_store_list_for_commit_returns_profiles_in_deterministic_order() {
    // Arrange
    let repo = TempDir::new().unwrap();
    init_git_repo(&repo);
    let store = CertificationStore::open(repo.path()).unwrap();
    let commit = "abc123";
    let beta = CertificationRecord {
        key: CertificationKey {
            commit: commit.to_string(),
            profile: "beta".to_string(),
        },
        contract_fingerprint: fingerprint(1),
    };
    let alpha = CertificationRecord {
        key: CertificationKey {
            commit: commit.to_string(),
            profile: "alpha:fmt".to_string(),
        },
        contract_fingerprint: fingerprint(2),
    };

    // Act
    store.write(&beta).unwrap();
    store.write(&alpha).unwrap();
    let listed = store.list_for_commit(commit).unwrap();

    // Assert
    assert_eq!(
        listed
            .into_iter()
            .map(|record| record.key.profile)
            .collect::<Vec<_>>(),
        vec!["alpha:fmt".to_string(), "beta".to_string()]
    );
}

#[test]
fn certification_store_invalid_commit_id_returns_error() {
    // Arrange
    let repo = TempDir::new().unwrap();
    init_git_repo(&repo);
    let store = CertificationStore::open(repo.path()).unwrap();
    let record = CertificationRecord {
        key: CertificationKey {
            commit: "refs/heads/main".to_string(),
            profile: "default".to_string(),
        },
        contract_fingerprint: fingerprint(1),
    };

    // Act
    let error = store.write(&record).unwrap_err();

    // Assert
    match error {
        StorageError::InvalidCommitId { commit } => assert_eq!(commit, "refs/heads/main"),
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn certification_store_read_mismatched_record_key_returns_error() {
    // Arrange
    let repo = TempDir::new().unwrap();
    init_git_repo(&repo);
    let store = CertificationStore::open(repo.path()).unwrap();
    let key = CertificationKey {
        commit: "abc123".to_string(),
        profile: "default".to_string(),
    };
    let wrong = CertificationRecord {
        key: CertificationKey {
            commit: "abc123".to_string(),
            profile: "other".to_string(),
        },
        contract_fingerprint: fingerprint(7),
    };
    let path = store.root_dir().join("abc123").join("64656661756c74.json");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, serde_json::to_vec_pretty(&wrong).unwrap()).unwrap();

    // Act
    let error = store.read(&key).unwrap_err();

    // Assert
    match error {
        StorageError::InvalidStoredRecordKey { .. } => {}
        other => panic!("unexpected error: {other:?}"),
    }
}

fn fingerprint(fill: u8) -> ContractFingerprint {
    ContractFingerprint::from_bytes([fill; 32])
}
