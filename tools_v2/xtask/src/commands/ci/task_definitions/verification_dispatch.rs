//! Zero-argument adapters for the verifications a [`super::TASKS`] row runs in-process.
//!
//! A `Step::Xtask` holds a plain `fn() -> Result<u8>` pointer, which cannot capture; each adapter
//! here supplies the one argument its verification needs — the repository root, or the terrain the
//! CI lane always checks.

use super::*;

pub(super) fn x_height_labels() -> anyhow::Result<u8> {
    crate::verifications::map_assets::height_labels("everon")
}

pub(super) fn x_terrain_manifest() -> anyhow::Result<u8> {
    crate::verifications::map_assets::terrain_manifest("everon")
}

pub(super) fn x_terrain_alignment() -> anyhow::Result<u8> {
    crate::verifications::map_assets::terrain_alignment("everon", false)
}

pub(super) fn x_terrain_alignment_strict() -> anyhow::Result<u8> {
    crate::verifications::map_assets::terrain_alignment("everon", true)
}

pub(super) fn x_no_select_star() -> anyhow::Result<u8> {
    crate::verifications::database::sql_deserialization::verify_no_select_star(&find_repo_root()?)
}

pub(super) fn x_route_tags() -> anyhow::Result<u8> {
    crate::verifications::architecture::route_tags::verify_route_tags(&find_repo_root()?)
}

pub(super) fn x_engine_layers() -> anyhow::Result<u8> {
    crate::verifications::architecture::engine_layer_boundaries::verify_engine_layers(
        &find_repo_root()?,
    )
}

pub(super) fn x_staging_compose_paths() -> anyhow::Result<u8> {
    crate::verifications::deployment::staging_compose_paths::verify_staging_compose_paths(
        &find_repo_root()?,
    )
}

pub(super) fn x_mission_rest_size_limits() -> anyhow::Result<u8> {
    crate::verifications::mod_scripts::mission_rest_size_limits::verify_mission_rest_size_limits(
        &find_repo_root()?,
    )
}

pub(super) fn x_ci_schema_parity() -> anyhow::Result<u8> {
    crate::verifications::ci::schema_parity::verify_ci_schema_parity(&find_repo_root()?)
}
