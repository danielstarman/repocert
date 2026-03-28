use std::fs;
use std::process::Command;

use repocert::config::{LoadOptions, RepoSession, load_repo_session, resolve_paths};
use tempfile::TempDir;

#[path = "certification/fingerprint.rs"]
mod certification_fingerprint;
#[path = "certification/store.rs"]
mod certification_store;

pub(crate) fn load_contract(options: LoadOptions) -> RepoSession {
    let paths = resolve_paths(options).unwrap();
    load_repo_session(paths).unwrap()
}

pub(crate) fn write_repo_file(repo: &TempDir, relative_path: &str, contents: &str) {
    let path = repo.path().join(relative_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, contents).unwrap();
}

pub(crate) fn run_git(repo_path: &std::path::Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(repo_path)
        .status()
        .unwrap();
    assert!(
        status.success(),
        "git command failed in {:?}: git {}",
        repo_path,
        args.join(" ")
    );
}

pub(crate) fn init_git_repo(repo: &TempDir) {
    run_git(repo.path(), &["init", "-q"]);
    run_git(repo.path(), &["config", "user.name", "Repocert Test"]);
    run_git(
        repo.path(),
        &["config", "user.email", "repocert@example.com"],
    );
}

pub(crate) fn commit_all(repo: &TempDir, message: &str) {
    run_git(repo.path(), &["add", "."]);
    run_git(repo.path(), &["commit", "-q", "-m", message]);
}
