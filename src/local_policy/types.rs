use crate::config::{LoadOptions, LoadPaths};

#[derive(Clone, Debug)]
pub struct LocalPolicyOptions {
    pub load_options: LoadOptions,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LocalPolicyViolation {
    ProtectedBranch {
        pattern: String,
        current_ref: String,
    },
    DirtyPrimaryCheckout {
        dirty_paths: Vec<String>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalPolicyDecision {
    pub paths: LoadPaths,
    pub current_ref: Option<String>,
    pub is_primary_checkout: bool,
    pub worktree_dirty: bool,
    pub violations: Vec<LocalPolicyViolation>,
}

impl LocalPolicyDecision {
    pub fn ok(&self) -> bool {
        self.violations.is_empty()
    }
}
