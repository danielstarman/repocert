use super::{CertificationKey, CertificationStore, ContractFingerprint, StorageError};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ProfileCertificationState {
    Certified,
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
) -> Result<ProfileCertificationInspection, StorageError> {
    let key = CertificationKey {
        commit: commit.to_string(),
        profile: profile.to_string(),
    };
    if let Some(record) = store.read(&key)? {
        let state = if record.contract_fingerprint == *current_fingerprint {
            ProfileCertificationState::Certified
        } else {
            ProfileCertificationState::StaleFingerprint
        };
        return Ok(ProfileCertificationInspection {
            profile: profile.to_string(),
            state,
            other_certified_commits: Vec::new(),
            recorded_fingerprint: Some(record.contract_fingerprint),
        });
    }

    let mut other_commits = store
        .list_for_profile(profile)?
        .into_iter()
        .map(|record| record.key.commit)
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
