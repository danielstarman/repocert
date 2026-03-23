mod checkout;
mod commit;
mod common_dir;
mod hooks_path;
mod worktree;

pub(crate) use checkout::{GitCheckoutError, inspect_checkout};
pub(crate) use commit::{GitCommitError, resolve_commit, resolve_head_commit};
pub(crate) use common_dir::{
    GitCommonDirError, GitDirError, resolve_git_common_dir, resolve_git_dir,
};
pub(crate) use hooks_path::{
    GitHooksPathError, enable_worktree_config, read_worktree_core_hooks_path,
    unset_local_core_hooks_path, write_worktree_core_hooks_path,
};
pub(crate) use worktree::{
    GitWorktreeError, capture_pathspec_snapshot, capture_worktree_snapshot, protected_pathspecs,
};
