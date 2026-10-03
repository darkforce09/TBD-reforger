//! The served map-asset trees and the export scratch beside them.
//!
//! **Role:** the built-in terrain datasets and the glyph atlases the map client reaches under
//! `/map-assets`, and the uncommitted per-island export scratch, each as a function that joins its
//! location onto a checkout root the caller passes.
//! **Position:** the `developer_tools` export, raster, verification and blueprint pipelines write
//! and read these trees; `xtask` verifies them, indexes the tiles and stages exports into the
//! scratch.
//! **Signals & state:** none; pure path joins.
//! **Invariants:** terrains and glyphs are sibling folders under `assets/`; the export scratch sits
//! outside the served terrain tree, so serving the terrains never publishes export intermediates.

use std::path::{Path, PathBuf};

/// Root of the built-in terrain datasets, relative to a checkout root; served at `/map-assets`.
pub const TERRAIN_ASSETS_DIR: &str = "assets/terrains";

/// Tactical symbology atlases and marker sources, relative to a checkout root; served at
/// `/map-assets/glyphs`.
pub const GLYPH_ASSETS_DIR: &str = "assets/glyphs";

/// Local, uncommitted export scratch, relative to a checkout root, one folder per island.
pub const MAP_SCRATCH_DIR: &str = "assets/scratch";

/// [`TERRAIN_ASSETS_DIR`] under a checkout root.
pub fn terrain_assets_dir(root: &Path) -> PathBuf {
    root.join(TERRAIN_ASSETS_DIR)
}

/// One island's dataset directory.
pub fn terrain_dir(root: &Path, terrain: &str) -> PathBuf {
    terrain_assets_dir(root).join(terrain)
}

/// Catalog of every registered platform terrain.
pub fn terrain_registry_path(root: &Path) -> PathBuf {
    terrain_assets_dir(root).join("terrain-registry.json")
}

/// One island's manifest: world bounds, DEM scaling, tile and object paths.
pub fn terrain_manifest_path(root: &Path, terrain: &str) -> PathBuf {
    terrain_dir(root, terrain).join("manifest.json")
}

/// [`GLYPH_ASSETS_DIR`] under a checkout root.
pub fn glyph_assets_dir(root: &Path) -> PathBuf {
    root.join(GLYPH_ASSETS_DIR)
}

/// The glyph registry and its UV coordinate catalog.
pub fn glyph_manifest_path(root: &Path) -> PathBuf {
    glyph_assets_dir(root).join("manifest.json")
}

/// Local, uncommitted export scratch for one island: stitched orthophotos, masks, spikes.
///
/// Ignored by git. Pipeline stages write intermediates here; nothing downstream of an export may
/// read from it, because a fresh clone does not have it.
pub fn map_scratch_dir(root: &Path, terrain: &str) -> PathBuf {
    root.join(MAP_SCRATCH_DIR).join(terrain)
}

#[cfg(test)]
#[path = "tests/map_assets_tests.rs"]
mod tests;
