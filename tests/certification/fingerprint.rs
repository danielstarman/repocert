use repocert::certification::{FingerprintError, compute_contract_fingerprint};
use repocert::config::LoadOptions;
use tempfile::TempDir;

use crate::{load_contract, write_repo_file};

#[test]
fn compute_contract_fingerprint_config_bytes_changed_returns_different_result() {
    // Arrange
    let repo = TempDir::new().unwrap();
    write_repo_file(&repo, ".repocert/config.toml", "schema_version = 1\n");

    let loaded = load_contract(LoadOptions::from_repo_root(repo.path()));
    let original = compute_contract_fingerprint(&loaded).unwrap();
    write_repo_file(
        &repo,
        ".repocert/config.toml",
        "schema_version = 1\n# changed\n",
    );

    // Act
    let changed = load_contract(LoadOptions::from_repo_root(repo.path()));
    let changed = compute_contract_fingerprint(&changed).unwrap();

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
    let original = load_contract(LoadOptions::from_repo_root(repo.path()));
    let original = compute_contract_fingerprint(&original).unwrap();

    write_repo_file(&repo, "docs/spec.md", "second\n");
    let changed = load_contract(LoadOptions::from_repo_root(repo.path()));

    // Act
    let changed = compute_contract_fingerprint(&changed).unwrap();

    // Assert
    assert_ne!(original, changed);
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
    let loaded = load_contract(LoadOptions::from_repo_root(repo.path()));

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
