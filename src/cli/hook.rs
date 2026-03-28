use std::io::{self, BufRead};
use std::process::ExitCode;

use repocert::config::LoadPaths;
use repocert::enforcement::{AuthorizeError, AuthorizeOptions, authorize_ref_update};
use repocert::hooks::GeneratedHook;
use repocert::local_policy::{LocalPolicyError, LocalPolicyViolation, check_local_commit_policy};

use super::app::{HookArgs, HookCommand, HookRunArgs};
use super::session::CommandRuntime;

pub(super) fn run(args: HookArgs) -> ExitCode {
    match args.command {
        HookCommand::Run(args) => run_hook(args),
    }
}

fn run_hook(args: HookRunArgs) -> ExitCode {
    let HookRunArgs {
        repo_root,
        config_path,
        hook,
        args,
    } = args;

    let hook = match GeneratedHook::parse(hook.as_str()) {
        Some(hook) => hook,
        other => {
            eprintln!("FAIL hook [input]");
            eprintln!(
                "unsupported generated hook {:?}",
                other.map(|hook| hook.as_str())
            );
            return ExitCode::from(1);
        }
    };

    let runtime = match CommandRuntime::load(
        "hook",
        super::app::OutputFormat::Human,
        repo_root,
        config_path,
    ) {
        Ok(runtime) => runtime,
        Err(code) => return code,
    };

    match hook {
        GeneratedHook::PreCommit | GeneratedHook::PreMergeCommit => run_local_policy_hook(&runtime),
        GeneratedHook::PrePush => run_pre_push_hook(&runtime),
        GeneratedHook::Update => run_update_hook(&runtime, &args),
    }
}

fn run_local_policy_hook(runtime: &CommandRuntime) -> ExitCode {
    match check_local_commit_policy(runtime.session()) {
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
        Err(error) => runtime.fail(
            local_policy_error_category(&error),
            &error.to_string(),
            None,
        ),
    }
}

fn run_pre_push_hook(runtime: &CommandRuntime) -> ExitCode {
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

        match authorize_with_runtime(runtime, remote_oid, local_oid, remote_ref) {
            Ok(true) => {}
            Ok(false) => return ExitCode::from(1),
            Err(code) => return code,
        }
    }

    ExitCode::SUCCESS
}

fn run_update_hook(runtime: &CommandRuntime, args: &[String]) -> ExitCode {
    let Some(reference) = args.first() else {
        eprintln!("FAIL hook [input]");
        eprintln!("update hook requires ref old new arguments");
        return ExitCode::from(1);
    };
    let Some(old) = args.get(1) else {
        eprintln!("FAIL hook [input]");
        eprintln!("update hook requires ref old new arguments");
        return ExitCode::from(1);
    };
    let Some(new) = args.get(2) else {
        eprintln!("FAIL hook [input]");
        eprintln!("update hook requires ref old new arguments");
        return ExitCode::from(1);
    };
    if args.len() != 3 {
        eprintln!("FAIL hook [input]");
        eprintln!("update hook requires ref old new arguments");
        return ExitCode::from(1);
    }

    match authorize_with_runtime(runtime, old, new, reference) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(1),
        Err(code) => code,
    }
}

fn authorize_with_runtime(
    runtime: &CommandRuntime,
    old: &str,
    new: &str,
    reference: &str,
) -> Result<bool, ExitCode> {
    let options = AuthorizeOptions {
        old: old.to_string(),
        new: new.to_string(),
        reference: reference.to_string(),
    };

    match authorize_ref_update(runtime.session(), options) {
        Ok(report) => {
            render_authorize_report(runtime.paths(), &report);
            Ok(report.ok())
        }
        Err(error) => Err(runtime.fail(authorize_error_category(&error), &error.to_string(), None)),
    }
}

fn render_authorize_report(paths: &LoadPaths, report: &repocert::enforcement::AuthorizeReport) {
    let overall = if report.allowed { "PASS" } else { "FAIL" };
    println!("{overall} authorize");
    println!("repo_root: {}", paths.repo_root.display());
    println!("config_path: {}", paths.config_path.display());
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
}

fn local_policy_error_category(error: &LocalPolicyError) -> &'static str {
    match error {
        LocalPolicyError::GitCheckout(_) | LocalPolicyError::GitWorktree(_) => "git",
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
        AuthorizeError::UnsupportedDeletion => "input",
        AuthorizeError::GitCommit(_) => "git",
        AuthorizeError::Fingerprint(_) => "fingerprint",
        AuthorizeError::Storage(_) => "storage",
        AuthorizeError::InvalidPattern { .. } => "pattern",
    }
}

fn authorize_state_label(state: &repocert::enforcement::AuthorizeProfileState) -> &'static str {
    match state {
        repocert::enforcement::AuthorizeProfileState::Certified => "certified",
        repocert::enforcement::AuthorizeProfileState::UntrustedSigner => "untrusted_signer",
        repocert::enforcement::AuthorizeProfileState::InvalidSignature => "invalid_signature",
        repocert::enforcement::AuthorizeProfileState::StaleCommit => "stale_commit",
        repocert::enforcement::AuthorizeProfileState::StaleFingerprint => "stale_fingerprint",
        repocert::enforcement::AuthorizeProfileState::Uncertified => "uncertified",
    }
}
