use crate::config::{CommandSpec, Contract};
use crate::selection::{
    SelectionError, collect_effective_fixers, resolve_named_fixers, resolve_profiles,
};

use super::error::FixSelectionError;
use super::types::FixSelectionMode;

#[derive(Clone, Debug)]
pub(super) struct PlannedFixer {
    pub name: String,
    pub command: CommandSpec,
}

#[derive(Clone, Debug)]
pub(super) struct FixPlan {
    pub selection_mode: FixSelectionMode,
    pub profile: Option<String>,
    pub fixers: Vec<String>,
    pub items: Vec<PlannedFixer>,
}

pub(super) fn build_fix_plan(
    contract: &Contract,
    profile: &Option<String>,
    names: &[String],
) -> Result<FixPlan, FixSelectionError> {
    if profile.is_some() && !names.is_empty() {
        return Err(FixSelectionError::from(
            SelectionError::ConflictingSelectors,
        ));
    }

    if !names.is_empty() {
        build_named_fixer_plan(contract, names)
    } else {
        build_profile_fixer_plan(contract, profile)
    }
}

fn build_named_fixer_plan(
    contract: &Contract,
    names: &[String],
) -> Result<FixPlan, FixSelectionError> {
    let selected_fixers = resolve_named_fixers(contract, names).map_err(FixSelectionError::from)?;
    let items = selected_fixers
        .iter()
        .map(|name| PlannedFixer {
            name: name.clone(),
            command: contract
                .fixers
                .get(name)
                .expect("named fixer should exist after validation")
                .command
                .clone(),
        })
        .collect();

    Ok(FixPlan {
        selection_mode: FixSelectionMode::Fixers,
        profile: None,
        fixers: selected_fixers,
        items,
    })
}

fn build_profile_fixer_plan(
    contract: &Contract,
    profile: &Option<String>,
) -> Result<FixPlan, FixSelectionError> {
    let requested = profile.iter().cloned().collect::<Vec<_>>();
    let profiles = resolve_profiles(contract, &requested).map_err(FixSelectionError::from)?;
    let profile_name = profiles
        .first()
        .expect("profile resolution should return one profile")
        .clone();
    let fixers = collect_effective_fixers(contract, &profiles);
    let items = fixers
        .iter()
        .map(|name| PlannedFixer {
            name: name.clone(),
            command: contract
                .fixers
                .get(name)
                .expect("selected fixer should exist")
                .command
                .clone(),
        })
        .collect();

    Ok(FixPlan {
        selection_mode: FixSelectionMode::Profile,
        profile: Some(profile_name),
        fixers,
        items,
    })
}
