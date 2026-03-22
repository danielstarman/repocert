use std::path::Path;
use std::process::Command;

use serde_json::Value;
use tempfile::TempDir;

use crate::write_repo_file;

fn repocert_bin() -> &'static str {
    env!("CARGO_BIN_EXE_repocert")
}

fn run_validate(args: &[&str], cwd: &Path) -> std::process::Output {
    let mut command = Command::new(repocert_bin());
    command.arg("validate");
    command.args(args);
    command.current_dir(cwd);
    command.env_remove("NO_COLOR");
    command.output().unwrap()
}

#[test]
fn validate_walkup_human_success_returns_pass_and_paths() {
    // Arrange
    let repo = TempDir::new().unwrap();
    write_repo_file(&repo, ".repocert/config.toml", "schema_version = 1");
    std::fs::create_dir_all(repo.path().join("nested/work")).unwrap();

    // Act
    let output = run_validate(&[], &repo.path().join("nested/work"));

    // Assert
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("PASS validate"));
    assert!(stdout.contains("repo_root:"));
    assert!(stdout.contains("config_path:"));
    assert!(String::from_utf8(output.stderr).unwrap().is_empty());
}

#[test]
fn validate_repo_root_json_success_returns_resolved_paths() {
    // Arrange
    let repo = TempDir::new().unwrap();
    write_repo_file(&repo, ".repocert/config.toml", "schema_version = 1");

    // Act
    let output = run_validate(
        &[
            "--repo-root",
            repo.path().to_str().unwrap(),
            "--format",
            "json",
        ],
        repo.path(),
    );

    // Assert
    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["ok"], true);
    assert_eq!(json["command"], "validate");
    assert_eq!(
        json["repo_root"],
        repo.path().canonicalize().unwrap().display().to_string()
    );
    assert_eq!(
        json["config_path"],
        repo.path()
            .join(".repocert/config.toml")
            .canonicalize()
            .unwrap()
            .display()
            .to_string()
    );
}

#[test]
fn validate_config_path_parse_failure_returns_json_error() {
    // Arrange
    let repo = TempDir::new().unwrap();
    write_repo_file(&repo, ".repocert/config.toml", "schema_version = [");

    // Act
    let output = run_validate(
        &[
            "--config-path",
            repo.path().join(".repocert/config.toml").to_str().unwrap(),
            "--format",
            "json",
        ],
        repo.path(),
    );

    // Assert
    assert_eq!(output.status.code(), Some(1));
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["ok"], false);
    assert_eq!(json["command"], "validate");
    assert_eq!(json["error"]["category"], "parse");
    assert!(
        json["error"]["message"]
            .as_str()
            .unwrap()
            .contains("could not parse TOML config")
    );
    assert_eq!(
        json["config_path"],
        repo.path()
            .join(".repocert/config.toml")
            .canonicalize()
            .unwrap()
            .display()
            .to_string()
    );
}

#[test]
fn validate_invalid_schema_returns_human_failure_output() {
    // Arrange
    let repo = TempDir::new().unwrap();
    write_repo_file(&repo, ".repocert/config.toml", "schema_version = 2");

    // Act
    let output = run_validate(&[], repo.path());

    // Assert
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("FAIL validate [validation]"));
    assert!(stderr.contains("schema_version"));
}

#[test]
fn validate_invalid_schema_json_failure_returns_resolved_paths() {
    // Arrange
    let repo = TempDir::new().unwrap();
    write_repo_file(&repo, ".repocert/config.toml", "schema_version = 2");

    // Act
    let output = run_validate(&["--format", "json"], repo.path());

    // Assert
    assert_eq!(output.status.code(), Some(1));
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["ok"], false);
    assert_eq!(json["error"]["category"], "validation");
    assert_eq!(
        json["repo_root"],
        repo.path().canonicalize().unwrap().display().to_string()
    );
    assert_eq!(
        json["config_path"],
        repo.path()
            .join(".repocert/config.toml")
            .canonicalize()
            .unwrap()
            .display()
            .to_string()
    );
}

#[test]
fn validate_does_not_execute_declared_commands() {
    // Arrange
    let repo = TempDir::new().unwrap();
    write_repo_file(
        &repo,
        ".repocert/config.toml",
        r#"
schema_version = 1

[checks.nonexistent]
argv = ["definitely-not-a-real-command"]
"#,
    );

    // Act
    let output = run_validate(&[], repo.path());

    // Assert
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("PASS validate"));
}

#[test]
fn validate_current_repo_contract_returns_success() {
    // Arrange
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"));

    // Act
    let output = run_validate(&["--repo-root", repo_root.to_str().unwrap()], repo_root);

    // Assert
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("PASS validate"));
    assert!(stdout.contains(&format!("repo_root: {}", repo_root.display())));
    assert!(stdout.contains(&format!(
        "config_path: {}",
        repo_root.join(".repocert/config.toml").display()
    )));
}
