use repocert::config::{LoadError, LoadOptions, load_contract};
use tempfile::TempDir;

use crate::write_repo_file;

#[test]
fn load_contract_relative_protected_path_returns_normalized_repo_path() {
    // Arrange
    let repo = TempDir::new().unwrap();
    write_repo_file(
        &repo,
        ".repocert/config.toml",
        r#"
schema_version = 1
protected_paths = ["./docs/./spec.md"]
"#,
    );

    // Act
    let loaded = load_contract(LoadOptions::from_repo_root(repo.path())).unwrap();

    // Assert
    assert!(
        loaded
            .contract
            .declared_protected_paths
            .iter()
            .any(|path| path.as_str() == "docs/spec.md")
    );
}

#[test]
fn load_contract_protected_path_escape_returns_validation_error() {
    // Arrange
    let repo = TempDir::new().unwrap();
    write_repo_file(
        &repo,
        ".repocert/config.toml",
        r#"
schema_version = 1
protected_paths = ["../outside.txt"]
"#,
    );

    // Act
    let error = load_contract(LoadOptions::from_repo_root(repo.path())).unwrap_err();

    // Assert
    match error.error {
        LoadError::Validation(errors) => assert!(errors.to_string().contains("escape")),
        other => panic!("unexpected error: {other:?}"),
    }
}
