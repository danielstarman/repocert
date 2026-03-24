use std::path::PathBuf;

/// Git-local certification store rooted under the repository's common git dir.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CertificationStore {
    pub(super) common_dir: PathBuf,
    pub(super) root_dir: PathBuf,
}
