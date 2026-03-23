use std::path::{Path, PathBuf};
use std::process::Command;

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

fn run_git_output(cwd: &Path, args: &[&str]) -> std::process::Output {
    Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .unwrap()
}

fn head_ref(repo: &TempDir) -> String {
    let output = run_git_output(repo.path(), &["symbolic-ref", "HEAD"]);
    assert!(output.status.success());
    String::from_utf8(output.stdout).unwrap().trim().to_string()
}

fn local_policy_config(protected_branch: &str) -> String {
    format!(
        r#"
schema_version = 1

[local_policy]
protected_branches = ["{protected_branch}"]
require_clean_primary_checkout = true

[hooks]
mode = "generated"

[hooks.generated]
hooks = ["pre-commit", "pre-merge-commit", "pre-push", "update"]
"#
    )
}

fn add_linked_worktree(repo: &TempDir, branch: &str) -> PathBuf {
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

#[test]
fn install_hooks_generated_blocks_direct_commit_on_protected_branch() {
    let repo = TempDir::new().unwrap();
    init_git_repo(&repo);
    let protected_branch = head_ref(&repo);
    write_repo_file(
        &repo,
        ".repocert/config.toml",
        &local_policy_config(&protected_branch),
    );
    write_repo_file(&repo, "README.md", "initial\n");
    commit_all(&repo, "initial");

    let install = run_install_hooks(&["--format", "json"], repo.path());
    assert!(install.status.success());

    write_repo_file(&repo, "README.md", "changed\n");
    let add = run_git_output(repo.path(), &["add", "README.md"]);
    assert!(add.status.success());
    let commit = run_git_output(repo.path(), &["commit", "-q", "-m", "blocked"]);

    assert_eq!(commit.status.code(), Some(1));
    let stderr = String::from_utf8(commit.stderr).unwrap();
    assert!(stderr.contains("local protected-branch policy blocks commits"));
}

#[test]
fn install_hooks_generated_blocks_commit_in_primary_checkout_even_on_feature_branch() {
    let repo = TempDir::new().unwrap();
    init_git_repo(&repo);
    let protected_branch = head_ref(&repo);
    write_repo_file(
        &repo,
        ".repocert/config.toml",
        &local_policy_config(&protected_branch),
    );
    write_repo_file(&repo, "README.md", "initial\n");
    commit_all(&repo, "initial");

    let install = run_install_hooks(&["--format", "json"], repo.path());
    assert!(install.status.success());

    let checkout = run_git_output(repo.path(), &["checkout", "-q", "-b", "feature"]);
    assert!(checkout.status.success());
    write_repo_file(&repo, "README.md", "feature\n");
    let add = run_git_output(repo.path(), &["add", "README.md"]);
    assert!(add.status.success());
    let commit = run_git_output(repo.path(), &["commit", "-q", "-m", "blocked"]);

    assert_eq!(commit.status.code(), Some(1));
    let stderr = String::from_utf8(commit.stderr).unwrap();
    assert!(stderr.contains("primary checkout to stay clean"));
}

#[test]
fn install_hooks_generated_allows_feature_branch_commit_in_linked_worktree() {
    let repo = TempDir::new().unwrap();
    init_git_repo(&repo);
    let protected_branch = head_ref(&repo);
    write_repo_file(
        &repo,
        ".repocert/config.toml",
        &local_policy_config(&protected_branch),
    );
    write_repo_file(&repo, "README.md", "initial\n");
    commit_all(&repo, "initial");

    let install = run_install_hooks(&["--format", "json"], repo.path());
    assert!(install.status.success());

    let worktree = add_linked_worktree(&repo, "feature");
    let worktree_install = run_install_hooks(&["--format", "json"], &worktree);
    assert!(worktree_install.status.success());
    write_repo_file_path(&worktree, "README.md", "feature\n");
    let add = run_git_output(&worktree, &["add", "README.md"]);
    assert!(add.status.success());
    let commit = run_git_output(&worktree, &["commit", "-q", "-m", "allowed"]);

    assert!(
        commit.status.success(),
        "{}",
        String::from_utf8_lossy(&commit.stderr)
    );
}

#[test]
fn install_hooks_generated_blocks_merge_commit_on_protected_branch() {
    let repo = TempDir::new().unwrap();
    init_git_repo(&repo);
    let protected_branch = head_ref(&repo);
    write_repo_file(
        &repo,
        ".repocert/config.toml",
        &local_policy_config(&protected_branch),
    );
    write_repo_file(&repo, "README.md", "initial\n");
    commit_all(&repo, "initial");

    let install = run_install_hooks(&["--format", "json"], repo.path());
    assert!(install.status.success());

    let worktree = add_linked_worktree(&repo, "feature");
    let worktree_install = run_install_hooks(&["--format", "json"], &worktree);
    assert!(worktree_install.status.success());
    write_repo_file_path(&worktree, "README.md", "feature\n");
    let add = run_git_output(&worktree, &["add", "README.md"]);
    assert!(add.status.success());
    let feature_commit = run_git_output(&worktree, &["commit", "-q", "-m", "feature"]);
    assert!(
        feature_commit.status.success(),
        "{}",
        String::from_utf8_lossy(&feature_commit.stderr)
    );

    let merge = run_git_output(repo.path(), &["merge", "--no-ff", "feature", "-m", "merge"]);
    assert_eq!(merge.status.code(), Some(1));
    let stderr = String::from_utf8(merge.stderr).unwrap();
    assert!(stderr.contains("local protected-branch policy blocks commits"));
}

fn write_repo_file_path(repo_root: &Path, relative_path: &str, contents: &str) {
    let path = repo_root.join(relative_path);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path, contents).unwrap();
}
