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

pub(super) fn x_t438() -> anyhow::Result<u8> {
    crate::verifications::deployment::staging_compose_paths::verify_t438(&find_repo_root()?)
}

pub(super) fn x_t456() -> anyhow::Result<u8> {
    crate::verifications::mod_scripts::mission_rest_size_limits::verify_t456(&find_repo_root()?)
}

pub(super) fn x_t468() -> anyhow::Result<u8> {
    crate::verifications::ci::schema_parity::verify_t468(&find_repo_root()?)
}
