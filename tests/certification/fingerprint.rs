use std::collections::{BTreeMap, BTreeSet};

use repocert::certification::{FingerprintError, compute_contract_fingerprint};
use repocert::config::{Contract, LoadOptions, LoadPaths, LoadedContract, RepoPath, load_contract};
use tempfile::TempDir;

use crate::write_repo_file;

#[test]
fn compute_contract_fingerprint_config_bytes_changed_returns_different_result() {
    // Arrange
    let repo = TempDir::new().unwrap();
    write_repo_file(&repo, ".repocert/config.toml", "schema_version = 1\n");

    let mut loaded = load_contract(LoadOptions::from_repo_root(repo.path())).unwrap();
    let original = compute_contract_fingerprint(&loaded).unwrap();
    loaded.config_bytes = b"schema_version = 1\n# changed\n".to_vec();

    // Act
    let changed = compute_contract_fingerprint(&loaded).unwrap();

    // Assert
    assert_ne!(original, changed);
}

#[test]
fn compute_contract_fingerprint_declared_protected_file_bytes_changed_returns_different_result() {
    // Arrange
    let repo = TempDir::new().unwrap();
    write_repo_file(
        &repo,
        ".repocert/config.toml",
        "schema_version = 1\nprotected_paths = [\"docs/spec.md\"]\n",
    );
    write_repo_file(&repo, "docs/spec.md", "first\n");
    let original = load_contract(LoadOptions::from_repo_root(repo.path())).unwrap();
    let original = compute_contract_fingerprint(&original).unwrap();

    write_repo_file(&repo, "docs/spec.md", "second\n");
    let changed = load_contract(LoadOptions::from_repo_root(repo.path())).unwrap();

    // Act
    let changed = compute_contract_fingerprint(&changed).unwrap();

    // Assert
    assert_ne!(original, changed);
}

#[test]
fn compute_contract_fingerprint_declared_paths_in_different_insertion_order_returns_same_result() {
    // Arrange
    let repo = TempDir::new().unwrap();
    write_repo_file(&repo, "a.txt", "alpha\n");
    write_repo_file(&repo, "b.txt", "beta\n");
    let paths = LoadPaths {
        repo_root: repo.path().canonicalize().unwrap(),
        config_path: repo.path().join(".repocert/config.toml"),
    };
    let config_bytes = b"schema_version = 1\n".to_vec();

    let first = LoadedContract {
        paths: paths.clone(),
        config_bytes: config_bytes.clone(),
        contract: minimal_contract(["a.txt", "b.txt"]),
    };
    let second = LoadedContract {
        paths,
        config_bytes,
        contract: minimal_contract(["b.txt", "a.txt"]),
    };

    // Act
    let first = compute_contract_fingerprint(&first).unwrap();
    let second = compute_contract_fingerprint(&second).unwrap();

    // Assert
    assert_eq!(first, second);
}

#[test]
fn compute_contract_fingerprint_missing_declared_protected_file_returns_error() {
    // Arrange
    let repo = TempDir::new().unwrap();
    write_repo_file(
        &repo,
        ".repocert/config.toml",
        "schema_version = 1\nprotected_paths = [\"docs/spec.md\"]\n",
    );
    let loaded = load_contract(LoadOptions::from_repo_root(repo.path())).unwrap();

    // Act
    let error = compute_contract_fingerprint(&loaded).unwrap_err();

    // Assert
    match error {
        FingerprintError::ProtectedPathIo { path, .. } => {
            assert!(path.ends_with("docs/spec.md"));
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

fn minimal_contract<const N: usize>(paths: [&str; N]) -> Contract {
    let mut declared_protected_paths = BTreeSet::new();
    for path in paths {
        declared_protected_paths.insert(RepoPath::new(path.to_string()));
    }

    Contract {
        schema_version: 1,
        checks: BTreeMap::new(),
        fixers: BTreeMap::new(),
        profiles: BTreeMap::new(),
        default_profile: None,
        built_in_protected_dir: RepoPath::new(".repocert".to_string()),
        declared_protected_paths,
        protected_refs: Vec::new(),
        hooks: None,
    }
}
