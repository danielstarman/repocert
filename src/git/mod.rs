mod common_dir;
mod head;
mod worktree;

pub(crate) use common_dir::{GitCommonDirError, resolve_git_common_dir};
pub(crate) use head::{GitHeadError, resolve_head_commit};
pub(crate) use worktree::{
    GitWorktreeError, capture_pathspec_snapshot, capture_worktree_snapshot, protected_pathspecs,
};
