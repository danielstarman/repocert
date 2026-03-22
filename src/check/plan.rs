use std::collections::BTreeSet;

use crate::check::error::CheckSelectionError;
use crate::check::types::{CheckItemKind, CheckSelectionMode};
use crate::config::{CommandSpec, Contract, FixerSpec};

#[derive(Clone, Debug)]
pub(super) struct PlannedItem {
    pub name: String,
    pub kind: CheckItemKind,
    pub command: CommandSpec,
}

#[derive(Clone, Debug)]
pub(super) struct SelectionPlan {
    pub selection_mode: CheckSelectionMode,
    pub profiles: Vec<String>,
    pub checks: Vec<String>,
    pub items: Vec<PlannedItem>,
}

pub(super) fn build_selection_plan(
    contract: &Contract,
    profiles: &[String],
    names: &[String],
) -> Result<SelectionPlan, CheckSelectionError> {
    let has_profiles = !profiles.is_empty();
    let has_names = !names.is_empty();

    if has_profiles && has_names {
        return Err(CheckSelectionError::ConflictingSelectors);
    }

    if has_names {
        build_named_check_plan(contract, names)
    } else {
        build_profile_plan(contract, profiles)
    }
}

fn build_named_check_plan(
    contract: &Contract,
    names: &[String],
) -> Result<SelectionPlan, CheckSelectionError> {
    let selected_checks = dedupe_preserving_order(names);
    let unknown = selected_checks
        .iter()
        .filter(|name| !contract.checks.contains_key(name.as_str()))
        .cloned()
        .collect::<Vec<_>>();

    if !unknown.is_empty() {
        return Err(CheckSelectionError::UnknownChecks(unknown.join(", ")));
    }

    let items = selected_checks
        .iter()
        .map(|name| PlannedItem {
            name: name.clone(),
            kind: CheckItemKind::Check,
            command: contract
                .checks
                .get(name)
                .expect("named check should exist after validation")
                .clone(),
        })
        .collect();

    Ok(SelectionPlan {
        selection_mode: CheckSelectionMode::Checks,
        profiles: Vec::new(),
        checks: selected_checks,
        items,
    })
}

fn build_profile_plan(
    contract: &Contract,
    requested_profiles: &[String],
) -> Result<SelectionPlan, CheckSelectionError> {
    let profiles = if requested_profiles.is_empty() {
        vec![
            contract
                .default_profile
                .clone()
                .ok_or(CheckSelectionError::NoDefaultProfile)?,
        ]
    } else {
        dedupe_preserving_order(requested_profiles)
    };

    let unknown = profiles
        .iter()
        .filter(|name| !contract.profiles.contains_key(name.as_str()))
        .cloned()
        .collect::<Vec<_>>();

    if !unknown.is_empty() {
        return Err(CheckSelectionError::UnknownProfiles(unknown.join(", ")));
    }

    let mut checks = Vec::new();
    let mut seen_checks = BTreeSet::new();
    let mut fixer_names = Vec::new();
    let mut seen_fixers = BTreeSet::new();

    for profile_name in &profiles {
        let profile = contract
            .profiles
            .get(profile_name)
            .expect("profile should exist after validation");

        for check in &profile.effective_checks {
            if seen_checks.insert(check.clone()) {
                checks.push(check.clone());
            }
        }

        for fixer in &profile.effective_fixers {
            if seen_fixers.insert(fixer.clone()) {
                fixer_names.push(fixer.clone());
            }
        }
    }

    let mut items = checks
        .iter()
        .map(|name| PlannedItem {
            name: name.clone(),
            kind: CheckItemKind::Check,
            command: contract
                .checks
                .get(name)
                .expect("selected check should exist")
                .clone(),
        })
        .collect::<Vec<_>>();

    items.extend(
        fixer_names
            .iter()
            .map(|name| build_probe_item(name, contract)),
    );

    Ok(SelectionPlan {
        selection_mode: CheckSelectionMode::Profiles,
        profiles,
        checks,
        items,
    })
}

fn build_probe_item(name: &str, contract: &Contract) -> PlannedItem {
    let fixer = contract
        .fixers
        .get(name)
        .expect("selected fixer should exist after validation");

    PlannedItem {
        name: name.to_string(),
        kind: CheckItemKind::FixerProbe,
        command: probe_command_spec(fixer),
    }
}

fn probe_command_spec(fixer: &FixerSpec) -> CommandSpec {
    CommandSpec {
        argv: fixer
            .probe_argv
            .clone()
            .expect("profile-selected fixer probes are validated"),
        env: fixer.command.env.clone(),
        timeout_ms: fixer.probe_timeout_ms.or(fixer.command.timeout_ms),
    }
}

fn dedupe_preserving_order(names: &[String]) -> Vec<String> {
    let mut ordered = Vec::new();
    let mut seen = BTreeSet::new();

    for name in names {
        if seen.insert(name.clone()) {
            ordered.push(name.clone());
        }
    }

    ordered
}
