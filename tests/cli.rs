use std::fs;

use tempfile::TempDir;

#[path = "cli/check.rs"]
mod cli_check;
#[path = "cli/validate.rs"]
mod cli_validate;

pub(crate) fn write_repo_file(repo: &TempDir, relative_path: &str, contents: &str) {
    let path = repo.path().join(relative_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, contents).unwrap();
}
