//! Wall extraction — the reason the split exists.
//!
//! `segments` (default): per y-slice interval observations clustered across ~12 slices per band,
//! with three independent roof-phantom vetoes (roof mask, persistence, stationarity). The dump's
//! continuous interval endpoints across many slices are strictly more information than the two
//! rasterized heights the live pipeline compared, which is what kept fragmenting the 2nd floor.
//!
//! `grid`: faithful port of the live ScanAxisX/Z cell marking + greedy RectsFromGrid +
//! MergeWallRects + two-height AND — the M2 equivalence bridge and the A/B fallback.

use std::collections::HashMap;

use super::pair::{ascending, pair_consuming};
use super::params::Params;
use super::types::{MassRect, PlanGrid, ScanMap, VerticalScan, VoxelDump, WallSeg};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Algo {
    Segments,
    Grid,
}

#[derive(Debug)]
pub struct BandWalls {
    pub walls: Vec<WallSeg>,
    /// (segment, is_exterior) — exterior classification is algorithm-specific.
    pub exterior: Vec<bool>,
    pub masses: Vec<MassRect>,
    /// Diagnostic: raw observation / rect count before merging.
    pub raw_count: usize,
}

/// One cluster's fate during extraction — the attribution record behind every wall (or gap)
/// the viewer shows. Emitted into the `--debug-dir` stages JSON.
#[derive(Debug, serde::Serialize)]
pub struct ClusterDebug {
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
pub struct BandDebug {
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
#[path = "../tests/walls/tests.rs"]
mod tests;

#[path = "wall_extraction/extract_band.rs"]
mod extract_band;
pub use extract_band::extract_band;

#[path = "wall_extraction/classify_exterior_flood.rs"]
mod classify_exterior_flood;
use classify_exterior_flood::classify_exterior_flood;
use classify_exterior_flood::grid_band;
pub use classify_exterior_flood::rects_from_grid;

#[cfg(test)]
use extract_band::cluster_columns;
