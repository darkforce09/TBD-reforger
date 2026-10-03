//! Purpose-selected native gameplay data and its Workbench policy tables.
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

use anyhow::Result;
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
