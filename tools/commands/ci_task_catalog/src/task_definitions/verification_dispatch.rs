//! Zero-argument adapters for the verifications a [`super::TASKS`] row runs in-process.
//!
//! **Role:** one adapter per in-process verification step of the task table.
//! **Position:** pulled into `task_definitions.rs` by `#[path]`; over the database lane and the map
//! asset verification crate.
//! **Signals & state:** none; each call reads the checkout afresh.
//! **Invariants:** an adapter returns its verification's exit status unchanged.
//!
//! A `Step::Xtask` holds a plain `fn() -> Result<u8>` pointer, which cannot capture; each adapter
//! here supplies the arguments its verification needs: the repository root and the terrain the CI
//! lane always checks.

use repository_root::find_repository_root;

/// The terrain every map asset step of the task table checks.
const CHECKED_TERRAIN: &str = "everon";

pub(super) fn run_map_object_golden() -> crate::Result<u8> {
    Ok(map_asset_verification::object_goldens::map_object_golden(
        &find_repository_root()?,
    )?)
}

pub(super) fn run_height_labels() -> crate::Result<u8> {
    Ok(map_asset_verification::labels::height_labels(
        &find_repository_root()?,
        CHECKED_TERRAIN,
    )?)
}

pub(super) fn run_terrain_manifest() -> crate::Result<u8> {
    Ok(map_asset_verification::terrain_manifest::terrain_manifest(
        &find_repository_root()?,
        CHECKED_TERRAIN,
    )?)
}

pub(super) fn run_terrain_alignment() -> crate::Result<u8> {
    Ok(map_asset_verification::labels::terrain_alignment(
        &find_repository_root()?,
        CHECKED_TERRAIN,
        false,
    )?)
}

pub(super) fn run_terrain_alignment_strict() -> crate::Result<u8> {
    Ok(map_asset_verification::labels::terrain_alignment(
        &find_repository_root()?,
        CHECKED_TERRAIN,
        true,
    )?)
}

/// The database lane's complete integration suite in process: a fresh database per run and the
/// cleanup after every outcome (`cargo xtask db test-it`, which `mk rust-ci` also runs).
pub(crate) fn run_database_test_suite() -> crate::Result<u8> {
    Ok(database_operations::local_database::test_it::run_complete_suite()?)
}
