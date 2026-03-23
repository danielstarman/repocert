use crate::config::load_contract;
use crate::contract::matches_pattern;
use crate::git::{capture_worktree_snapshot, inspect_checkout};

use super::error::LocalPolicyError;
use super::types::{LocalPolicyDecision, LocalPolicyOptions, LocalPolicyViolation};

pub fn check_local_commit_policy(
    options: LocalPolicyOptions,
) -> Result<LocalPolicyDecision, LocalPolicyError> {
    let loaded = load_contract(options.load_options)?;
    let checkout = inspect_checkout(&loaded.paths.repo_root).map_err(|error| {
        LocalPolicyError::GitCheckout {
            paths: loaded.paths.clone(),
            error,
        }
    })?;
    let snapshot = capture_worktree_snapshot(&loaded.paths.repo_root).map_err(|error| {
        LocalPolicyError::GitWorktree {
            paths: loaded.paths.clone(),
            error,
        }
    })?;

    let mut violations = Vec::new();
    if let Some(policy) = loaded.contract.local_policy.as_ref() {
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
                            paths: loaded.paths.clone(),
                            pattern: pattern.clone(),
                            message,
                        });
                    }
                }
            }
        }
    }

    Ok(LocalPolicyDecision {
        paths: loaded.paths,
        current_ref: checkout.head_ref,
        is_primary_checkout: checkout.is_primary_checkout,
        worktree_dirty: !snapshot.is_clean(),
        violations,
    })
}
