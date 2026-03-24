use std::collections::BTreeMap;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(super) struct RawConfig {
    pub schema_version: u64,
    #[serde(default)]
    pub checks: BTreeMap<String, RawCommand>,
    #[serde(default)]
    pub fixers: BTreeMap<String, RawFixer>,
    #[serde(default)]
    pub profiles: BTreeMap<String, RawProfile>,
    #[serde(default)]
    pub protected_paths: Vec<String>,
    #[serde(default)]
    pub protected_refs: Vec<RawProtectedRef>,
    pub local_policy: Option<RawLocalPolicy>,
    pub hooks: Option<RawHooks>,
}

#[derive(Debug, Deserialize)]
pub(super) struct RawCommand {
    pub argv: Vec<String>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub(super) struct RawFixer {
    #[serde(flatten)]
    pub command: RawCommand,
    pub probe_argv: Option<Vec<String>>,
    pub probe_timeout_ms: Option<u64>,
}

#[derive(Debug, Default, Deserialize)]
pub(super) struct RawProfile {
    #[serde(default)]
    pub checks: Vec<String>,
    #[serde(default)]
    pub fixers: Vec<String>,
    #[serde(default)]
    pub includes: Vec<String>,
    #[serde(default)]
    pub default: bool,
    #[serde(default)]
    pub certify: bool,
}

#[derive(Debug, Deserialize)]
pub(super) struct RawProtectedRef {
    pub pattern: String,
    pub profile: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct RawLocalPolicy {
    #[serde(default)]
    pub protected_branches: Vec<String>,
    #[serde(default)]
    pub require_clean_primary_checkout: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RawHooks {
    pub mode: String,
    pub generated: Option<RawGeneratedHooks>,
}

#[derive(Debug, Deserialize)]
pub(super) struct RawGeneratedHooks {
    pub hooks: Option<Vec<String>>,
}
