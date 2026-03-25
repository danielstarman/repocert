use std::path::Path;

use crate::certification::{CertificationKey, CertificationRecord, StorageError};
use crate::git::resolve_git_common_dir;

use super::{CertificationStore, layout, records};

impl CertificationStore {
    pub fn open(repo_root: &Path) -> Result<Self, StorageError> {
        let common_dir = resolve_git_common_dir(repo_root)?;
        let root_dir = common_dir.join("repocert").join("certifications");
        Ok(Self {
            common_dir,
            root_dir,
        })
    }

    pub fn common_dir(&self) -> &Path {
        &self.common_dir
    }

    pub fn root_dir(&self) -> &Path {
        &self.root_dir
    }

    pub fn read(
        &self,
        key: &CertificationKey,
    ) -> Result<Option<CertificationRecord>, StorageError> {
        let path = layout::record_path(&self.root_dir, key)?;
        if !path.exists() {
            return Ok(None);
        }

        Ok(Some(records::read_record(&path, key)?))
    }

    pub fn write(&self, record: &CertificationRecord) -> Result<(), StorageError> {
        let directory = layout::commit_dir(&self.root_dir, &record.key().commit)?;
        records::write_record(&directory, record)
    }

    pub fn list_for_commit(&self, commit: &str) -> Result<Vec<CertificationRecord>, StorageError> {
        let directory = layout::commit_dir(&self.root_dir, commit)?;
        if !directory.exists() {
            return Ok(Vec::new());
        }

        let mut entries = records::list_commit_records(&directory, commit)?;
        entries.sort_by(|left, right| left.key().profile.cmp(&right.key().profile));
        Ok(entries)
    }

    pub fn list_for_profile(
        &self,
        profile: &str,
    ) -> Result<Vec<CertificationRecord>, StorageError> {
        if !self.root_dir.exists() {
            return Ok(Vec::new());
        }

        let mut entries = records::list_profile_records(&self.root_dir, profile)?;
        entries.sort_by(|left, right| left.key().commit.cmp(&right.key().commit));
        Ok(entries)
    }
}
