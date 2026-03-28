use std::fs;
use std::path::{Path, PathBuf};

use crate::config::Contract;
use crate::hooks::InstallHooksError;

use super::types::GeneratedHook;

pub(super) fn generated_hooks_for_contract(contract: &Contract) -> Vec<GeneratedHook> {
    let mut hooks = Vec::new();

    if contract.local_policy.is_some() {
        hooks.push(GeneratedHook::PreCommit);
        hooks.push(GeneratedHook::PreMergeCommit);
    }

    if !contract.protected_refs.is_empty() {
        hooks.push(GeneratedHook::PrePush);
        hooks.push(GeneratedHook::Update);
    }

    hooks
}

pub(super) fn generated_hooks_dir(git_dir: &Path) -> PathBuf {
    git_dir.join("repocert").join("hooks").join("generated")
}

pub(super) fn sync_generated_hooks(
    hooks_dir: &Path,
    hooks: &[GeneratedHook],
    executable_path: &Path,
    repo_root: &Path,
) -> Result<Vec<String>, InstallHooksError> {
    fs::create_dir_all(hooks_dir).map_err(|source| InstallHooksError::GeneratedHookWrite {
        hook: "directory".to_string(),
        path: hooks_dir.to_path_buf(),
        source,
    })?;

    let mut repaired = Vec::new();
    for hook in hooks {
        let path = hooks_dir.join(hook.as_str());
        let content = hook_script(hook, executable_path, repo_root);
        let needs_write = match fs::read_to_string(&path) {
            Ok(existing) => existing != content,
            Err(_) => true,
        };

        if needs_write {
            fs::write(&path, content).map_err(|source| InstallHooksError::GeneratedHookWrite {
                hook: hook.as_str().to_string(),
                path: path.clone(),
                source,
            })?;
            set_executable(&path).map_err(|source| InstallHooksError::GeneratedHookWrite {
                hook: hook.as_str().to_string(),
                path: path.clone(),
                source,
            })?;
            repaired.push(format!("generated hook {}", hook.as_str()));
        }
    }

    let desired = hooks
        .iter()
        .map(GeneratedHook::as_str)
        .map(str::to_string)
        .collect::<std::collections::BTreeSet<_>>();
    for entry in
        fs::read_dir(hooks_dir).map_err(|source| InstallHooksError::GeneratedHookPrune {
            path: hooks_dir.to_path_buf(),
            source,
        })?
    {
        let entry = entry.map_err(|source| InstallHooksError::GeneratedHookPrune {
            path: hooks_dir.to_path_buf(),
            source,
        })?;
        let file_name = entry.file_name().to_string_lossy().into_owned();
        if !desired.contains(&file_name) && entry.path().is_file() {
            fs::remove_file(entry.path()).map_err(|source| {
                InstallHooksError::GeneratedHookPrune {
                    path: entry.path(),
                    source,
                }
            })?;
            repaired.push(format!("removed stale generated hook {file_name}"));
        }
    }

    Ok(repaired)
}

fn hook_script(hook: &GeneratedHook, executable_path: &Path, repo_root: &Path) -> String {
    let exe = shell_quote(executable_path);
    let repo = shell_quote(repo_root);
    match hook {
        GeneratedHook::PreCommit => {
            format!("#!/bin/sh\nset -eu\nexec {exe} hook run --repo-root {repo} pre-commit\n")
        }
        GeneratedHook::PreMergeCommit => {
            format!("#!/bin/sh\nset -eu\nexec {exe} hook run --repo-root {repo} pre-merge-commit\n")
        }
        GeneratedHook::Update => format!(
            "#!/bin/sh\nset -eu\nexec {exe} hook run --repo-root {repo} update \"$1\" \"$2\" \"$3\"\n"
        ),
        GeneratedHook::PrePush => {
            format!("#!/bin/sh\nset -eu\nexec {exe} hook run --repo-root {repo} pre-push\n")
        }
    }
}

fn shell_quote(path: &Path) -> String {
    let raw = path.display().to_string();
    format!("'{}'", raw.replace('\'', "'\"'\"'"))
}

#[cfg(unix)]
fn set_executable(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions)
}

#[cfg(not(unix))]
fn set_executable(_path: &Path) -> std::io::Result<()> {
    Ok(())
}
