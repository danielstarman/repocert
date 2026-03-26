use crate::config::{CertificationConfig, CertificationMode};

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

pub(crate) fn inspect_profile_certification(
    store: &CertificationStore,
    commit: &str,
    profile: &str,
    current_fingerprint: &ContractFingerprint,
    certification: &CertificationConfig,
) -> Result<ProfileCertificationInspection, StorageError> {
    let key = CertificationKey {
        commit: commit.to_string(),
        profile: profile.to_string(),
    };
    if let Some(record) = store.read(&key)? {
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

    let mut other_commits = store
        .list_for_profile(profile)?
        .into_iter()
        .filter(|record| counts_as_certified_elsewhere(record, certification))
        .map(|record| record.key().commit.clone())
        .collect::<Vec<_>>();
    other_commits.retain(|other_commit| other_commit != commit);

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

fn counts_as_certified_elsewhere(
    record: &CertificationRecord,
    certification: &CertificationConfig,
) -> bool {
    match certification {
        CertificationConfig {
            mode: CertificationMode::SshSigned { trusted_signer },
        } => verify_payload_with_ssh(record, trusted_signer).is_ok(),
    }
}
