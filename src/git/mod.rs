mod common_dir;
mod status;

pub(crate) use common_dir::{GitCommonDirError, resolve_git_common_dir};
pub(crate) use status::{capture_snapshot, protected_pathspecs};
