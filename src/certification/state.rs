use crate::config::{CertificationConfig, CertificationMode};

use super::{
    CertificationKey, CertificationRecord, CertificationStore, ContractFingerprint, StorageError,
    verify_payload_with_ssh,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ProfileCertificationState {
    Certified,
    LegacyUnsigned,
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
    pub other_certified_commits: Vec<String>,
    pub recorded_fingerprint: Option<ContractFingerprint>,
}

pub(crate) fn inspect_profile_certification(
    store: &CertificationStore,
    commit: &str,
    profile: &str,
    current_fingerprint: &ContractFingerprint,
    certification: Option<&CertificationConfig>,
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
            other_certified_commits: Vec::new(),
            recorded_fingerprint: None,
        })
    } else {
        Ok(ProfileCertificationInspection {
            profile: profile.to_string(),
            state: ProfileCertificationState::StaleCommit,
            other_certified_commits: other_commits,
            recorded_fingerprint: None,
        })
    }
}

fn authenticate_record(
    record: &CertificationRecord,
    certification: Option<&CertificationConfig>,
) -> Result<ProfileCertificationState, StorageError> {
    let Some(certification) = certification else {
        return Ok(ProfileCertificationState::Certified);
    };

    match (&certification.mode, record) {
        (_, CertificationRecord::Legacy(_)) => Ok(ProfileCertificationState::LegacyUnsigned),
        (CertificationMode::SshSigned { trusted_signer }, CertificationRecord::Signed(record)) => {
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
    certification: Option<&CertificationConfig>,
) -> bool {
    match certification {
        None => true,
        Some(CertificationConfig {
            mode: CertificationMode::SshSigned { trusted_signer },
        }) => match record {
            CertificationRecord::Legacy(_) => false,
            CertificationRecord::Signed(record) => {
                verify_payload_with_ssh(record, trusted_signer).is_ok()
            }
        },
    }
}
