use std::fs;

use repocert::config::{HookMode, LoadError, LoadOptions, load_contract};
use tempfile::TempDir;

#[cfg(unix)]
use std::os::unix::fs as unix_fs;

fn write_repo_file(repo: &TempDir, relative_path: &str, contents: &str) {
    let path = repo.path().join(relative_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, contents).unwrap();
}

#[test]
fn loads_and_validates_a_contract_from_discovery() {
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
mode = "repo-owned"

[hooks.repo_owned]
path = ".repocert/hooks"
"#,
    );
    fs::create_dir_all(repo.path().join("nested/work")).unwrap();

    let loaded =
        load_contract(LoadOptions::discover_from(repo.path().join("nested/work"))).unwrap();

    assert_eq!(loaded.repo_root, repo.path().canonicalize().unwrap());
    assert_eq!(
        loaded.config_path,
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
    match &loaded.contract.hooks.as_ref().unwrap().mode {
        HookMode::RepoOwned { path } => assert_eq!(path.as_str(), ".repocert/hooks"),
        other => panic!("unexpected hook mode: {other:?}"),
    }
}

#[test]
fn explicit_repo_root_requires_default_config_path() {
    let repo = TempDir::new().unwrap();

    let error = load_contract(LoadOptions::from_repo_root(repo.path())).unwrap_err();

    match error {
        LoadError::Discovery(error) => {
            let message = error.to_string();
            assert!(message.contains(".repocert/config.toml"));
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn explicit_repo_root_and_config_path_must_match() {
    let repo = TempDir::new().unwrap();
    let other = TempDir::new().unwrap();
    write_repo_file(&repo, ".repocert/config.toml", "schema_version = 1");
    write_repo_file(&other, ".repocert/config.toml", "schema_version = 1");

    let error = load_contract(LoadOptions {
        start_dir: None,
        repo_root: Some(repo.path().to_path_buf()),
        config_path: Some(other.path().join(".repocert/config.toml")),
    })
    .unwrap_err();

    match error {
        LoadError::Discovery(error) => {
            assert!(error.to_string().contains("do not match"));
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[cfg(unix)]
#[test]
fn canonicalizes_config_path_consistently_across_resolution_modes() {
    let repo = TempDir::new().unwrap();
    write_repo_file(&repo, ".repocert/real-config.toml", "schema_version = 1");
    fs::create_dir_all(repo.path().join("nested/work")).unwrap();
    unix_fs::symlink(
        repo.path().join(".repocert/real-config.toml"),
        repo.path().join(".repocert/config.toml"),
    )
    .unwrap();

    let from_repo_root = load_contract(LoadOptions::from_repo_root(repo.path())).unwrap();
    let from_config_path = load_contract(LoadOptions::from_config_path(
        repo.path().join(".repocert/config.toml"),
    ))
    .unwrap();
    let from_discovery =
        load_contract(LoadOptions::discover_from(repo.path().join("nested/work"))).unwrap();

    let canonical_target = repo
        .path()
        .join(".repocert/real-config.toml")
        .canonicalize()
        .unwrap();
    assert_eq!(from_repo_root.config_path, canonical_target);
    assert_eq!(from_config_path.config_path, canonical_target);
    assert_eq!(from_discovery.config_path, canonical_target);
}

#[test]
fn rejects_invalid_schema_version() {
    let repo = TempDir::new().unwrap();
    write_repo_file(&repo, ".repocert/config.toml", "schema_version = 2");

    let error = load_contract(LoadOptions::from_repo_root(repo.path())).unwrap_err();

    match error {
        LoadError::Validation(errors) => {
            assert!(errors.to_string().contains("schema_version"));
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn rejects_profile_include_cycles() {
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

    let error = load_contract(LoadOptions::from_repo_root(repo.path())).unwrap_err();

    match error {
        LoadError::Validation(errors) => {
            assert!(errors.to_string().contains("profile include cycle"))
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn rejects_profile_fixers_without_probe_commands() {
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

    let error = load_contract(LoadOptions::from_repo_root(repo.path())).unwrap_err();

    match error {
        LoadError::Validation(errors) => assert!(errors.to_string().contains("probe_argv")),
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn rejects_protected_paths_that_escape_repo_root() {
    let repo = TempDir::new().unwrap();
    write_repo_file(
        &repo,
        ".repocert/config.toml",
        r#"
schema_version = 1
protected_paths = ["../outside.txt"]
"#,
    );

    let error = load_contract(LoadOptions::from_repo_root(repo.path())).unwrap_err();

    match error {
        LoadError::Validation(errors) => assert!(errors.to_string().contains("escape")),
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn rejects_protected_refs_to_non_certifiable_profiles() {
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

    let error = load_contract(LoadOptions::from_repo_root(repo.path())).unwrap_err();

    match error {
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
fn rejects_conflicting_hook_mode_configuration() {
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
hooks = ["pre-push"]
"#,
    );

    let error = load_contract(LoadOptions::from_repo_root(repo.path())).unwrap_err();

    match error {
        LoadError::Validation(errors) => {
            assert!(
                errors
                    .to_string()
                    .contains("repo-owned hook configuration is not allowed")
            );
        }
        other => panic!("unexpected error: {other:?}"),
    }
}
