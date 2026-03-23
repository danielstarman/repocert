use std::fs;
use std::path::Path;
use std::process::Command;

use serde_json::Value;
use tempfile::TempDir;

use crate::{init_git_repo, write_repo_file};

fn repocert_bin() -> &'static str {
    env!("CARGO_BIN_EXE_repocert")
}

fn run_install_hooks(args: &[&str], cwd: &Path) -> std::process::Output {
    let mut command = Command::new(repocert_bin());
    command.arg("install-hooks");
    command.args(args);
    command.current_dir(cwd);
    command.env_remove("NO_COLOR");
    command.output().unwrap()
}

fn read_hooks_path(repo: &TempDir) -> Option<String> {
    let output = Command::new("git")
        .args(["config", "--local", "--get", "core.hooksPath"])
        .current_dir(repo.path())
        .output()
        .unwrap();
    if output.status.success() {
        Some(String::from_utf8(output.stdout).unwrap().trim().to_string())
    } else {
        None
    }
}

#[test]
fn install_hooks_repo_owned_sets_core_hooks_path_and_is_idempotent() {
    // Arrange
    let repo = TempDir::new().unwrap();
    init_git_repo(&repo);
    write_repo_file(
        &repo,
        ".repocert/config.toml",
        r#"
schema_version = 1

[checks.missing]
argv = ["definitely-not-a-real-command"]

[profiles.default]
checks = ["missing"]

[hooks]
mode = "repo-owned"

[hooks.repo_owned]
path = ".repocert/hooks"
"#,
    );
    write_repo_file(&repo, ".repocert/hooks/pre-push", "#!/bin/sh\nexit 0\n");

    // Act
    let first = run_install_hooks(&["--format", "json"], repo.path());
    let second = run_install_hooks(&["--format", "json"], repo.path());

    // Assert
    assert!(first.status.success());
    assert!(second.status.success());
    let first_json: Value = serde_json::from_slice(&first.stdout).unwrap();
    let second_json: Value = serde_json::from_slice(&second.stdout).unwrap();
    assert_eq!(first_json["error"], Value::Null);
    assert_eq!(second_json["error"], Value::Null);
    assert_eq!(first_json["mode"], "repo-owned");
    assert_eq!(second_json["changed"], false);
    assert_eq!(
        read_hooks_path(&repo).unwrap(),
        repo.path()
            .join(".repocert/hooks")
            .canonicalize()
            .unwrap()
            .display()
            .to_string()
    );
}

#[test]
fn install_hooks_generated_writes_wrappers_and_is_idempotent() {
    // Arrange
    let repo = TempDir::new().unwrap();
    init_git_repo(&repo);
    write_repo_file(
        &repo,
        ".repocert/config.toml",
        r#"
schema_version = 1

[hooks]
mode = "generated"

[hooks.generated]
hooks = ["pre-push", "update"]
"#,
    );

    // Act
    let first = run_install_hooks(&["--format", "json"], repo.path());
    let second = run_install_hooks(&["--format", "json"], repo.path());

    // Assert
    assert!(first.status.success());
    assert!(second.status.success());
    let first_json: Value = serde_json::from_slice(&first.stdout).unwrap();
    let second_json: Value = serde_json::from_slice(&second.stdout).unwrap();
    assert_eq!(first_json["error"], Value::Null);
    assert_eq!(second_json["error"], Value::Null);
    assert_eq!(first_json["mode"], "generated");
    assert_eq!(second_json["changed"], false);

    let hooks_path = Path::new(first_json["hooks_path"].as_str().unwrap());
    assert!(hooks_path.join("pre-push").exists());
    assert!(hooks_path.join("update").exists());
    let pre_push = fs::read_to_string(hooks_path.join("pre-push")).unwrap();
    assert!(pre_push.contains("authorize"));
    assert!(pre_push.contains(repocert_bin()));
}

#[test]
fn install_hooks_generated_repairs_stale_wrapper_content() {
    // Arrange
    let repo = TempDir::new().unwrap();
    init_git_repo(&repo);
    write_repo_file(
        &repo,
        ".repocert/config.toml",
        r#"
schema_version = 1

[hooks]
mode = "generated"

[hooks.generated]
hooks = ["update"]
"#,
    );
    let first = run_install_hooks(&["--format", "json"], repo.path());
    assert!(first.status.success());
    let first_json: Value = serde_json::from_slice(&first.stdout).unwrap();
    let hooks_path = Path::new(first_json["hooks_path"].as_str().unwrap());
    fs::write(hooks_path.join("update"), "#!/bin/sh\nexit 1\n").unwrap();

    // Act
    let repaired = run_install_hooks(&["--format", "json"], repo.path());

    // Assert
    assert!(repaired.status.success());
    let repaired_json: Value = serde_json::from_slice(&repaired.stdout).unwrap();
    assert_eq!(repaired_json["changed"], true);
    let update = fs::read_to_string(hooks_path.join("update")).unwrap();
    assert!(update.contains("authorize"));
}

#[test]
fn install_hooks_generated_unsupported_hook_name_errors() {
    // Arrange
    let repo = TempDir::new().unwrap();
    init_git_repo(&repo);
    write_repo_file(
        &repo,
        ".repocert/config.toml",
        r#"
schema_version = 1

[hooks]
mode = "generated"

[hooks.generated]
hooks = ["pre-receive"]
"#,
    );

    // Act
    let output = run_install_hooks(&["--format", "json"], repo.path());

    // Assert
    assert_eq!(output.status.code(), Some(1));
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["error"]["category"], "hooks");
}

#[test]
fn install_hooks_repo_owned_missing_directory_errors() {
    // Arrange
    let repo = TempDir::new().unwrap();
    init_git_repo(&repo);
    write_repo_file(
        &repo,
        ".repocert/config.toml",
        r#"
schema_version = 1

[hooks]
mode = "repo-owned"

[hooks.repo_owned]
path = ".repocert/hooks"
"#,
    );

    // Act
    let output = run_install_hooks(&["--format", "json"], repo.path());

    // Assert
    assert_eq!(output.status.code(), Some(1));
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["error"]["category"], "hooks");
}
