use std::path::Path;
use std::process::Command;

use serde_json::Value;
use tempfile::TempDir;

use crate::write_repo_file;

fn repocert_bin() -> &'static str {
    env!("CARGO_BIN_EXE_repocert")
}

fn run_check(args: &[&str], cwd: &Path) -> std::process::Output {
    let mut command = Command::new(repocert_bin());
    command.arg("check");
    command.args(args);
    command.current_dir(cwd);
    command.env_remove("NO_COLOR");
    command.output().unwrap()
}

#[test]
fn check_default_profile_runs_checks_and_probes_in_deterministic_order() {
    // Arrange
    let repo = TempDir::new().unwrap();
    write_repo_file(
        &repo,
        ".repocert/config.toml",
        r#"
schema_version = 1

[checks.alpha]
argv = ["sh", "-c", "exit 0"]

[checks.beta]
argv = ["sh", "-c", "exit 0"]

[fixers.format]
argv = ["sh", "-c", "exit 0"]
probe_argv = ["sh", "-c", "exit 0"]

[profiles.base]
checks = ["alpha"]
fixers = ["format"]
default = true

[profiles.extended]
includes = ["base"]
checks = ["beta"]
"#,
    );

    // Act
    let output = run_check(&["--format", "json"], repo.path());

    // Assert
    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["ok"], true);
    assert_eq!(json["selection_mode"], "profiles");
    assert_eq!(json["profiles"], serde_json::json!(["base"]));
    assert_eq!(json["checks"], serde_json::json!(["alpha"]));
    assert_eq!(json["results"][0]["name"], "alpha");
    assert_eq!(json["results"][0]["kind"], "check");
    assert_eq!(json["results"][1]["name"], "format");
    assert_eq!(json["results"][1]["kind"], "fixer_probe");
}

#[test]
fn check_name_runs_direct_named_checks_without_probes() {
    // Arrange
    let repo = TempDir::new().unwrap();
    write_repo_file(
        &repo,
        ".repocert/config.toml",
        r#"
schema_version = 1

[checks.alpha]
argv = ["sh", "-c", "exit 0"]

[checks.beta]
argv = ["sh", "-c", "exit 0"]

[fixers.format]
argv = ["sh", "-c", "exit 0"]
probe_argv = ["sh", "-c", "exit 1"]

[profiles.default]
checks = ["alpha"]
fixers = ["format"]
default = true
"#,
    );

    // Act
    let output = run_check(&["--name", "beta", "--format", "json"], repo.path());

    // Assert
    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["selection_mode"], "checks");
    assert_eq!(json["checks"], serde_json::json!(["beta"]));
    assert_eq!(json["results"].as_array().unwrap().len(), 1);
    assert_eq!(json["results"][0]["name"], "beta");
    assert_eq!(json["results"][0]["kind"], "check");
}

#[test]
fn check_without_default_profile_returns_selection_error() {
    // Arrange
    let repo = TempDir::new().unwrap();
    write_repo_file(
        &repo,
        ".repocert/config.toml",
        r#"
schema_version = 1

[checks.alpha]
argv = ["sh", "-c", "exit 0"]

[profiles.first]
checks = ["alpha"]

[profiles.second]
checks = ["alpha"]
"#,
    );

    // Act
    let output = run_check(&["--format", "json"], repo.path());

    // Assert
    assert_eq!(output.status.code(), Some(1));
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["error"]["category"], "selection");
    assert!(
        json["error"]["message"]
            .as_str()
            .unwrap()
            .contains("default profile")
    );
}

#[test]
fn check_timeout_is_classified_distinctly() {
    // Arrange
    let repo = TempDir::new().unwrap();
    write_repo_file(
        &repo,
        ".repocert/config.toml",
        r#"
schema_version = 1

[checks.slow]
argv = ["sh", "-c", "sleep 1"]
timeout_ms = 10

[profiles.default]
checks = ["slow"]
default = true
"#,
    );

    // Act
    let output = run_check(&["--format", "json"], repo.path());

    // Assert
    assert_eq!(output.status.code(), Some(1));
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["results"][0]["outcome"], "timeout");
    assert_eq!(json["summary"]["timeout"], 1);
}

#[test]
fn check_probe_exit_one_is_repair_needed_and_does_not_fail_fast() {
    // Arrange
    let repo = TempDir::new().unwrap();
    write_repo_file(
        &repo,
        ".repocert/config.toml",
        r#"
schema_version = 1

[checks.first]
argv = ["sh", "-c", "exit 2"]

[checks.second]
argv = ["sh", "-c", "exit 0"]

[fixers.format]
argv = ["sh", "-c", "exit 0"]
probe_argv = ["sh", "-c", "exit 1"]

[profiles.default]
checks = ["first", "second"]
fixers = ["format"]
default = true
"#,
    );

    // Act
    let output = run_check(&["--format", "json"], repo.path());

    // Assert
    assert_eq!(output.status.code(), Some(1));
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["results"].as_array().unwrap().len(), 3);
    assert_eq!(json["results"][0]["outcome"], "fail");
    assert_eq!(json["results"][1]["outcome"], "pass");
    assert_eq!(json["results"][2]["outcome"], "repair_needed");
    assert_eq!(json["summary"]["fail"], 1);
    assert_eq!(json["summary"]["repair_needed"], 1);
}

#[test]
fn check_missing_tool_is_reported_as_failed_item_not_config_error() {
    // Arrange
    let repo = TempDir::new().unwrap();
    write_repo_file(
        &repo,
        ".repocert/config.toml",
        r#"
schema_version = 1

[checks.missing]
argv = ["definitely-not-a-real-command"]

[profiles.default]
checks = ["missing"]
default = true
"#,
    );

    // Act
    let output = run_check(&["--format", "json"], repo.path());

    // Assert
    assert_eq!(output.status.code(), Some(1));
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["error"], Value::Null);
    assert_eq!(json["results"][0]["outcome"], "fail");
    assert!(
        json["results"][0]["message"]
            .as_str()
            .unwrap()
            .contains("failed to spawn command")
    );
}

#[test]
fn check_current_repo_named_fmt_returns_success() {
    // Arrange
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"));

    // Act
    let output = run_check(
        &[
            "--repo-root",
            repo_root.to_str().unwrap(),
            "--name",
            "fmt",
            "--format",
            "json",
        ],
        repo_root,
    );

    // Assert
    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["ok"], true);
    assert_eq!(json["selection_mode"], "checks");
    assert_eq!(json["checks"], serde_json::json!(["fmt"]));
    assert_eq!(json["results"][0]["name"], "fmt");
    assert_eq!(json["results"][0]["outcome"], "pass");
}
