//! The map asset checks the schema, terrain and BLAS verbs run, forwarded to the developer tools'
//! map verification.
//!
//! **Role:** one adapter per map asset check (the prefab BLAS library, the map-object goldens, the
//! terrain manifest, and the height, location, town and road labels): each finds the repository
//! root and returns the check's exit status unchanged.
//! **Position:** called by the xtask binary's `verify` and `schema` groups and by the
//! `schema-validate`, `verify-terrain` and `verify-terrain-strict` rows of
//! [`crate::task_runner::TASKS`]; over `developer_tools::map_verification`, so this crate never
//! names a map engine type.
//! **Signals & state:** none; each call reads the checkout afresh.
//! **Invariants:** a check's own failure text reaches the caller whole, its cause chain included
//! ([`crate::Error::MapAssetCheck`]).

use std::path::Path;

use repository_layout::find_repository_root;

use crate::error::{Error, Result};

/// `cargo xtask verify blas-manifest`: the prefab BLAS library against its manifest.
pub fn verify_blas_manifest(root: &Path) -> Result<u8> {
    developer_tools::map_verification::blas_manifest::verify_blas_manifest(root)
        .map_err(Error::map_asset_check)
}

/// `cargo xtask schema map-object-golden`: the map-object goldens.
pub fn map_object_golden() -> Result<u8> {
    developer_tools::map_verification::object_goldens::map_object_golden(&find_repository_root()?)
        .map_err(Error::map_asset_check)
}

/// `cargo xtask schema terrain-manifest`: the manifest of `terrain`.
pub fn terrain_manifest(terrain: &str) -> Result<u8> {
    developer_tools::map_verification::terrain_manifest::terrain_manifest(
        &find_repository_root()?,
        terrain,
    )
    .map_err(Error::map_asset_check)
}

/// `cargo xtask schema height-labels`: the spot heights of `terrain`.
pub fn height_labels(terrain: &str) -> Result<u8> {
    developer_tools::map_verification::labels::height_labels(&find_repository_root()?, terrain)
        .map_err(Error::map_asset_check)
}

/// `cargo xtask schema locations`: the location labels of `terrain`.
pub fn locations(terrain: &str) -> Result<u8> {
    developer_tools::map_verification::labels::locations(&find_repository_root()?, terrain)
        .map_err(Error::map_asset_check)
}

/// `cargo xtask schema terrain-alignment`: the labels of `terrain` against its elevation model,
/// every warning red when `strict`.
pub fn terrain_alignment(terrain: &str, strict: bool) -> Result<u8> {
    developer_tools::map_verification::labels::terrain_alignment(
        &find_repository_root()?,
        terrain,
        strict,
    )
    .map_err(Error::map_asset_check)
}

/// `cargo xtask schema town-labels`: the town labels of `terrain` at `zoom`.
pub fn town_labels(terrain: &str, zoom: f64) -> Result<u8> {
    developer_tools::map_verification::labels::town_labels(&find_repository_root()?, terrain, zoom)
        .map_err(Error::map_asset_check)
}

/// `cargo xtask schema road-names`: the road names of `terrain` at `zoom`.
pub fn road_names(terrain: &str, zoom: f64) -> Result<u8> {
    developer_tools::map_verification::labels::road_names(&find_repository_root()?, terrain, zoom)
        .map_err(Error::map_asset_check)
}
