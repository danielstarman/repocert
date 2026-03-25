use std::path::{Path, PathBuf};
use std::process::Command;

use repocert::certification::{
    CertificationKey, CertificationPayload, CertificationRecord, CertificationStore,
    compute_contract_fingerprint,
};
use repocert::config::{LoadOptions, load_contract};
use serde_json::Value;
use tempfile::TempDir;

use crate::{commit_all, init_git_repo, write_repo_file};

fn repocert_bin() -> &'static str {
    env!("CARGO_BIN_EXE_repocert")
}

fn run_status(args: &[&str], cwd: &Path) -> std::process::Output {
    let mut command = Command::new(repocert_bin());
    command.arg("status");
    command.args(args);
    command.current_dir(cwd);
    command.env_remove("NO_COLOR");
    command.output().unwrap()
}

fn generate_ssh_signer() -> (TempDir, PathBuf, String) {
    let dir = TempDir::new().unwrap();
    let key_path = dir.path().join("signer");
    let output = Command::new("ssh-keygen")
        .args(["-q", "-t", "ed25519", "-N", "", "-f"])
        .arg(&key_path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let public_key_path = PathBuf::from(format!("{}.pub", key_path.display()));
    let public_key = std::fs::read_to_string(&public_key_path).unwrap();
    (dir, public_key_path, public_key.trim().to_string())
}

#[test]
fn status_current_repo_reports_default_profile_and_main_protection() {
    // Arrange
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"));

    // Act
    let output = run_status(
        &[
            "--repo-root",
            repo_root.to_str().unwrap(),
            "--format",
            "json",
        ],
        repo_root,
    );

    // Assert
    assert_eq!(output.status.code(), Some(0));
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["ok"], true);
    assert_eq!(json["error"], Value::Null);
    assert_eq!(json["profiles"], serde_json::json!(["default", "release"]));
    assert_eq!(json["protected_refs"][0]["pattern"], "refs/heads/main");
    assert_eq!(json["protected_refs"][0]["profile"], "default");
    assert_eq!(json["protected_refs"][1]["pattern"], "refs/heads/release/*");
    assert_eq!(json["protected_refs"][1]["profile"], "release");
    assert_eq!(json["protected_refs"][2]["pattern"], "refs/tags/v*");
    assert_eq!(json["protected_refs"][2]["profile"], "release");
}

#[test]
fn status_certified_profile_returns_certified_state() {
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
certify = true
default = true
"#,
    );
    commit_all(&repo, "initial");
    let certify = Command::new(repocert_bin())
        .arg("certify")
        .arg("--format")
        .arg("json")
        .current_dir(repo.path())
        .output()
        .unwrap();
    assert!(certify.status.success());

    // Act
    let output = run_status(&["--format", "json"], repo.path());

    // Assert
    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["ok"], true);
    assert_eq!(json["error"], Value::Null);
    assert_eq!(json["profile_results"][0]["state"], "certified");
}

#[test]
fn status_changed_contract_returns_stale_fingerprint() {
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
certify = true
default = true
"#,
    );
    commit_all(&repo, "initial");
    let certify = Command::new(repocert_bin())
        .arg("certify")
        .arg("--format")
        .arg("json")
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

[profiles.default]
checks = ["test"]
certify = true
default = true
"#,
    );

    // Act
    let output = run_status(&["--format", "json"], repo.path());

    // Assert
    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["profile_results"][0]["state"], "stale_fingerprint");
}

#[test]
fn status_other_commit_record_returns_stale_commit() {
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
certify = true
default = true
"#,
    );
    commit_all(&repo, "initial");
    let certify = Command::new(repocert_bin())
        .arg("certify")
        .arg("--format")
        .arg("json")
        .current_dir(repo.path())
        .output()
        .unwrap();
    assert!(certify.status.success());
    write_repo_file(&repo, "README.md", "next\n");
    commit_all(&repo, "next");

    // Act
    let output = run_status(&["--format", "json"], repo.path());

    // Assert
    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["profile_results"][0]["state"], "stale_commit");
    assert_eq!(
        json["profile_results"][0]["other_certified_commits"],
        serde_json::json!([head_commit_previous(&repo)])
    );
}

#[test]
fn status_assert_certified_returns_failure_for_uncertified_profile() {
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
certify = true
default = true
"#,
    );
    commit_all(&repo, "initial");

    // Act
    let output = run_status(&["--format", "json", "--assert-certified"], repo.path());

    // Assert
    assert_eq!(output.status.code(), Some(1));
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["ok"], false);
    assert_eq!(json["profile_results"][0]["state"], "uncertified");
}

#[test]
fn status_protected_refs_report_certification_state() {
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
        .arg("certify")
        .arg("--format")
        .arg("json")
        .current_dir(repo.path())
        .output()
        .unwrap();
    assert!(certify.status.success());

    // Act
    let output = run_status(&["--format", "json"], repo.path());

    // Assert
    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["protected_refs"][0]["pattern"], "refs/heads/main");
    assert_eq!(json["protected_refs"][0]["profile"], "release");
    assert_eq!(json["protected_refs"][0]["certified"], true);
}

#[test]
fn status_signed_mode_surfaces_legacy_unsigned_records_explicitly() {
    let repo = TempDir::new().unwrap();
    let (_key_dir, _public_key_path, public_key) = generate_ssh_signer();
    init_git_repo(&repo);
    write_repo_file(
        &repo,
        ".repocert/config.toml",
        &format!(
            r#"
schema_version = 1

[checks.test]
argv = ["sh", "-c", "exit 0"]

[profiles.default]
checks = ["test"]
certify = true
default = true

[certification]
mode = "ssh-signed"
trusted_signers = ["{public_key}"]
"#
        ),
    );
    commit_all(&repo, "initial");

    let loaded = load_contract(LoadOptions::from_repo_root(repo.path())).unwrap();
    let fingerprint = compute_contract_fingerprint(&loaded).unwrap();
    let commit = head_commit(&repo);
    let store = CertificationStore::open(repo.path()).unwrap();
    store
        .write(&CertificationRecord::Legacy(CertificationPayload {
            key: CertificationKey {
                commit,
                profile: "default".to_string(),
            },
            contract_fingerprint: fingerprint,
        }))
        .unwrap();

    let output = run_status(&["--format", "json"], repo.path());

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["profile_results"][0]["state"], "legacy_unsigned");
    assert_eq!(json["summary"]["legacy_unsigned"], 1);
}

fn head_commit_previous(repo: &TempDir) -> String {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD^"])
        .current_dir(repo.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    String::from_utf8(output.stdout).unwrap().trim().to_string()
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
