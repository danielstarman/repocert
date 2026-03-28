use std::path::Path;

use crate::config::{CertificationConfig, CertificationMode};
use crate::git::{GitCommitError, commit_exists};
use thiserror::Error;

use super::{
    CertificationKey, CertificationRecord, CertificationStore, ContractFingerprint, StorageError,
    find_trusted_signer, verify_payload_with_ssh,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ProfileCertificationState {
    Certified,
    UntrustedSigner,
    InvalidSignature,
    StaleCommit,
    StaleFingerprint,
    Uncertified,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ProfileCertificationInspection {
    pub profile: String,
    pub state: ProfileCertificationState,
    pub signer_name: Option<String>,
    pub other_certified_commits: Vec<String>,
    pub recorded_fingerprint: Option<ContractFingerprint>,
}

#[derive(Debug, Error)]
pub(crate) enum ProfileCertificationError {
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error(transparent)]
    GitCommit(#[from] GitCommitError),
}

pub(crate) fn inspect_profile_certification(
    store: &CertificationStore,
    repo_root: &Path,
    commit: &str,
    profile: &str,
    current_fingerprint: &ContractFingerprint,
    certification: &CertificationConfig,
) -> Result<ProfileCertificationInspection, ProfileCertificationError> {
    let key = CertificationKey {
        commit: commit.to_string(),
        profile: profile.to_string(),
    };
    if let Some(record) = read_record_for_inspection(store, &key)? {
        let state = if record.contract_fingerprint() == current_fingerprint {
            authenticate_record(&record, certification)?
        } else {
            ProfileCertificationState::StaleFingerprint
        };
        return Ok(ProfileCertificationInspection {
            profile: profile.to_string(),
            state,
            signer_name: signer_name_for_record(&record, certification),
            other_certified_commits: Vec::new(),
            recorded_fingerprint: Some(record.contract_fingerprint().clone()),
        });
    }

    let other_commits =
        collect_other_certified_commits(store, repo_root, commit, profile, certification)?;

    if other_commits.is_empty() {
        Ok(ProfileCertificationInspection {
            profile: profile.to_string(),
            state: ProfileCertificationState::Uncertified,
            signer_name: None,
            other_certified_commits: Vec::new(),
            recorded_fingerprint: None,
        })
    } else {
        Ok(ProfileCertificationInspection {
            profile: profile.to_string(),
            state: ProfileCertificationState::StaleCommit,
            signer_name: None,
            other_certified_commits: other_commits,
            recorded_fingerprint: None,
        })
    }
}

fn signer_name_for_record(
    record: &CertificationRecord,
    certification: &CertificationConfig,
) -> Option<String> {
    let CertificationConfig {
        mode: CertificationMode::SshSigned { trusted_signer },
    } = certification;

    find_trusted_signer(trusted_signer.as_slice(), &record.signer_fingerprint)
        .map(|signer| signer.name.clone())
}

fn authenticate_record(
    record: &CertificationRecord,
    certification: &CertificationConfig,
) -> Result<ProfileCertificationState, StorageError> {
    match &certification.mode {
        CertificationMode::SshSigned { trusted_signer } => {
            match verify_payload_with_ssh(record, trusted_signer) {
                Ok(()) => Ok(ProfileCertificationState::Certified),
                Err(crate::certification::SigningError::UntrustedSigner { .. }) => {
                    Ok(ProfileCertificationState::UntrustedSigner)
                }
                Err(crate::certification::SigningError::InvalidSignature { .. }) => {
                    Ok(ProfileCertificationState::InvalidSignature)
                }
                Err(error) => Err(error.into()),
            }
        }
    }
}

fn is_valid_alternate_certification(
    repo_root: &Path,
    record: &CertificationRecord,
    certification: &CertificationConfig,
) -> Result<bool, GitCommitError> {
    if !commit_exists(repo_root, &record.key().commit)? {
        return Ok(false);
    }

    Ok(match certification {
        CertificationConfig {
            mode: CertificationMode::SshSigned { trusted_signer },
        } => verify_payload_with_ssh(record, trusted_signer).is_ok(),
    })
}

fn collect_other_certified_commits(
    store: &CertificationStore,
    repo_root: &Path,
    inspected_commit: &str,
    profile: &str,
    certification: &CertificationConfig,
) -> Result<Vec<String>, ProfileCertificationError> {
    let mut commits = Vec::new();

    for commit in store.list_commit_ids()? {
        if commit == inspected_commit {
            continue;
        }

        let key = CertificationKey {
            commit: commit.clone(),
            profile: profile.to_string(),
        };
        let Some(record) = read_record_for_inspection(store, &key)? else {
            continue;
        };
        if is_valid_alternate_certification(repo_root, &record, certification)? {
            commits.push(commit);
        }
    }

    Ok(commits)
}

fn read_record_for_inspection(
    store: &CertificationStore,
    key: &CertificationKey,
) -> Result<Option<CertificationRecord>, ProfileCertificationError> {
    match store.read(key) {
        Ok(record) => Ok(record),
        Err(error) if should_ignore_during_inspection(&error) => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn should_ignore_during_inspection(error: &StorageError) -> bool {
    matches!(
        error,
        StorageError::InvalidCommitId { .. }
            | StorageError::Json { .. }
            | StorageError::InvalidStoredRecordKey { .. }
    )
}
