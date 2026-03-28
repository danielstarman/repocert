#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]

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
//!
//! ## Module guide
//!
//! - [`config`] discovers, parses, and validates repository contracts.
//! - [`check`], [`fix`], and [`certify`] run the main contract-driven workflows.
//! - [`status`] and [`enforcement`] inspect and enforce certification state.
//! - [`certification`] exposes fingerprinting, storage, and signed-record types.
//! - [`hooks`] and [`local_policy`] support git hook installation and local
//!   checkout policy enforcement.
//!
//! ## Typical embedded flow
//!
//! Most integrations start by resolving and loading a repository session:
//!
//! - [`config::resolve_paths`]
//! - [`config::load_repo_session`]
//!
//! Then call one of the command-style entrypoints with that [`config::RepoSession`]:
//!
//! - [`check::run_check`]
//! - [`fix::run_fix`]
//! - [`certify::run_certify`]
//! - [`status::run_status`]
//! - [`enforcement::authorize_ref_update`]
//!
//! `repocert` remains CLI-first. Even when embedded as a library, certification
//! records are stored in git-local metadata and contract semantics are defined by
//! the repository's `.repocert/config.toml`.

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
