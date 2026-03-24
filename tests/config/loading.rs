use repocert::config::{HookMode, LoadError, LoadOptions, load_contract};
use tempfile::TempDir;

#[cfg(unix)]
use std::fs;
#[cfg(unix)]
use std::os::unix::fs as unix_fs;

use crate::write_repo_file;

#[test]
fn load_contract_discovered_config_returns_validated_contract() {
    // Arrange
    let repo = TempDir::new().unwrap();
    write_repo_file(
        &repo,
        ".repocert/config.toml",
        r#"
schema_version = 1
protected_paths = ["docs/spec.md", "./README.md"]

[checks.fmt]
argv = ["cargo", "fmt", "--check"]
timeout_ms = 1000

[checks.test]
argv = ["cargo", "test"]

[fixers.fmt]
argv = ["cargo", "fmt"]
probe_argv = ["cargo", "fmt", "--check"]

[profiles.base]
checks = ["fmt"]

[profiles.release]
includes = ["base"]
checks = ["test"]
fixers = ["fmt"]
default = true
certify = true

[[protected_refs]]
pattern = "refs/heads/main"
profile = "release"

[hooks]
mode = "generated"
"#,
    );
    std::fs::create_dir_all(repo.path().join("nested/work")).unwrap();

    // Act
    let loaded =
        load_contract(LoadOptions::discover_from(repo.path().join("nested/work"))).unwrap();

    // Assert
    assert_eq!(loaded.paths.repo_root, repo.path().canonicalize().unwrap());
    assert_eq!(
        loaded.paths.config_path,
        repo.path()
            .join(".repocert/config.toml")
            .canonicalize()
            .unwrap()
    );
    assert_eq!(loaded.contract.default_profile.as_deref(), Some("release"));
    assert_eq!(
        loaded
            .contract
            .profiles
            .get("release")
            .unwrap()
            .effective_checks,
        vec!["fmt".to_string(), "test".to_string()]
    );
    assert!(
        loaded
            .contract
            .declared_protected_paths
            .iter()
            .any(|path| path.as_str() == "README.md")
    );
    assert_eq!(
        loaded.contract.hooks.as_ref().unwrap().mode,
        HookMode::Generated
    );
}

#[test]
fn load_contract_repo_root_without_default_config_returns_discovery_error() {
    // Arrange
    let repo = TempDir::new().unwrap();

    // Act
    let error = load_contract(LoadOptions::from_repo_root(repo.path())).unwrap_err();

    // Assert
    match error.error {
        LoadError::Discovery(error) => {
            let message = error.to_string();
            assert!(message.contains(".repocert/config.toml"));
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn load_contract_mismatched_repo_root_and_config_path_returns_discovery_error() {
    // Arrange
    let repo = TempDir::new().unwrap();
    let other = TempDir::new().unwrap();
    write_repo_file(&repo, ".repocert/config.toml", "schema_version = 1");
    write_repo_file(&other, ".repocert/config.toml", "schema_version = 1");

    // Act
    let error = load_contract(LoadOptions {
        start_dir: None,
        repo_root: Some(repo.path().to_path_buf()),
        config_path: Some(other.path().join(".repocert/config.toml")),
    })
    .unwrap_err();

    // Assert
    match error.error {
        LoadError::Discovery(error) => {
            assert!(error.to_string().contains("do not match"));
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[cfg(unix)]
#[test]
fn load_contract_symlinked_config_returns_canonical_path_in_all_modes() {
    // Arrange
    let repo = TempDir::new().unwrap();
    write_repo_file(&repo, ".repocert/real-config.toml", "schema_version = 1");
    fs::create_dir_all(repo.path().join("nested/work")).unwrap();
    unix_fs::symlink(
        repo.path().join(".repocert/real-config.toml"),
        repo.path().join(".repocert/config.toml"),
    )
    .unwrap();

    // Act
    let from_repo_root = load_contract(LoadOptions::from_repo_root(repo.path())).unwrap();
    let from_config_path = load_contract(LoadOptions::from_config_path(
        repo.path().join(".repocert/config.toml"),
    ))
    .unwrap();
    let from_discovery =
        load_contract(LoadOptions::discover_from(repo.path().join("nested/work"))).unwrap();

    // Assert
    let canonical_target = repo
        .path()
        .join(".repocert/real-config.toml")
        .canonicalize()
        .unwrap();
    assert_eq!(from_repo_root.paths.config_path, canonical_target);
    assert_eq!(from_config_path.paths.config_path, canonical_target);
    assert_eq!(from_discovery.paths.config_path, canonical_target);
}

#[test]
fn load_contract_invalid_schema_version_returns_validation_error() {
    // Arrange
    let repo = TempDir::new().unwrap();
    write_repo_file(&repo, ".repocert/config.toml", "schema_version = 2");

    // Act
    let error = load_contract(LoadOptions::from_repo_root(repo.path())).unwrap_err();

    // Assert
    assert_eq!(
        error.paths.as_ref().unwrap().repo_root,
        repo.path().canonicalize().unwrap()
    );
    assert_eq!(
        error.paths.as_ref().unwrap().config_path,
        repo.path()
            .join(".repocert/config.toml")
            .canonicalize()
            .unwrap()
    );
    match error.error {
        LoadError::Validation(errors) => {
            assert!(errors.to_string().contains("schema_version"));
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn load_contract_profile_include_cycle_returns_validation_error() {
    // Arrange
    let repo = TempDir::new().unwrap();
    write_repo_file(
        &repo,
        ".repocert/config.toml",
        r#"
schema_version = 1

[checks.test]
argv = ["cargo", "test"]

[profiles.a]
includes = ["b"]
certify = true

[profiles.b]
includes = ["a"]
checks = ["test"]
"#,
    );

    // Act
    let error = load_contract(LoadOptions::from_repo_root(repo.path())).unwrap_err();

    // Assert
    match error.error {
        LoadError::Validation(errors) => {
            assert!(errors.to_string().contains("profile include cycle"));
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn load_contract_profile_fixer_without_probe_returns_validation_error() {
    // Arrange
    let repo = TempDir::new().unwrap();
    write_repo_file(
        &repo,
        ".repocert/config.toml",
        r#"
schema_version = 1

[checks.test]
argv = ["cargo", "test"]

[fixers.fmt]
argv = ["cargo", "fmt"]

[profiles.release]
checks = ["test"]
fixers = ["fmt"]
certify = true
"#,
    );

    // Act
    let error = load_contract(LoadOptions::from_repo_root(repo.path())).unwrap_err();

    // Assert
    match error.error {
        LoadError::Validation(errors) => assert!(errors.to_string().contains("probe_argv")),
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn load_contract_local_policy_requires_hooks_configuration() {
    // Arrange
    let repo = TempDir::new().unwrap();
    write_repo_file(
        &repo,
        ".repocert/config.toml",
        r#"
schema_version = 1

[local_policy]
protected_branches = ["refs/heads/main"]
require_clean_primary_checkout = true
"#,
    );

    // Act
    let error = load_contract(LoadOptions::from_repo_root(repo.path())).unwrap_err();

    // Assert
    match error.error {
        LoadError::Validation(errors) => {
            assert!(errors.to_string().contains("requires hooks configuration"));
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn load_contract_local_policy_with_generated_hooks_returns_validated_contract() {
    // Arrange
    let repo = TempDir::new().unwrap();
    write_repo_file(
        &repo,
        ".repocert/config.toml",
        r#"
schema_version = 1

[local_policy]
protected_branches = ["refs/heads/main"]
require_clean_primary_checkout = true

[hooks]
mode = "generated"
"#,
    );

    // Act
    let loaded = load_contract(LoadOptions::from_repo_root(repo.path())).unwrap();

    // Assert
    let local_policy = loaded.contract.local_policy.as_ref().unwrap();
    assert_eq!(
        local_policy.protected_branches,
        vec!["refs/heads/main".to_string()]
    );
    assert!(local_policy.require_clean_primary_checkout);
}

#[test]
fn load_contract_generated_hook_mode_requires_contract_semantics() {
    let repo = TempDir::new().unwrap();
    write_repo_file(
        &repo,
        ".repocert/config.toml",
        r#"
schema_version = 1

[hooks]
mode = "generated"
"#,
    );

    let error = load_contract(LoadOptions::from_repo_root(repo.path())).unwrap_err();

    match error.error {
        LoadError::Validation(errors) => {
            assert!(
                errors
                    .to_string()
                    .contains("requires protected_refs and/or local_policy")
            );
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn load_contract_repo_owned_hook_mode_returns_unsupported_mode_error() {
    let repo = TempDir::new().unwrap();
    write_repo_file(
        &repo,
        ".repocert/config.toml",
        r#"
schema_version = 1

[hooks]
mode = "repo-owned"
"#,
    );

    let error = load_contract(LoadOptions::from_repo_root(repo.path())).unwrap_err();

    match error.error {
        LoadError::Validation(errors) => {
            assert!(errors.to_string().contains("unsupported hook mode"));
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn load_contract_unknown_repo_owned_hook_table_returns_parse_error() {
    let repo = TempDir::new().unwrap();
    write_repo_file(
        &repo,
        ".repocert/config.toml",
        r#"
schema_version = 1

[hooks]
mode = "generated"

[hooks.repo_owned]
path = ".repocert/hooks"
"#,
    );

    let error = load_contract(LoadOptions::from_repo_root(repo.path())).unwrap_err();

    match error.error {
        LoadError::Parse(error) => {
            assert!(error.to_string().contains("repo_owned"));
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn load_contract_non_certifiable_protected_ref_returns_validation_error() {
    // Arrange
    let repo = TempDir::new().unwrap();
    write_repo_file(
        &repo,
        ".repocert/config.toml",
        r#"
schema_version = 1

[checks.test]
argv = ["cargo", "test"]

[profiles.dev]
checks = ["test"]

[[protected_refs]]
pattern = "refs/heads/main"
profile = "dev"
"#,
    );

    // Act
    let error = load_contract(LoadOptions::from_repo_root(repo.path())).unwrap_err();

    // Assert
    match error.error {
        LoadError::Validation(errors) => {
            assert!(
                errors
                    .to_string()
                    .contains("non-certification-eligible profile")
            );
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn load_contract_conflicting_hook_mode_tables_returns_validation_error() {
    // Arrange
    let repo = TempDir::new().unwrap();
    write_repo_file(
        &repo,
        ".repocert/config.toml",
        r#"
schema_version = 1

[hooks]
mode = "generated"

[hooks.repo_owned]
path = ".repocert/hooks"

[hooks.generated]
"#,
    );

    // Act
    let error = load_contract(LoadOptions::from_repo_root(repo.path())).unwrap_err();

    // Assert
    match error.error {
        LoadError::Parse(error) => {
            assert!(error.to_string().contains("repo_owned"));
        }
        other => panic!("unexpected error: {other:?}"),
    }
}
