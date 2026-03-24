//! `repocert` is a local-first library and CLI for repository contract
//! validation, certification, and protected-ref enforcement.
//!
//! Repositories declare their contract in `.repocert/config.toml`, then use the
//! command-oriented APIs exposed here to:
//!
//! - load and validate the contract
//! - run checks and fixer probes
//! - apply repo-declared fixers
//! - certify exact commits against the current contract fingerprint
//! - inspect certification state
//! - authorize protected ref updates
//! - install generated git hooks that enforce the contract locally
//!
//! The crate primarily powers the `repocert` CLI, but its public modules are
//! available for embedding in other Rust tooling that wants the same contract
//! model and result types.

/// Certification storage and contract fingerprinting.
pub mod certification;
/// Non-mutating certification of the current `HEAD` commit.
pub mod certify;
/// Check execution and fixer probe evaluation.
pub mod check;
/// Contract discovery, loading, and structural validation.
pub mod config;
mod contract;
/// Protected ref authorization against stored certifications.
pub mod enforcement;
mod exec;
/// Mutating fixer execution.
pub mod fix;
mod git;
/// Generated hook installation and hook entrypoint types.
pub mod hooks;
/// Local checkout policy checks used by generated commit hooks.
pub mod local_policy;
/// Certification status inspection for commits and protected refs.
pub mod status;
