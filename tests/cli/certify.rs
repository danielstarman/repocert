use std::path::Path;
use std::process::Command;

use repocert::certification::{CertificationKey, CertificationStore};
use serde_json::Value;
use tempfile::TempDir;

use crate::{commit_all, init_git_repo, write_repo_file};

fn repocert_bin() -> &'static str {
    env!("CARGO_BIN_EXE_repocert")
}

fn run_certify(args: &[&str], cwd: &Path) -> std::process::Output {
    let mut command = Command::new(repocert_bin());
    command.arg("certify");
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
fn certify_default_profile_passes_and_writes_record() {
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

[fixers.format]
argv = ["sh", "-c", "exit 0"]
probe_argv = ["sh", "-c", "exit 0"]

[profiles.default]
checks = ["test"]
fixers = ["format"]
certify = true
default = true
"#,
    );
    commit_all(&repo, "initial");

    // Act
    let output = run_certify(&["--format", "json"], repo.path());

    // Assert
    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["ok"], true);
    assert_eq!(json["profiles"], serde_json::json!(["default"]));
    assert_eq!(json["profile_results"][0]["outcome"], "certified");
    assert_eq!(json["profile_results"][0]["record_written"], true);

    let commit = head_commit(&repo);
    let store = CertificationStore::open(repo.path()).unwrap();
    let record = store
        .read(&CertificationKey {
            commit,
            profile: "default".to_string(),
        })
        .unwrap();
    assert!(record.is_some());
}

#[test]
fn certify_repair_needed_probe_fails_and_does_not_write_record() {
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

[fixers.format]
argv = ["sh", "-c", "exit 0"]
probe_argv = ["sh", "-c", "exit 1"]

[profiles.default]
checks = ["test"]
fixers = ["format"]
certify = true
default = true
"#,
    );
    commit_all(&repo, "initial");

    // Act
    let output = run_certify(&["--format", "json"], repo.path());

    // Assert
    assert_eq!(output.status.code(), Some(1));
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["ok"], false);
    assert_eq!(json["error"], Value::Null);
    assert_eq!(json["profile_results"][0]["outcome"], "repair_needed");
    assert_eq!(json["profile_results"][0]["record_written"], false);

    let commit = head_commit(&repo);
    let store = CertificationStore::open(repo.path()).unwrap();
    let record = store
        .read(&CertificationKey {
            commit,
            profile: "default".to_string(),
        })
        .unwrap();
    assert!(record.is_none());
}

#[test]
fn certify_dirty_untracked_file_fails_before_execution() {
    // Arrange
    let repo = TempDir::new().unwrap();
    init_git_repo(&repo);
    write_repo_file(
        &repo,
        ".repocert/config.toml",
        r#"
schema_version = 1

[checks.marker]
argv = ["sh", "-c", "touch marker.out"]

[profiles.default]
checks = ["marker"]
certify = true
default = true
"#,
    );
    commit_all(&repo, "initial");
    write_repo_file(&repo, "untracked.txt", "dirty\n");

    // Act
    let output = run_certify(&["--format", "json"], repo.path());

    // Assert
    assert_eq!(output.status.code(), Some(1));
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["error"]["category"], "worktree");
    assert!(json["dirty_paths"].as_array().unwrap().len() >= 1);
    assert!(!repo.path().join("marker.out").exists());
}

#[test]
fn certify_non_certifiable_default_profile_returns_selection_error() {
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

[profiles.default]
checks = ["test"]
default = true
"#,
    );
    commit_all(&repo, "initial");

    // Act
    let output = run_certify(&["--format", "json"], repo.path());

    // Assert
    assert_eq!(output.status.code(), Some(1));
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["error"]["category"], "selection");
    assert!(
        json["error"]["message"]
            .as_str()
            .unwrap()
            .contains("not certification-eligible")
    );
}

#[test]
fn certify_multiple_profiles_continue_after_failure_and_record_later_success() {
    // Arrange
    let repo = TempDir::new().unwrap();
    init_git_repo(&repo);
    write_repo_file(
        &repo,
        ".repocert/config.toml",
        r#"
schema_version = 1

[checks.fail]
argv = ["sh", "-c", "exit 2"]

[checks.pass]
argv = ["sh", "-c", "exit 0"]

[profiles.first]
checks = ["fail"]
certify = true

[profiles.second]
checks = ["pass"]
certify = true
"#,
    );
    commit_all(&repo, "initial");

    // Act
    let output = run_certify(
        &[
            "--profile",
            "first",
            "--profile",
            "second",
            "--format",
            "json",
        ],
        repo.path(),
    );

    // Assert
    assert_eq!(output.status.code(), Some(1));
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["profile_results"][0]["profile"], "first");
    assert_eq!(json["profile_results"][0]["outcome"], "failed");
    assert_eq!(json["profile_results"][0]["record_written"], false);
    assert_eq!(json["profile_results"][1]["profile"], "second");
    assert_eq!(json["profile_results"][1]["outcome"], "certified");
    assert_eq!(json["profile_results"][1]["record_written"], true);

    let commit = head_commit(&repo);
    let store = CertificationStore::open(repo.path()).unwrap();
    let first = store
        .read(&CertificationKey {
            commit: commit.clone(),
            profile: "first".to_string(),
        })
        .unwrap();
    let second = store
        .read(&CertificationKey {
            commit,
            profile: "second".to_string(),
        })
        .unwrap();
    assert!(first.is_none());
    assert!(second.is_some());
}

#[test]
fn certify_current_repo_default_profile_returns_selection_error() {
    // Arrange
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"));

    // Act
    let output = run_certify(
        &[
            "--repo-root",
            repo_root.to_str().unwrap(),
            "--format",
            "json",
        ],
        repo_root,
    );

    // Assert
    assert_eq!(output.status.code(), Some(1));
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["error"]["category"], "selection");
}
