use crate::check::error::CheckSelectionError;
use crate::check::types::CheckSelectionMode;
use crate::config::Contract;
use crate::contract::{
    EvaluationItem, EvaluationItemKind, SelectionError, build_profile_evaluation_plan,
    resolve_named_checks, resolve_profiles,
};

#[derive(Clone, Debug)]
pub(super) struct SelectionPlan {
    pub selection_mode: CheckSelectionMode,
    pub profiles: Vec<String>,
    pub checks: Vec<String>,
    pub items: Vec<EvaluationItem>,
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
        .map(|name| EvaluationItem {
            name: name.clone(),
            kind: EvaluationItemKind::Check,
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
    let mut checks = Vec::new();
    let mut items = Vec::new();

    for profile in &profiles {
        let plan = build_profile_evaluation_plan(contract, profile);
        for check in plan.checks {
            if !checks.contains(&check) {
                checks.push(check);
            }
        }
        for item in plan.items {
            if !items.iter().any(|existing: &EvaluationItem| {
                existing.name == item.name && existing.kind == item.kind
            }) {
                items.push(item);
            }
        }
    }

    Ok(SelectionPlan {
        selection_mode: CheckSelectionMode::Profiles,
        profiles,
        checks,
        items,
    })
}
