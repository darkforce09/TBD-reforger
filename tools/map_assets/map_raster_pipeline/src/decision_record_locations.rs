//! The repository locations only the map raster pipeline names.
//!
//! **Role:** the committed decision records of the inland-water, aerial-orthophoto and
//! cartographic lanes; the two a lane writes also as a function of a checkout root the caller
//! passes.
//! **Position:** `map_raster_pipeline` reads and writes these; the locations more than one tool
//! names (the contract tree, the terrain and glyph trees, the export scratch) come from the
//! `repository_layout` crate, and the world export's own locations from
//! `world_export_pipeline::export_locations`.
//! **Signals & state:** none; constants and pure path joins.
//! **Invariants:** every location lies under the checkout root it is given; the decision records
//! lie inside the agent artifact tree.

use std::path::{Path, PathBuf};

/* ─────────────────────────────── analysis artifacts ─────────────────────────────── */

/// Committed decision records for the inland-water classifier: the water and source spikes the
/// classifier writes and the refine spike it compares the current run against.
pub(crate) const INLAND_WATER_ARTIFACTS_DIR: &str = ".ai/artifacts/inland_water";

/// Committed decision records for the aerial orthophoto lane: the seam analysis the stitcher's
/// seam verifier writes.
pub(crate) const AERIAL_ORTHOPHOTO_ARTIFACTS_DIR: &str = ".ai/artifacts/aerial_orthophoto";

/// Committed decision records for the cartographic lane: the land-cover source spike the mask
/// builder cites in the ortho metadata it emits.
pub(crate) const CARTOGRAPHIC_RENDERING_ARTIFACTS_DIR: &str =
    ".ai/artifacts/cartographic_rendering";

/// [`INLAND_WATER_ARTIFACTS_DIR`] under a checkout root.
pub(crate) fn inland_water_artifacts_dir(root: &Path) -> PathBuf {
    root.join(INLAND_WATER_ARTIFACTS_DIR)
}

/// [`AERIAL_ORTHOPHOTO_ARTIFACTS_DIR`] under a checkout root.
pub(crate) fn aerial_orthophoto_artifacts_dir(root: &Path) -> PathBuf {
    root.join(AERIAL_ORTHOPHOTO_ARTIFACTS_DIR)
}
