//! **Role:** In-memory model of a `tbd-voxel-dump/1` file (see TBD_BuildingVoxelDump.c for the wire
//! format). All coordinates are DUMP-NORMALIZED: meters from `meta.origin` on each axis. The
//! emitter adds `origin` back to return to the building's local frame (the blueprint contract).
//!
//! **Position:** built by [`crate::voxel_processing::dump_parser`] and the generators; read by every interpretation stage.
//! **Signals & state:** none; plain data.
//! **Invariants:** coordinates are dump-normalized: meters from `meta.origin` on each axis.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

pub(crate) const DUMP_VERSION: &str = "tbd-voxel-dump/1";

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DumpMetadata {
    pub v: String,
    pub slug: String,
    pub resource: String,
    /// Padded local scan minimum — the normalization origin.
    pub origin: [f64; 3],
    pub cell: f64,
    pub dims: [usize; 3],
    pub span: [f64; 3],
    /// Unpadded entity bounds (profile numbers).
    pub bbox_min: [f64; 3],
    pub bbox_max: [f64; 3],
    pub root_yaw_deg: f64,
    pub excluded: ExcludedCounts,
    #[serde(default)]
    pub tick: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct ExcludedCounts {
    pub doors: usize,
    pub glass: usize,
    pub furniture: usize,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FurnitureRecord {
    pub name: String,
    pub res: String,
    /// Local-frame position (NOT normalized — the dumper writes WorldToLocal directly).
    pub pos: [f64; 3],
    pub world_yaw_deg: f64,
    pub size: [f64; 3],
    /// Wire field, unread yet: level assignment follows the live extractor (entity origin y);
    /// reserved for basement/stairs assignment when those land.
    #[allow(dead_code)]
    pub bounds_min_y: f64,
}

/// One march direction's scanlines, keyed by the two fixed lattice indices (j, k).
/// Axis mapping (dumper contract): x → (iy, iz), y → (ix, iz), z → (ix, iy).
pub(crate) type ScanMap = HashMap<(usize, usize), Vec<f64>>;

#[derive(Debug, Default)]
pub(crate) struct VoxelDump {
    pub meta: Option<DumpMetadata>,
    pub x_pos: ScanMap,
    pub x_neg: ScanMap,
    pub y_down: ScanMap,
    pub y_up: ScanMap,
    pub z_pos: ScanMap,
    pub z_neg: ScanMap,
    pub furniture: Vec<FurnitureRecord>,
    /// Scanlines that hit the dumper's MAX_MARCH_HITS cap (logged, treated as-is).
    pub truncated: usize,
}

impl VoxelDump {
    pub(crate) fn metadata(&self) -> &DumpMetadata {
        self.meta.as_ref().expect("parse_dump guarantees meta")
    }
}

/// A paired solid interval along one scanline, normalized axis coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct SolidInterval {
    pub a: f64,
    pub b: f64,
    pub one_sided: bool,
}

impl SolidInterval {
    pub(crate) fn len(&self) -> f64 {
        self.b - self.a
    }
    pub(crate) fn midpoint(&self) -> f64 {
        (self.a + self.b) * 0.5
    }
}

/// Row-major plan grid (index `ix * nz + iz`, matching the live scanner's layout).
#[derive(Debug, Clone)]
pub(crate) struct PlanGrid {
    pub nx: usize,
    pub nz: usize,
    pub cells: Vec<bool>,
}

impl PlanGrid {
    pub(crate) fn new(nx: usize, nz: usize) -> Self {
        Self {
            nx,
            nz,
            cells: vec![false; nx * nz],
        }
    }
    #[inline]
    pub(crate) fn get(&self, ix: usize, iz: usize) -> bool {
        self.cells[ix * self.nz + iz]
    }
    #[inline]
    pub(crate) fn set(&mut self, ix: usize, iz: usize, v: bool) {
        self.cells[ix * self.nz + iz] = v;
    }
    pub(crate) fn count(&self) -> usize {
        self.cells.iter().filter(|c| **c).count()
    }
}

/// Vertical-analysis products consumed by every later stage.
#[derive(Debug)]
pub(crate) struct VerticalScan {
    /// Slab elevations (normalized y), ascending.
    pub slabs: Vec<f64>,
    /// Floor slabs after the live (min_floor_y, eave - clearance) filter; never empty.
    pub floors: Vec<f64>,
    pub eave: f64,
    pub ridge: f64,
    /// Only present when a spike clears ridge + chimney_margin.
    pub chimney: Option<f64>,
    /// Top surface height per plan cell (first down-entry), None where nothing was hit.
    pub top: Vec<Option<f64>>,
    /// |gradient| of `top` (central difference / cell pitch), 0.0 where undefined.
    pub top_slope: Vec<f64>,
    pub nx: usize,
    pub nz: usize,
}

impl VerticalScan {
    #[inline]
    pub(crate) fn top_at(&self, ix: usize, iz: usize) -> Option<f64> {
        self.top[ix * self.nz + iz]
    }
    #[inline]
    pub(crate) fn slope_at(&self, ix: usize, iz: usize) -> f64 {
        self.top_slope[ix * self.nz + iz]
    }
}

/// One extracted wall run in normalized plan coordinates.
#[derive(Debug, Clone)]
pub(crate) struct WallSegment {
    pub start: [f64; 2],
    pub end: [f64; 2],
    pub thickness: f64,
}

/// An interior mass rect (stair block, chimney shaft) — full-cover furniture in the blueprint.
#[derive(Debug, Clone)]
pub(crate) struct MassRectangle {
    /// [min_x, min_z, max_x, max_z], normalized.
    pub rect: [f64; 4],
}
