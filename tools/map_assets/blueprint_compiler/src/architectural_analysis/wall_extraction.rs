//! **Role:** wall extraction, the interpretation step that needs the whole dump.
//!
//! `segments` (default): per y-slice interval observations clustered across ~12 slices per band,
//! with three independent roof-phantom vetoes (roof mask, persistence, stationarity). The dump's
//! continuous interval endpoints across many slices are strictly more information than the two
//! rasterized heights the live pipeline compared, which is what kept fragmenting the 2nd floor.
//!
//! `grid`: faithful port of the live ScanAxisX/Z cell marking + greedy RectsFromGrid +
//! MergeWallRects + two-height AND — the M2 equivalence bridge and the A/B fallback.
//!
//! **Position:** called by `blueprint-from-voxels` per band; its walls and masses feed the blueprint assembly.
//! **Signals & state:** none; pure functions; the `--debug-dir` records are returned, not written.
//! **Invariants:** both extractors read the same band and tunables and return walls in the dump's normalized frame.

use std::collections::HashMap;

use crate::architectural_analysis::collision_face_pairing::{
    ascending_closing_faces, pair_faces_consuming,
};
use crate::voxel_processing::analysis_parameters::AnalysisParameters;
use crate::voxel_processing::voxel_types::{
    MassRectangle, PlanGrid, ScanMap, VerticalScan, VoxelDump, WallSegment,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WallAlgorithm {
    Segments,
    Grid,
}

#[derive(Debug)]
pub(crate) struct BandWalls {
    pub walls: Vec<WallSegment>,
    /// (segment, is_exterior) — exterior classification is algorithm-specific.
    pub exterior: Vec<bool>,
    pub masses: Vec<MassRectangle>,
    /// Diagnostic: raw observation / rect count before merging.
    pub raw_count: usize,
}

/// One cluster's fate during extraction — the attribution record behind every wall (or gap)
/// the viewer shows. Emitted into the `--debug-dir` stages JSON.
#[derive(Debug, serde::Serialize)]
pub(crate) struct ClusterDebug {
    /// "z-running" (constant x) or "x-running" (constant z).
    pub axis: &'static str,
    /// Fixed lattice index (iz for z-running, ix for x-running).
    pub fixed: usize,
    /// Cluster centerline along the interval axis, normalized meters.
    pub center: f64,
    pub thick: f64,
    pub rows_seen: usize,
    /// Roof-clipped slice rows available at this column (the persistence denominator).
    pub rows_avail: usize,
    /// Rows required: max(min_persist_rows, ceil(persistence_frac × rows_avail)).
    pub need: usize,
    pub drift: f64,
    /// "accepted" | "persistence" | "drift".
    pub verdict: &'static str,
}

/// Per-band extraction attribution for `--debug-dir` (never allocated on normal runs).
#[derive(Debug, Default, serde::Serialize)]
pub(crate) struct BandDebug {
    pub clusters: Vec<ClusterDebug>,
    pub graze_vetoed: usize,
    pub mass_cells: usize,
}

// ── shared helpers ──────────────────────────────────────────────────────────────────────────────

// ── segments algorithm ──────────────────────────────────────────────────────────────────────────

struct Obs {
    row: usize,
    center: f64,
    thick: f64,
}

// ── grid algorithm (live port) ──────────────────────────────────────────────────────────────────

#[cfg(test)]
#[path = "tests/wall_extraction_tests.rs"]
mod tests;

#[path = "wall_extraction/segment_wall_extractor.rs"]
mod segment_wall_extractor;
pub(crate) use segment_wall_extractor::extract_band_walls;

#[path = "wall_extraction/grid_wall_extractor.rs"]
mod grid_wall_extractor;
use grid_wall_extractor::classify_exterior_by_flood_fill;
use grid_wall_extractor::grid_band_walls;
pub(crate) use grid_wall_extractor::rectangles_from_grid;

#[cfg(test)]
use segment_wall_extractor::cluster_columns;
