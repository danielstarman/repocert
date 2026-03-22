use crate::check::error::CheckSelectionError;
use crate::check::types::{CheckItemKind, CheckSelectionMode};
use crate::config::{CommandSpec, Contract, FixerSpec};
use crate::contract::{
    SelectionError, collect_effective_checks, collect_effective_fixers, resolve_named_checks,
    resolve_profiles,
};

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
        return Err(CheckSelectionError::from(
            SelectionError::ConflictingSelectors,
        ));
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
    let selected_checks =
        resolve_named_checks(contract, names).map_err(CheckSelectionError::from)?;

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
    let profiles =
        resolve_profiles(contract, requested_profiles).map_err(CheckSelectionError::from)?;
    let checks = collect_effective_checks(contract, &profiles);
    let fixer_names = collect_effective_fixers(contract, &profiles);

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
