/// One local policy violation found in the current checkout.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LocalPolicyViolation {
    /// The current branch matches a protected-branch pattern.
    ProtectedBranch {
        /// Matched protected-branch pattern.
        pattern: String,
        /// Current symbolic ref name.
        current_ref: String,
    },
    /// The primary checkout is dirty when policy requires it to stay clean.
    DirtyPrimaryCheckout {
        /// Dirty paths visible in the checkout snapshot.
        dirty_paths: Vec<String>,
    },
}

/// Result of checking local policy against the current checkout.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalPolicyDecision {
    /// Current symbolic ref, when one is available.
    pub current_ref: Option<String>,
    /// Whether the current checkout is the repository's primary checkout.
    pub is_primary_checkout: bool,
    /// Whether the current worktree snapshot is dirty.
    pub worktree_dirty: bool,
    /// Violations detected for the current checkout.
    pub violations: Vec<LocalPolicyViolation>,
}

impl LocalPolicyDecision {
    /// Returns `true` when no local policy violations were found.
    pub fn ok(&self) -> bool {
        self.violations.is_empty()
    }
}
