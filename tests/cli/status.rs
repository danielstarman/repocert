use std::path::{Path, PathBuf};
use std::process::Command;

use repocert::certification::{
    CertificationKey, CertificationPayload, CertificationRecord, CertificationStore,
    ContractFingerprint, compute_contract_fingerprint, sign_payload_with_ssh,
};
use repocert::config::LoadOptions;
use serde_json::Value;
use tempfile::TempDir;

use crate::{commit_all, init_git_repo, load_contract, write_repo_file};

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

[profiles.default]
checks = ["test"]
certify = true
default = true
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

[profiles.default]
checks = ["test"]
certify = true
default = true
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

[profiles.default]
checks = ["test"]
certify = true
default = true
{}
"#,
            certification_block(&public_key)
        ),
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

[profiles.default]
checks = ["test"]
certify = true
default = true
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
{}
"#,
            certification_block(&public_key)
        ),
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
fn status_assert_certified_on_main_infers_default_profile() {
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

    let output = run_status(&["--format", "json", "--assert-certified"], repo.path());

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["profiles"], serde_json::json!(["default"]));
    assert_eq!(json["profile_results"][0]["state"], "certified");
}

#[test]
fn status_assert_certified_on_release_branch_infers_release_profile() {
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
{}
"#,
            certification_block(&public_key)
        ),
    );
    commit_all(&repo, "initial");
    let checkout = Command::new("git")
        .args(["checkout", "-q", "-b", "release/0.3"])
        .current_dir(repo.path())
        .output()
        .unwrap();
    assert!(checkout.status.success());
    let certify = Command::new(repocert_bin())
        .args([
            "certify",
            "--profile",
            "release",
            "--format",
            "json",
            "--signing-key",
            public_key_path.to_str().unwrap(),
        ])
        .current_dir(repo.path())
        .output()
        .unwrap();
    assert!(certify.status.success());

    let output = run_status(&["--format", "json", "--assert-certified"], repo.path());

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["profiles"], serde_json::json!(["release"]));
    assert_eq!(json["profile_results"][0]["state"], "certified");
}

#[test]
fn status_protected_refs_report_certification_state() {
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
fn status_legacy_unsigned_record_is_ignored_as_uncertified() {
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

[[certification.trusted_signer]]
name = "test"
public_key = "{public_key}"
"#
        ),
    );
    commit_all(&repo, "initial");

    let commit = head_commit(&repo);
    let store = CertificationStore::open(repo.path()).unwrap();
    let path = store.root_dir().join(&commit).join("64656661756c74.json");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(
        &path,
        serde_json::to_vec_pretty(&CertificationPayload {
            key: CertificationKey {
                commit,
                profile: "default".to_string(),
            },
            contract_fingerprint: repocert::certification::ContractFingerprint::from_bytes([7; 32]),
        })
        .unwrap(),
    )
    .unwrap();

    let output = run_status(&["--format", "json"], repo.path());

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["ok"], true);
    assert_eq!(json["error"], Value::Null);
    assert_eq!(json["profile_results"][0]["state"], "uncertified");
}

#[test]
fn status_orphaned_other_commit_record_is_ignored() {
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

[profiles.default]
checks = ["test"]
certify = true
default = true

[certification]
mode = "ssh-signed"

[[certification.trusted_signer]]
name = "test"
public_key = "{public_key}"
"#
        ),
    );
    commit_all(&repo, "initial");

    let orphan_commit = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    let store = CertificationStore::open(repo.path()).unwrap();
    let path = store
        .root_dir()
        .join(orphan_commit)
        .join("64656661756c74.json");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let record = sign_payload_with_ssh(
        &public_key_path,
        &CertificationPayload {
            key: CertificationKey {
                commit: orphan_commit.to_string(),
                profile: "default".to_string(),
            },
            contract_fingerprint: ContractFingerprint::from_bytes([7; 32]),
        },
    )
    .unwrap();
    std::fs::write(path, serde_json::to_vec_pretty(&record).unwrap()).unwrap();

    let output = run_status(&["--format", "json"], repo.path());

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["ok"], true);
    assert_eq!(json["error"], Value::Null);
    assert_eq!(json["profile_results"][0]["state"], "uncertified");
    assert_eq!(
        json["profile_results"][0]["other_certified_commits"],
        serde_json::json!([])
    );
}

#[test]
fn status_signed_mode_reports_signer_name() {
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

[profiles.default]
checks = ["test"]
certify = true
default = true

[certification]
mode = "ssh-signed"

[[certification.trusted_signer]]
name = "test"
public_key = "{public_key}"
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

    let output = run_status(&["--format", "json"], repo.path());

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["profile_results"][0]["state"], "certified");
    assert_eq!(json["profile_results"][0]["signer_name"], "test");
}

#[test]
fn status_signed_mode_untrusted_signer_reports_state() {
    let repo = TempDir::new().unwrap();
    let (_trusted_dir, _trusted_key_path, trusted_public_key) = generate_ssh_signer();
    let (_other_dir, other_key_path, _other_public_key) = generate_ssh_signer();
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

[[certification.trusted_signer]]
name = "trusted"
public_key = "{trusted_public_key}"
"#
        ),
    );
    commit_all(&repo, "initial");
    let commit = head_commit(&repo);
    let record = sign_payload_with_ssh(
        &other_key_path,
        &CertificationPayload {
            key: CertificationKey {
                commit,
                profile: "default".to_string(),
            },
            contract_fingerprint: current_contract_fingerprint(&repo),
        },
    )
    .unwrap();
    CertificationStore::open(repo.path())
        .unwrap()
        .write(&record)
        .unwrap();

    let json_output = run_status(&["--format", "json"], repo.path());

    assert!(json_output.status.success());
    let json: Value = serde_json::from_slice(&json_output.stdout).unwrap();
    assert_eq!(json["profile_results"][0]["state"], "untrusted_signer");
    assert_eq!(json["profile_results"][0]["signer_name"], Value::Null);
}

#[test]
fn status_signed_mode_invalid_signature_reports_state() {
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

[profiles.default]
checks = ["test"]
certify = true
default = true
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

    let commit = head_commit(&repo);
    let store = CertificationStore::open(repo.path()).unwrap();
    let path = store.root_dir().join(&commit).join("64656661756c74.json");
    tamper_record_signature(&path);

    let output = run_status(&["--format", "json"], repo.path());

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["profile_results"][0]["state"], "invalid_signature");
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

fn tamper_record_signature(path: &Path) {
    let mut record: CertificationRecord =
        serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    record.signature = "invalid-signature".to_string();
    std::fs::write(path, serde_json::to_vec_pretty(&record).unwrap()).unwrap();
}

fn current_contract_fingerprint(repo: &TempDir) -> ContractFingerprint {
    let loaded = load_contract(LoadOptions {
        start_dir: None,
        repo_root: Some(repo.path().to_path_buf()),
        config_path: None,
    });
    compute_contract_fingerprint(&loaded).unwrap()
}
