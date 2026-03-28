use std::path::{Path, PathBuf};
use std::process::Command;

use repocert::certification::{CertificationKey, CertificationPayload, CertificationStore};
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

fn certification_block(public_key: &str) -> String {
    format!(
        r#"
[certification]
mode = "ssh-signed"

[[certification.trusted_signer]]
name = "test"
public_key = "{public_key}"
"#
    )
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

[profiles.release]
checks = ["test"]
certify = true
default = true

[[protected_refs]]
pattern = "refs/heads/main"
profile = "release"
{}
"#,
            certification_block(&public_key)
        ),
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
    let (_key_dir, public_key_path, public_key) = generate_ssh_signer();
    init_git_repo(&repo);
    write_repo_file(
        &repo,
        ".repocert/config.toml",
        &format!(
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
{}
"#,
            certification_block(&public_key)
        ),
    );
    commit_all(&repo, "initial");
    let certify = Command::new(repocert_bin())
        .args([
            "certify",
            "--format",
            "json",
            "--signing-key",
            public_key_path.to_str().unwrap(),
        ])
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
fn authorize_legacy_unsigned_record_is_treated_as_uncertified() {
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

[profiles.release]
checks = ["test"]
certify = true
default = true

[[protected_refs]]
pattern = "refs/heads/main"
profile = "release"

[certification]
mode = "ssh-signed"

[[certification.trusted_signer]]
name = "test"
public_key = "{public_key}"
"#
        ),
    );
    commit_all(&repo, "initial");
    let head = head_commit(&repo);

    let store = CertificationStore::open(repo.path()).unwrap();
    let path = store.root_dir().join(&head).join("72656c65617365.json");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(
        &path,
        serde_json::to_vec_pretty(&CertificationPayload {
            key: CertificationKey {
                commit: head.clone(),
                profile: "release".to_string(),
            },
            contract_fingerprint: repocert::certification::ContractFingerprint::from_bytes([7; 32]),
        })
        .unwrap(),
    )
    .unwrap();

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

    assert_eq!(output.status.code(), Some(1));
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["ok"], false);
    assert_eq!(json["error"], Value::Null);
    assert_eq!(json["allowed"], false);
    assert_eq!(json["profile_results"][0]["state"], "uncertified");
}

#[test]
fn authorize_release_branch_requires_release_profile() {
    let repo = TempDir::new().unwrap();
    let (_key_dir, public_key_path, public_key) = generate_ssh_signer();
    init_git_repo(&repo);
    write_repo_file(
        &repo,
        ".repocert/config.toml",
        &format!(
            r#"
schema_version = 1

[checks.fast]
argv = ["sh", "-c", "exit 0"]

[checks.docs]
argv = ["sh", "-c", "exit 0"]

[profiles.default]
checks = ["fast"]
certify = true
default = true

[profiles.release]
includes = ["default"]
checks = ["docs"]
certify = true

[[protected_refs]]
pattern = "refs/heads/main"
profile = "default"

[[protected_refs]]
pattern = "refs/heads/release/*"
profile = "release"

[[protected_refs]]
pattern = "refs/tags/v*"
profile = "release"
{}
"#,
            certification_block(&public_key)
        ),
    );
    commit_all(&repo, "initial");
    let certify = Command::new(repocert_bin())
        .args([
            "certify",
            "--format",
            "json",
            "--signing-key",
            public_key_path.to_str().unwrap(),
        ])
        .current_dir(repo.path())
        .output()
        .unwrap();
    assert!(certify.status.success());
    let head = head_commit(&repo);

    let output = run_authorize(
        &[
            "1111111111111111111111111111111111111111",
            &head,
            "refs/heads/release/0.3",
            "--format",
            "json",
        ],
        repo.path(),
    );

    assert_eq!(output.status.code(), Some(1));
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["allowed"], false);
    assert_eq!(json["required_profiles"], serde_json::json!(["release"]));
    assert_eq!(json["profile_results"][0]["state"], "uncertified");
}

#[test]
fn authorize_release_tag_requires_release_profile() {
    let repo = TempDir::new().unwrap();
    let (_key_dir, public_key_path, public_key) = generate_ssh_signer();
    init_git_repo(&repo);
    write_repo_file(
        &repo,
        ".repocert/config.toml",
        &format!(
            r#"
schema_version = 1

[checks.fast]
argv = ["sh", "-c", "exit 0"]

[checks.docs]
argv = ["sh", "-c", "exit 0"]

[profiles.default]
checks = ["fast"]
certify = true
default = true

[profiles.release]
includes = ["default"]
checks = ["docs"]
certify = true

[[protected_refs]]
pattern = "refs/heads/main"
profile = "default"

[[protected_refs]]
pattern = "refs/heads/release/*"
profile = "release"

[[protected_refs]]
pattern = "refs/tags/v*"
profile = "release"
{}
"#,
            certification_block(&public_key)
        ),
    );
    commit_all(&repo, "initial");
    let certify = Command::new(repocert_bin())
        .args([
            "certify",
            "--format",
            "json",
            "--signing-key",
            public_key_path.to_str().unwrap(),
        ])
        .current_dir(repo.path())
        .output()
        .unwrap();
    assert!(certify.status.success());
    let head = head_commit(&repo);

    let output = run_authorize(
        &[
            "1111111111111111111111111111111111111111",
            &head,
            "refs/tags/v0.3.0",
            "--format",
            "json",
        ],
        repo.path(),
    );

    assert_eq!(output.status.code(), Some(1));
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["allowed"], false);
    assert_eq!(json["required_profiles"], serde_json::json!(["release"]));
    assert_eq!(json["profile_results"][0]["state"], "uncertified");
}

#[test]
fn authorize_signed_mode_with_valid_signed_certification_allows() {
    let repo = TempDir::new().unwrap();
    let (_key_dir, public_key_path, public_key) = generate_ssh_signer();
    init_git_repo(&repo);
    write_repo_file(
        &repo,
        ".repocert/config.toml",
        &format!(
            r#"
schema_version = 1

[checks.test]
argv = ["sh", "-c", "exit 0"]

[profiles.release]
checks = ["test"]
certify = true
default = true

[certification]
mode = "ssh-signed"

[[certification.trusted_signer]]
name = "test"
public_key = "{public_key}"

[[protected_refs]]
pattern = "refs/heads/*"
profile = "release"
"#
        ),
    );
    commit_all(&repo, "initial");
    let certify = Command::new(repocert_bin())
        .args([
            "certify",
            "--format",
            "json",
            "--signing-key",
            public_key_path.to_str().unwrap(),
        ])
        .current_dir(repo.path())
        .output()
        .unwrap();
    assert!(certify.status.success());
    let head = head_commit(&repo);

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

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["allowed"], true);
    assert_eq!(json["profile_results"][0]["state"], "certified");
    assert_eq!(json["profile_results"][0]["signer_name"], "test");
}

#[test]
fn authorize_stale_fingerprint_denies() {
    // Arrange
    let repo = TempDir::new().unwrap();
    let (_key_dir, public_key_path, public_key) = generate_ssh_signer();
    init_git_repo(&repo);
    write_repo_file(
        &repo,
        ".repocert/config.toml",
        &format!(
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
{}
"#,
            certification_block(&public_key)
        ),
    );
    commit_all(&repo, "initial");
    let certify = Command::new(repocert_bin())
        .args([
            "certify",
            "--format",
            "json",
            "--signing-key",
            public_key_path.to_str().unwrap(),
        ])
        .current_dir(repo.path())
        .output()
        .unwrap();
    assert!(certify.status.success());
    write_repo_file(
        &repo,
        ".repocert/config.toml",
        &format!(
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
{}
"#,
            certification_block(&public_key)
        ),
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
