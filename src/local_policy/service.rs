use crate::config::RepoSession;
use crate::contract::matches_pattern;
use crate::git::{capture_worktree_snapshot, inspect_checkout};

use super::error::LocalPolicyError;
use super::types::{LocalPolicyDecision, LocalPolicyViolation};

/// Check whether the current checkout satisfies the configured local policy.
pub fn check_local_commit_policy(
    session: &RepoSession,
) -> Result<LocalPolicyDecision, LocalPolicyError> {
    let checkout = inspect_checkout(&session.paths.repo_root)?;
    let snapshot = capture_worktree_snapshot(&session.paths.repo_root)?;

    let mut violations = Vec::new();
    if let Some(policy) = session.contract.local_policy.as_ref() {
        if policy.require_clean_primary_checkout
            && checkout.is_primary_checkout
            && !snapshot.is_clean()
        {
            violations.push(LocalPolicyViolation::DirtyPrimaryCheckout {
                dirty_paths: snapshot.paths(),
            });
        }

        if let Some(current_ref) = checkout.head_ref.as_ref() {
            for pattern in &policy.protected_branches {
                match matches_pattern(pattern, current_ref) {
                    Ok(true) => violations.push(LocalPolicyViolation::ProtectedBranch {
                        pattern: pattern.clone(),
                        current_ref: current_ref.clone(),
                    }),
                    Ok(false) => {}
                    Err(message) => {
                        return Err(LocalPolicyError::InvalidPattern {
                            pattern: pattern.clone(),
                            message,
                        });
                    }
                }
            }
        }
    }

    Ok(LocalPolicyDecision {
        current_ref: checkout.head_ref,
        is_primary_checkout: checkout.is_primary_checkout,
        worktree_dirty: !snapshot.is_clean(),
        violations,
    })
}
