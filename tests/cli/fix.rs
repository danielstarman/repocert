use std::fs;
use std::path::Path;
use std::process::Command;

use serde_json::Value;
use tempfile::TempDir;

use crate::{commit_all, init_git_repo, write_repo_file};

fn repocert_bin() -> &'static str {
    env!("CARGO_BIN_EXE_repocert")
}

fn run_fix(args: &[&str], cwd: &Path) -> std::process::Output {
    let mut command = Command::new(repocert_bin());
    command.arg("fix");
    command.args(args);
    command.current_dir(cwd);
    command.env_remove("NO_COLOR");
    command.output().unwrap()
}

#[test]
fn fix_named_fixer_without_probe_is_allowed() {
    // Arrange
    let repo = TempDir::new().unwrap();
    init_git_repo(&repo);
    write_repo_file(
        &repo,
        ".repocert/config.toml",
        r#"
schema_version = 1

[fixers.write_note]
argv = ["sh", "-c", "printf 'hello' > note.txt"]
"#,
    );
    commit_all(&repo, "initial");

    // Act
    let output = run_fix(&["--name", "write_note", "--format", "json"], repo.path());

    // Assert
    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["ok"], true);
    assert_eq!(json["selection_mode"], "fixers");
    assert_eq!(json["fixers"], serde_json::json!(["write_note"]));
    assert_eq!(json["results"][0]["outcome"], "pass");
    assert_eq!(
        fs::read_to_string(repo.path().join("note.txt")).unwrap(),
        "hello"
    );
}

#[test]
fn fix_profile_selected_fixer_without_probe_returns_validation_error() {
    // Arrange
    let repo = TempDir::new().unwrap();
    init_git_repo(&repo);
    write_repo_file(
        &repo,
        ".repocert/config.toml",
        r#"
schema_version = 1

[fixers.write_note]
argv = ["sh", "-c", "printf 'hello' > note.txt"]

[profiles.default]
fixers = ["write_note"]
"#,
    );
    commit_all(&repo, "initial");

    // Act
    let output = run_fix(&["--format", "json"], repo.path());

    // Assert
    assert_eq!(output.status.code(), Some(1));
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["error"]["category"], "validation");
    assert!(
        json["error"]["message"]
            .as_str()
            .unwrap()
            .contains("probe_argv")
    );
}

#[test]
fn fix_stops_on_first_failing_fixer() {
    // Arrange
    let repo = TempDir::new().unwrap();
    init_git_repo(&repo);
    write_repo_file(
        &repo,
        ".repocert/config.toml",
        r#"
schema_version = 1

[fixers.fail_first]
argv = ["sh", "-c", "printf 'x' > first.txt && exit 2"]

[fixers.never_run]
argv = ["sh", "-c", "printf 'y' > second.txt"]
"#,
    );
    commit_all(&repo, "initial");

    // Act
    let output = run_fix(
        &[
            "--name",
            "fail_first",
            "--name",
            "never_run",
            "--format",
            "json",
        ],
        repo.path(),
    );

    // Assert
    assert_eq!(output.status.code(), Some(1));
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["results"].as_array().unwrap().len(), 1);
    assert_eq!(json["results"][0]["name"], "fail_first");
    assert!(!repo.path().join("second.txt").exists());
}

#[test]
fn fix_timeout_is_classified_and_stops() {
    // Arrange
    let repo = TempDir::new().unwrap();
    init_git_repo(&repo);
    write_repo_file(
        &repo,
        ".repocert/config.toml",
        r#"
schema_version = 1

[fixers.slow]
argv = ["sh", "-c", "sleep 1"]
timeout_ms = 10
"#,
    );
    commit_all(&repo, "initial");

    // Act
    let output = run_fix(&["--name", "slow", "--format", "json"], repo.path());

    // Assert
    assert_eq!(output.status.code(), Some(1));
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["results"][0]["outcome"], "timeout");
    assert_eq!(json["summary"]["timeout"], 1);
}

#[test]
fn fix_protected_path_touch_fails_immediately() {
    // Arrange
    let repo = TempDir::new().unwrap();
    init_git_repo(&repo);
    write_repo_file(
        &repo,
        ".repocert/config.toml",
        r#"
schema_version = 1

[fixers.mutate_contract]
argv = ["sh", "-c", "printf '\\n# changed\\n' >> .repocert/config.toml"]
"#,
    );
    commit_all(&repo, "initial");

    // Act
    let output = run_fix(
        &["--name", "mutate_contract", "--format", "json"],
        repo.path(),
    );

    // Assert
    assert_eq!(output.status.code(), Some(1));
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["results"][0]["outcome"], "fail");
    assert!(
        json["results"][0]["message"]
            .as_str()
            .unwrap()
            .contains("protected contract path")
    );
}

#[test]
fn fix_dirty_protected_path_is_allowed_if_fixer_does_not_change_it() {
    // Arrange
    let repo = TempDir::new().unwrap();
    init_git_repo(&repo);
    write_repo_file(
        &repo,
        ".repocert/config.toml",
        r#"
schema_version = 1

[fixers.write_note]
argv = ["sh", "-c", "printf 'ok' > note.txt"]
"#,
    );
    commit_all(&repo, "initial");
    fs::write(
        repo.path().join(".repocert/config.toml"),
        r#"
schema_version = 1

[fixers.write_note]
argv = ["sh", "-c", "printf 'ok' > note.txt"]

# dirty
"#,
    )
    .unwrap();

    // Act
    let output = run_fix(&["--name", "write_note", "--format", "json"], repo.path());

    // Assert
    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["ok"], true);
    assert_eq!(json["results"][0]["outcome"], "pass");
}

#[test]
fn fix_current_repo_without_fixers_succeeds() {
    // Arrange
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"));

    // Act
    let output = run_fix(
        &[
            "--repo-root",
            repo_root.to_str().unwrap(),
            "--format",
            "json",
        ],
        repo_root,
    );

    // Assert
    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["ok"], true);
    assert_eq!(json["selection_mode"], "profile");
    assert_eq!(json["profile"], "default");
    assert_eq!(json["results"].as_array().unwrap().len(), 0);
}
