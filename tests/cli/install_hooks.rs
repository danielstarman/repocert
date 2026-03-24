use std::fs;
use std::path::Path;
use std::process::Command;

use serde_json::Value;
use tempfile::TempDir;

use crate::{commit_all, init_git_repo, write_repo_file};

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

fn read_hooks_path(cwd: &Path) -> Option<String> {
    let output = Command::new("git")
        .args(["config", "--get", "core.hooksPath"])
        .current_dir(cwd)
        .output()
        .unwrap();
    if output.status.success() {
        Some(String::from_utf8(output.stdout).unwrap().trim().to_string())
    } else {
        None
    }
}

fn read_local_hooks_path(cwd: &Path) -> Option<String> {
    let output = Command::new("git")
        .args(["config", "--local", "--get", "core.hooksPath"])
        .current_dir(cwd)
        .output()
        .unwrap();
    if output.status.success() {
        Some(String::from_utf8(output.stdout).unwrap().trim().to_string())
    } else {
        None
    }
}

fn run_git_output(cwd: &Path, args: &[&str]) -> std::process::Output {
    Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .unwrap()
}

fn add_linked_worktree(repo: &TempDir, branch: &str) -> std::path::PathBuf {
    let parent = TempDir::new().unwrap();
    let worktree = parent.path().join("linked");
    let output = run_git_output(
        repo.path(),
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            branch,
            worktree.to_str().unwrap(),
            "HEAD",
        ],
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    std::mem::forget(parent);
    worktree
}

fn generated_protected_ref_config() -> &'static str {
    r#"
schema_version = 1

[checks.git-status]
argv = ["git", "status", "--short"]

[profiles.release]
checks = ["git-status"]
certify = true

[[protected_refs]]
pattern = "refs/heads/main"
profile = "release"

[hooks]
mode = "generated"
"#
}

#[test]
fn install_hooks_generated_writes_wrappers_and_is_idempotent() {
    // Arrange
    let repo = TempDir::new().unwrap();
    init_git_repo(&repo);
    write_repo_file(
        &repo,
        ".repocert/config.toml",
        generated_protected_ref_config(),
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
    assert!(pre_push.contains("hook run"));
    assert!(pre_push.contains("pre-push"));
    assert!(pre_push.contains(repocert_bin()));
    assert_eq!(read_local_hooks_path(repo.path()), None);
    assert_eq!(
        read_hooks_path(repo.path()),
        first_json["hooks_path"].as_str().map(str::to_string)
    );
}

#[test]
fn install_hooks_generated_repairs_stale_wrapper_content() {
    // Arrange
    let repo = TempDir::new().unwrap();
    init_git_repo(&repo);
    write_repo_file(
        &repo,
        ".repocert/config.toml",
        generated_protected_ref_config(),
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
    assert!(update.contains("hook run"));
    assert!(update.contains("update"));
}

#[test]
fn install_hooks_generated_without_protected_refs_or_local_policy_errors() {
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
"#,
    );

    // Act
    let output = run_install_hooks(&["--format", "json"], repo.path());

    // Assert
    assert_eq!(output.status.code(), Some(1));
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["error"]["category"], "validation");
}

#[test]
fn install_hooks_repo_owned_mode_returns_validation_error() {
    let repo = TempDir::new().unwrap();
    init_git_repo(&repo);
    write_repo_file(
        &repo,
        ".repocert/config.toml",
        r#"
schema_version = 1

[hooks]
mode = "repo-owned"
"#,
    );

    let output = run_install_hooks(&["--format", "json"], repo.path());

    assert_eq!(output.status.code(), Some(1));
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["error"]["category"], "validation");
    assert!(
        json["error"]["message"]
            .as_str()
            .unwrap()
            .contains("unsupported hook mode")
    );
}

#[test]
fn install_hooks_generated_in_linked_worktree_does_not_hijack_primary_checkout() {
    let repo = TempDir::new().unwrap();
    init_git_repo(&repo);
    write_repo_file(
        &repo,
        ".repocert/config.toml",
        generated_protected_ref_config(),
    );
    write_repo_file(&repo, "README.md", "initial\n");
    commit_all(&repo, "initial");

    let primary_install = run_install_hooks(&["--format", "json"], repo.path());
    assert!(primary_install.status.success());
    let primary_json: Value = serde_json::from_slice(&primary_install.stdout).unwrap();
    let primary_hooks_path = primary_json["hooks_path"].as_str().unwrap().to_string();
    let primary_update = fs::read_to_string(Path::new(&primary_hooks_path).join("update")).unwrap();
    assert!(primary_update.contains(repo.path().to_str().unwrap()));

    let worktree = add_linked_worktree(&repo, "feature");
    let worktree_install = run_install_hooks(&["--format", "json"], &worktree);
    assert!(worktree_install.status.success());
    let worktree_json: Value = serde_json::from_slice(&worktree_install.stdout).unwrap();
    let worktree_hooks_path = worktree_json["hooks_path"].as_str().unwrap().to_string();
    let worktree_update =
        fs::read_to_string(Path::new(&worktree_hooks_path).join("update")).unwrap();

    assert_ne!(primary_hooks_path, worktree_hooks_path);
    assert_eq!(
        read_hooks_path(repo.path()).as_deref(),
        Some(primary_hooks_path.as_str())
    );
    assert_eq!(
        read_hooks_path(&worktree).as_deref(),
        Some(worktree_hooks_path.as_str())
    );
    assert!(worktree_update.contains(worktree.to_str().unwrap()));

    let primary_update_after =
        fs::read_to_string(Path::new(&primary_hooks_path).join("update")).unwrap();
    assert_eq!(primary_update_after, primary_update);
}
