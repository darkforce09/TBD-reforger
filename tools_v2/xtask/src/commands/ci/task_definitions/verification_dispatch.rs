//! Zero-argument adapters for the verifications a [`super::TASKS`] row runs in-process.
//!
//! A `Step::Xtask` holds a plain `fn() -> Result<u8>` pointer, which cannot capture; each adapter
//! here supplies the arguments its verification needs — the repository root, the terrain the CI
//! lane always checks, or, for a documentation gate, a request for the whole repository's
//! committed files.

use super::*;
use crate::verifications::documentation::GateRequest;

pub(super) fn run_height_labels() -> anyhow::Result<u8> {
    crate::verifications::map_assets::height_labels("everon")
}

pub(super) fn run_terrain_manifest() -> anyhow::Result<u8> {
    crate::verifications::map_assets::terrain_manifest("everon")
}

pub(super) fn run_terrain_alignment() -> anyhow::Result<u8> {
    crate::verifications::map_assets::terrain_alignment("everon", false)
}

pub(super) fn run_terrain_alignment_strict() -> anyhow::Result<u8> {
    crate::verifications::map_assets::terrain_alignment("everon", true)
}

pub(super) fn run_no_select_star() -> anyhow::Result<u8> {
    crate::verifications::database::sql_deserialization::verify_no_select_star(&find_repo_root()?)
}

pub(super) fn run_route_tags() -> anyhow::Result<u8> {
    crate::verifications::architecture::route_tags::verify_route_tags(&find_repo_root()?)
}

pub(super) fn run_engine_layers() -> anyhow::Result<u8> {
    crate::verifications::architecture::engine_layer_boundaries::verify_engine_layers(
        &find_repo_root()?,
    )
}

pub(super) fn run_staging_compose_paths() -> anyhow::Result<u8> {
    crate::verifications::deployment::staging_compose_paths::verify_staging_compose_paths(
        &find_repo_root()?,
    )
}

pub(super) fn run_mission_rest_size_limits() -> anyhow::Result<u8> {
    crate::verifications::mod_scripts::mission_rest_size_limits::verify_mission_rest_size_limits(
        &find_repo_root()?,
    )
}

pub(super) fn run_ci_schema_parity() -> anyhow::Result<u8> {
    crate::verifications::ci::schema_parity::verify_ci_schema_parity(&find_repo_root()?)
}

/// `cargo xtask verify readme-coverage` over the whole repository's committed files, the view
/// CI judges.
pub(super) fn run_readme_coverage() -> anyhow::Result<u8> {
    Ok(
        crate::verifications::documentation::readme_coverage::verify_readme_coverage(
            &find_repo_root()?,
            &GateRequest::default(),
        ),
    )
}

/// `cargo xtask verify link-check` over the whole repository's committed files, printing the
/// first breaks in full as the bare verb does.
pub(super) fn run_link_check() -> anyhow::Result<u8> {
    use crate::verifications::documentation::link_check::{BreakListing, verify_link_check};
    Ok(verify_link_check(
        &find_repo_root()?,
        &GateRequest::default(),
        BreakListing::First,
    ))
}

/// `cargo xtask verify markdown-placement` over the whole repository's committed files.
pub(super) fn run_markdown_placement() -> anyhow::Result<u8> {
    Ok(
        crate::verifications::documentation::markdown_placement::verify_markdown_placement(
            &find_repo_root()?,
            &GateRequest::default(),
        ),
    )
}
