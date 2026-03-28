use crate::config::{HookMode, RepoSession};
use crate::git::{
    enable_worktree_config, read_worktree_core_hooks_path, resolve_git_dir,
    unset_local_core_hooks_path, write_worktree_core_hooks_path,
};

use super::error::InstallHooksError;
use super::generated::{generated_hooks_dir, generated_hooks_for_contract, sync_generated_hooks};
use super::types::{HookInstallMode, InstallHooksOptions, InstallHooksReport};

/// Install generated git hooks for the target repository/worktree.
pub fn install_hooks(
    session: &RepoSession,
    options: InstallHooksOptions,
) -> Result<InstallHooksReport, InstallHooksError> {
    let InstallHooksOptions { executable_path } = options;

    let hooks = session
        .contract
        .hooks
        .as_ref()
        .ok_or(InstallHooksError::MissingHooksConfig)?;

    let (mode, hooks_path, desired_hooks_path, mut repaired_items) = match &hooks.mode {
        HookMode::Generated => {
            let git_dir = resolve_git_dir(&session.paths.repo_root)?;
            let hooks_path = generated_hooks_dir(&git_dir);
            let hooks = generated_hooks_for_contract(&session.contract);
            let repaired = sync_generated_hooks(
                &hooks_path,
                &hooks,
                &executable_path,
                &session.paths.repo_root,
            )?;
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

    if enable_worktree_config(&session.paths.repo_root)? {
        repaired_items.push("extensions.worktreeConfig".to_string());
    }

    if unset_local_core_hooks_path(&session.paths.repo_root)? {
        repaired_items.push("core.hooksPath (local)".to_string());
    }

    let current_hooks_path = read_worktree_core_hooks_path(&session.paths.repo_root)?;
    if current_hooks_path.as_deref() != Some(desired_hooks_path.as_str()) {
        write_worktree_core_hooks_path(&session.paths.repo_root, &desired_hooks_path)?;
        repaired_items.push("core.hooksPath".to_string());
    }

    let changed = !repaired_items.is_empty();

    Ok(InstallHooksReport {
        mode,
        hooks_path,
        changed,
        repaired_items,
    })
}
