//! The repository locations only the map export and raster pipelines name.
//!
//! **Role:** the committed density fixtures the export re-densifies from, the per-terrain export
//! operation log and type inventory, and the committed decision records of the inland-water,
//! aerial-orthophoto and cartographic lanes, each as a function of a checkout root the caller
//! passes.
//! **Position:** `world_export_pipeline` and `map_raster_pipeline` read and write these; the
//! locations more than one tool names (the contract tree, the terrain and glyph trees, the export
//! scratch) come from the `repository_layout` crate.
//! **Signals & state:** none; constants and pure path joins.
//! **Invariants:** every location lies under the checkout root it is given; the operation logs and
//! decision records lie inside the agent artifact tree.

use std::path::{Path, PathBuf};

/* ─────────────────────────────── density fixtures ─────────────────────────────── */

/// Committed forest-density fixtures, read when re-densifying without a Workbench export.
pub fn density_fixtures_dir(root: &Path) -> PathBuf {
    ::repository_layout::map_fixtures_dir(root).join("density")
}

/* ─────────────────────────────── export operation logs ─────────────────────────────── */

// The world-export pipeline writes its per-terrain operation log and type inventory into the
// agent artifact tree, `repository_layout::ARTIFACTS_DIR`, where its verifiers read them back.

/// One terrain's export operation log: every stage that ran, with what it produced.
pub fn export_operations_log(root: &Path, terrain: &str) -> PathBuf {
    root.join(::repository_layout::ARTIFACTS_DIR)
        .join(format!("map_export_{terrain}.json"))
}

/// One terrain's object type inventory: every world-object type the export saw, and its census
/// status.
pub fn object_type_inventory(root: &Path, terrain: &str) -> PathBuf {
    root.join(::repository_layout::ARTIFACTS_DIR)
        .join(format!("type_inventory_{terrain}.json"))
}

/* ─────────────────────────────── analysis artifacts ─────────────────────────────── */

/// Committed decision records for the inland-water classifier: the water and source spikes the
/// classifier writes and the refine spike it compares the current run against.
pub const INLAND_WATER_ARTIFACTS_DIR: &str = ".ai/artifacts/inland_water";

/// Committed decision records for the aerial orthophoto lane: the seam analysis the stitcher's
/// seam verifier writes.
pub const AERIAL_ORTHOPHOTO_ARTIFACTS_DIR: &str = ".ai/artifacts/aerial_orthophoto";

/// Committed decision records for the cartographic lane: the land-cover source spike the mask
/// builder cites in the ortho metadata it emits.
pub const CARTOGRAPHIC_RENDERING_ARTIFACTS_DIR: &str = ".ai/artifacts/cartographic_rendering";

/// [`INLAND_WATER_ARTIFACTS_DIR`] under a checkout root.
pub fn inland_water_artifacts_dir(root: &Path) -> PathBuf {
    root.join(INLAND_WATER_ARTIFACTS_DIR)
}

/// [`AERIAL_ORTHOPHOTO_ARTIFACTS_DIR`] under a checkout root.
pub fn aerial_orthophoto_artifacts_dir(root: &Path) -> PathBuf {
    root.join(AERIAL_ORTHOPHOTO_ARTIFACTS_DIR)
}

/// [`CARTOGRAPHIC_RENDERING_ARTIFACTS_DIR`] under a checkout root.
pub fn cartographic_rendering_artifacts_dir(root: &Path) -> PathBuf {
    root.join(CARTOGRAPHIC_RENDERING_ARTIFACTS_DIR)
}

#[cfg(test)]
#[path = "tests/map_pipeline_layout.rs"]
mod tests;
