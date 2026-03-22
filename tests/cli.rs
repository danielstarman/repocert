use std::fs;
use std::process::Command;

use tempfile::TempDir;

#[path = "cli/check.rs"]
mod cli_check;
#[path = "cli/fix.rs"]
mod cli_fix;
#[path = "cli/validate.rs"]
mod cli_validate;

pub(crate) fn write_repo_file(repo: &TempDir, relative_path: &str, contents: &str) {
    let path = repo.path().join(relative_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, contents).unwrap();
}

pub(crate) fn run_git(repo: &TempDir, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(repo.path())
        .status()
        .unwrap();
    assert!(
        status.success(),
        "git command failed: git {}",
        args.join(" ")
    );
}

pub(crate) fn init_git_repo(repo: &TempDir) {
    run_git(repo, &["init", "-q"]);
    run_git(repo, &["config", "user.name", "Repocert Test"]);
    run_git(repo, &["config", "user.email", "repocert@example.com"]);
}

pub(crate) fn commit_all(repo: &TempDir, message: &str) {
    run_git(repo, &["add", "."]);
    run_git(repo, &["commit", "-q", "-m", message]);
}
