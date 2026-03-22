use std::path::PathBuf;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CertificationStore {
    pub(super) common_dir: PathBuf,
    pub(super) root_dir: PathBuf,
}
