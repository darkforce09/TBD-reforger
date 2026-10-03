//! Purpose-selected native gameplay data and its Workbench policy tables.
//!
//! **Role:** `project_command` projects a full diagnostic Workbench generation through the
//! gameplay selection policy into a gameplay generation; `generate_command` writes (or with
//! `--check` compares) the export addon's selection tables generated from that policy; `validate`
//! checks a gameplay generation.
//! **Position:** called by [`crate::mod_dispatch`] for `mod project-equipment-gameplay` and
//! `mod generate-equipment-gameplay-policy`, and by [`crate::equipment_vehicle_export`] for a
//! generation whose `document_type` is `gameplay_generation`; the policy lives under
//! `contracts/rules/equipment-gameplay/`.
//! **Signals & state:** none beyond the files the commands write.
//! **Invariants:** the policy is the one authority on which fields and classes a projection keeps;
//! the generated tables depend only on it.

mod model;
mod policy;
mod policy_codegen;
mod projection;
mod resource_projection;
mod validation;
mod validation_resource;

pub(super) use validation::validate;

#[cfg(test)]
#[path = "tests/catalog.rs"]
mod tests;

use crate::Result;
use std::path::Path;

pub(crate) fn project_command(input: &Path, output: &Path) -> Result<u8> {
    let root = repository_layout::find_repository_root()?;
    projection::project(&root, input, output)?;
    Ok(0)
}

pub(crate) fn generate_command(check: bool) -> Result<u8> {
    let root = repository_layout::find_repository_root()?;
    policy_codegen::generate(&root, check)?;
    Ok(0)
}
