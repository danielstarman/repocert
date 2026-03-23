use std::path::Path;
use std::process::Command;

use serde_json::Value;
use tempfile::TempDir;

use crate::{commit_all, init_git_repo, write_repo_file};

fn repocert_bin() -> &'static str {
    env!("CARGO_BIN_EXE_repocert")
}

fn run_authorize(args: &[&str], cwd: &Path) -> std::process::Output {
    let mut command = Command::new(repocert_bin());
    command.arg("authorize");
    command.args(args);
    command.current_dir(cwd);
    command.env_remove("NO_COLOR");
    command.output().unwrap()
}

fn head_commit(repo: &TempDir) -> String {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    String::from_utf8(output.stdout).unwrap().trim().to_string()
}

#[test]
fn authorize_no_matching_rule_returns_allowed() {
    // Arrange
    let repo = TempDir::new().unwrap();
    init_git_repo(&repo);
    write_repo_file(&repo, ".repocert/config.toml", "schema_version = 1\n");
    commit_all(&repo, "initial");
    let head = head_commit(&repo);

    // Act
    let output = run_authorize(
        &[
            "0000000000000000000000000000000000000000",
            &head,
            "refs/heads/main",
            "--format",
            "json",
        ],
        repo.path(),
    );

    // Assert
    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["error"], Value::Null);
    assert_eq!(json["allowed"], true);
    assert_eq!(json["matched_rules"], serde_json::json!([]));
}

#[test]
fn authorize_matching_rule_without_certification_denies() {
    // Arrange
    let repo = TempDir::new().unwrap();
    init_git_repo(&repo);
    write_repo_file(
        &repo,
        ".repocert/config.toml",
        r#"
schema_version = 1

[checks.test]
argv = ["sh", "-c", "exit 0"]

[profiles.release]
checks = ["test"]
certify = true
default = true

[[protected_refs]]
pattern = "refs/heads/main"
profile = "release"
"#,
    );
    commit_all(&repo, "initial");
    let head = head_commit(&repo);

    // Act
    let output = run_authorize(
        &[
            "1111111111111111111111111111111111111111",
            &head,
            "refs/heads/main",
            "--format",
            "json",
        ],
        repo.path(),
    );

    // Assert
    assert_eq!(output.status.code(), Some(1));
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["allowed"], false);
    assert_eq!(json["profile_results"][0]["state"], "uncertified");
}

#[test]
fn authorize_matching_rule_with_valid_certification_allows() {
    // Arrange
    let repo = TempDir::new().unwrap();
    init_git_repo(&repo);
    write_repo_file(
        &repo,
        ".repocert/config.toml",
        r#"
schema_version = 1

[checks.test]
argv = ["sh", "-c", "exit 0"]

[profiles.release]
checks = ["test"]
certify = true
default = true

[[protected_refs]]
pattern = "refs/heads/*"
profile = "release"
"#,
    );
    commit_all(&repo, "initial");
    let certify = Command::new(repocert_bin())
        .args(["certify", "--format", "json"])
        .current_dir(repo.path())
        .output()
        .unwrap();
    assert!(certify.status.success());
    let head = head_commit(&repo);

    // Act
    let output = run_authorize(
        &[
            "1111111111111111111111111111111111111111",
            &head,
            "refs/heads/main",
            "--format",
            "json",
        ],
        repo.path(),
    );

    // Assert
    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["error"], Value::Null);
    assert_eq!(json["allowed"], true);
    assert_eq!(json["matched_rules"][0]["pattern"], "refs/heads/*");
    assert_eq!(json["profile_results"][0]["state"], "certified");
}

#[test]
fn authorize_stale_fingerprint_denies() {
    // Arrange
    let repo = TempDir::new().unwrap();
    init_git_repo(&repo);
    write_repo_file(
        &repo,
        ".repocert/config.toml",
        r#"
schema_version = 1

[checks.test]
argv = ["sh", "-c", "exit 0"]

[profiles.release]
checks = ["test"]
certify = true
default = true

[[protected_refs]]
pattern = "refs/heads/main"
profile = "release"
"#,
    );
    commit_all(&repo, "initial");
    let certify = Command::new(repocert_bin())
        .args(["certify", "--format", "json"])
        .current_dir(repo.path())
        .output()
        .unwrap();
    assert!(certify.status.success());
    write_repo_file(
        &repo,
        ".repocert/config.toml",
        r#"
schema_version = 1

[checks.test]
argv = ["sh", "-c", "exit 0"]

[checks.extra]
argv = ["sh", "-c", "exit 0"]

[profiles.release]
checks = ["test"]
certify = true
default = true

[[protected_refs]]
pattern = "refs/heads/main"
profile = "release"
"#,
    );
    let head = head_commit(&repo);

    // Act
    let output = run_authorize(
        &[
            "1111111111111111111111111111111111111111",
            &head,
            "refs/heads/main",
            "--format",
            "json",
        ],
        repo.path(),
    );

    // Assert
    assert_eq!(output.status.code(), Some(1));
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["profile_results"][0]["state"], "stale_fingerprint");
}

#[test]
fn authorize_zero_new_returns_input_error() {
    // Arrange
    let repo = TempDir::new().unwrap();
    init_git_repo(&repo);
    write_repo_file(&repo, ".repocert/config.toml", "schema_version = 1\n");
    commit_all(&repo, "initial");

    // Act
    let output = run_authorize(
        &[
            "1111111111111111111111111111111111111111",
            "0000000000000000000000000000000000000000",
            "refs/heads/main",
            "--format",
            "json",
        ],
        repo.path(),
    );

    // Assert
    assert_eq!(output.status.code(), Some(1));
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["error"]["category"], "input");
}
