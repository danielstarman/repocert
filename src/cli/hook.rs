use std::io::{self, BufRead};
use std::process::ExitCode;

use repocert::config::LoadError;
use repocert::enforcement::{AuthorizeError, AuthorizeOptions, authorize_ref_update};
use repocert::hooks::GeneratedHook;
use repocert::local_policy::{
    LocalPolicyError, LocalPolicyOptions, LocalPolicyViolation, check_local_commit_policy,
};

use super::app::{HookArgs, HookCommand, HookRunArgs};

pub(super) fn run(args: HookArgs) -> ExitCode {
    match args.command {
        HookCommand::Run(args) => run_hook(args),
    }
}

fn run_hook(args: HookRunArgs) -> ExitCode {
    match GeneratedHook::parse(args.hook.as_str()) {
        Some(GeneratedHook::PreCommit | GeneratedHook::PreMergeCommit) => {
            run_local_policy_hook(args)
        }
        Some(GeneratedHook::PrePush) => run_pre_push_hook(args),
        Some(GeneratedHook::Update) => run_update_hook(args),
        other => {
            eprintln!("FAIL hook [input]");
            eprintln!(
                "unsupported generated hook {:?}",
                other.map(|hook| hook.as_str())
            );
            ExitCode::from(1)
        }
    }
}

fn run_local_policy_hook(args: HookRunArgs) -> ExitCode {
    let options = LocalPolicyOptions {
        load_options: repocert::config::LoadOptions {
            start_dir: None,
            repo_root: args.repo_root,
            config_path: args.config_path,
        },
    };

    match check_local_commit_policy(options) {
        Ok(decision) => {
            if decision.ok() {
                ExitCode::SUCCESS
            } else {
                for violation in &decision.violations {
                    eprintln!("{}", local_policy_violation_message(violation));
                }
                ExitCode::from(1)
            }
        }
        Err(error) => {
            eprintln!("FAIL hook [{}]", local_policy_error_category(&error));
            eprintln!("{error}");
            ExitCode::from(1)
        }
    }
}

fn run_pre_push_hook(args: HookRunArgs) -> ExitCode {
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let line = match line {
            Ok(line) => line,
            Err(error) => {
                eprintln!("FAIL hook [io]");
                eprintln!("failed to read pre-push stdin: {error}");
                return ExitCode::from(1);
            }
        };
        if line.trim().is_empty() {
            continue;
        }

        let mut parts = line.split_whitespace();
        let Some(_local_ref) = parts.next() else {
            continue;
        };
        let Some(local_oid) = parts.next() else {
            eprintln!("FAIL hook [input]");
            eprintln!("invalid pre-push input: {line:?}");
            return ExitCode::from(1);
        };
        let Some(remote_ref) = parts.next() else {
            eprintln!("FAIL hook [input]");
            eprintln!("invalid pre-push input: {line:?}");
            return ExitCode::from(1);
        };
        let Some(remote_oid) = parts.next() else {
            eprintln!("FAIL hook [input]");
            eprintln!("invalid pre-push input: {line:?}");
            return ExitCode::from(1);
        };
        if parts.next().is_some() {
            eprintln!("FAIL hook [input]");
            eprintln!("invalid pre-push input: {line:?}");
            return ExitCode::from(1);
        }

        match run_authorize(
            args.repo_root.clone(),
            args.config_path.clone(),
            remote_oid,
            local_oid,
            remote_ref,
        ) {
            Ok(true) => {}
            Ok(false) => return ExitCode::from(1),
            Err(message) => {
                eprintln!("{message}");
                return ExitCode::from(1);
            }
        }
    }

    ExitCode::SUCCESS
}

fn run_update_hook(args: HookRunArgs) -> ExitCode {
    let Some(reference) = args.args.first() else {
        eprintln!("FAIL hook [input]");
        eprintln!("update hook requires ref old new arguments");
        return ExitCode::from(1);
    };
    let Some(old) = args.args.get(1) else {
        eprintln!("FAIL hook [input]");
        eprintln!("update hook requires ref old new arguments");
        return ExitCode::from(1);
    };
    let Some(new) = args.args.get(2) else {
        eprintln!("FAIL hook [input]");
        eprintln!("update hook requires ref old new arguments");
        return ExitCode::from(1);
    };
    if args.args.len() != 3 {
        eprintln!("FAIL hook [input]");
        eprintln!("update hook requires ref old new arguments");
        return ExitCode::from(1);
    }

    match run_authorize(args.repo_root, args.config_path, old, new, reference) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(1),
        Err(message) => {
            eprintln!("{message}");
            ExitCode::from(1)
        }
    }
}

fn run_authorize(
    repo_root: Option<std::path::PathBuf>,
    config_path: Option<std::path::PathBuf>,
    old: &str,
    new: &str,
    reference: &str,
) -> Result<bool, String> {
    let options = AuthorizeOptions {
        load_options: repocert::config::LoadOptions {
            start_dir: None,
            repo_root,
            config_path,
        },
        old: old.to_string(),
        new: new.to_string(),
        reference: reference.to_string(),
    };

    match authorize_ref_update(options) {
        Ok(report) => {
            let overall = if report.allowed { "PASS" } else { "FAIL" };
            println!("{overall} authorize");
            println!("repo_root: {}", report.paths.repo_root.display());
            println!("config_path: {}", report.paths.config_path.display());
            println!("old: {}", report.old);
            println!("new: {}", report.new);
            println!("ref: {}", report.reference);
            println!("target_commit: {}", report.target_commit);
            println!(
                "contract_fingerprint: {}",
                report.contract_fingerprint.to_hex()
            );

            if !report.matched_rules.is_empty() {
                println!("matched_rules:");
                for rule in &report.matched_rules {
                    println!("- {} -> {}", rule.pattern, rule.profile);
                }
            }

            if !report.required_profiles.is_empty() {
                println!("required_profiles: {}", report.required_profiles.join(", "));
            }

            for result in &report.profile_results {
                println!(
                    "- profile {} {}",
                    result.profile,
                    authorize_state_label(&result.state)
                );
            }

            println!("allowed: {}", report.allowed);
            Ok(report.ok())
        }
        Err(error) => {
            eprintln!("FAIL authorize [{}]", authorize_error_category(&error));
            eprintln!("{error}");
            Err(String::new())
        }
    }
}

fn local_policy_error_category(error: &LocalPolicyError) -> &'static str {
    match error {
        LocalPolicyError::Load(failure) => match &failure.error {
            LoadError::Discovery(_) => "discovery",
            LoadError::Parse(_) => "parse",
            LoadError::Validation(_) => "validation",
        },
        LocalPolicyError::GitCheckout { .. } | LocalPolicyError::GitWorktree { .. } => "git",
        LocalPolicyError::InvalidPattern { .. } => "pattern",
    }
}

fn local_policy_violation_message(violation: &LocalPolicyViolation) -> String {
    match violation {
        LocalPolicyViolation::ProtectedBranch {
            pattern,
            current_ref,
        } => format!(
            "local protected-branch policy blocks commits on {current_ref}; matched pattern {pattern:?}. Use a dedicated worktree branch, certify there, then merge the certified commit back."
        ),
        LocalPolicyViolation::DirtyPrimaryCheckout { dirty_paths } => format!(
            "local protected-checkout policy requires the primary checkout to stay clean; dirty path(s): {}. Use a dedicated worktree for implementation.",
            dirty_paths.join(", ")
        ),
    }
}

fn authorize_error_category(error: &AuthorizeError) -> &'static str {
    match error {
        AuthorizeError::Load(failure) => match &failure.error {
            LoadError::Discovery(_) => "discovery",
            LoadError::Parse(_) => "parse",
            LoadError::Validation(_) => "validation",
        },
        AuthorizeError::UnsupportedDeletion { .. } => "input",
        AuthorizeError::GitCommit { .. } => "git",
        AuthorizeError::Fingerprint { .. } => "fingerprint",
        AuthorizeError::Storage { .. } => "storage",
        AuthorizeError::InvalidPattern { .. } => "pattern",
    }
}

fn authorize_state_label(state: &repocert::enforcement::AuthorizeProfileState) -> &'static str {
    match state {
        repocert::enforcement::AuthorizeProfileState::Certified => "certified",
        repocert::enforcement::AuthorizeProfileState::StaleCommit => "stale_commit",
        repocert::enforcement::AuthorizeProfileState::StaleFingerprint => "stale_fingerprint",
        repocert::enforcement::AuthorizeProfileState::Uncertified => "uncertified",
    }
}
