use crate::config::{HookMode, load_contract};
use crate::git::{
    enable_worktree_config, read_worktree_core_hooks_path, resolve_git_dir,
    unset_local_core_hooks_path, write_worktree_core_hooks_path,
};

use super::error::InstallHooksError;
use super::generated::{generated_hooks_dir, sync_generated_hooks, validate_supported_hooks};
use super::types::{HookInstallMode, InstallHooksOptions, InstallHooksReport};

pub fn install_hooks(
    options: InstallHooksOptions,
) -> Result<InstallHooksReport, InstallHooksError> {
    let InstallHooksOptions {
        load_options,
        executable_path,
    } = options;

    let loaded = load_contract(load_options)?;
    let hooks =
        loaded
            .contract
            .hooks
            .as_ref()
            .ok_or_else(|| InstallHooksError::MissingHooksConfig {
                paths: loaded.paths.clone(),
            })?;

    let (mode, hooks_path, desired_hooks_path, mut repaired_items) = match &hooks.mode {
        HookMode::RepoOwned { path } => {
            let hooks_path = loaded.paths.repo_root.join(path.as_str());
            if !hooks_path.is_dir() {
                return Err(InstallHooksError::MissingRepoOwnedHookDir {
                    paths: loaded.paths.clone(),
                    path: hooks_path,
                });
            }
            (
                HookInstallMode::RepoOwned,
                hooks_path,
                path.as_str().to_string(),
                Vec::new(),
            )
        }
        HookMode::Generated { hooks } => {
            validate_supported_hooks(&loaded.paths, hooks)?;
            let git_dir = resolve_git_dir(&loaded.paths.repo_root).map_err(|error| {
                InstallHooksError::GitDir {
                    paths: loaded.paths.clone(),
                    error,
                }
            })?;
            let hooks_path = generated_hooks_dir(&git_dir);
            let repaired =
                sync_generated_hooks(&loaded.paths, &hooks_path, hooks, &executable_path)?;
            let desired_hooks_path = hooks_path
                .canonicalize()
                .unwrap_or_else(|_| hooks_path.clone())
                .display()
                .to_string();
            (
                HookInstallMode::Generated,
                hooks_path,
                desired_hooks_path,
                repaired,
            )
        }
    };

    let hooks_path = hooks_path
        .canonicalize()
        .unwrap_or_else(|_| hooks_path.clone());

    if enable_worktree_config(&loaded.paths.repo_root).map_err(|error| {
        InstallHooksError::GitHooksPath {
            paths: loaded.paths.clone(),
            error,
        }
    })? {
        repaired_items.push("extensions.worktreeConfig".to_string());
    }

    if unset_local_core_hooks_path(&loaded.paths.repo_root).map_err(|error| {
        InstallHooksError::GitHooksPath {
            paths: loaded.paths.clone(),
            error,
        }
    })? {
        repaired_items.push("core.hooksPath (local)".to_string());
    }

    let current_hooks_path =
        read_worktree_core_hooks_path(&loaded.paths.repo_root).map_err(|error| {
            InstallHooksError::GitHooksPath {
                paths: loaded.paths.clone(),
                error,
            }
        })?;
    if current_hooks_path.as_deref() != Some(desired_hooks_path.as_str()) {
        write_worktree_core_hooks_path(&loaded.paths.repo_root, &desired_hooks_path).map_err(
            |error| InstallHooksError::GitHooksPath {
                paths: loaded.paths.clone(),
                error,
            },
        )?;
        repaired_items.push("core.hooksPath".to_string());
    }

    let changed = !repaired_items.is_empty();

    Ok(InstallHooksReport {
        paths: loaded.paths,
        mode,
        hooks_path,
        changed,
        repaired_items,
    })
}
