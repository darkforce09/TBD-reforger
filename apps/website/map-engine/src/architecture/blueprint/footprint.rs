//! Role: footprint.
//! Position: `architecture/blueprint` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::architecture::blueprint::structure::BBox2D;
use serde::Deserialize;
use serde::Serialize;

/// Vertical elevation and roof structure metadata for macro line-of-sight.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerticalProfile {
    /// Pivot elevation offset m.
    pub pivot_elevation_offset_m: f64,

    /// Foundation skirt depth m.
    pub foundation_skirt_depth_m: f64,

    /// Total height m.
    pub total_height_m: f64,

    /// Eave height m.
    pub eave_height_m: f64,

    /// Ridge height m.
    pub ridge_height_m: f64,

    /// Chimney height m.
    pub chimney_height_m: Option<f64>,

    /// Roof type.
    pub roof_type: String,
}

/// Overall footprint.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OverallFootprint {
    /// Polygon2 d.
    pub polygon2_d: Vec<[f64; 2]>,

    /// Bounding box2 d.
    pub bounding_box2_d: BBox2D,

    /// Footprint sq m.
    pub footprint_sq_m: f64,
}

/// Downsampled top-surface heightfield in the building's local frame (y up). Optional — the roof view paints it, and a structural hit near its surface is attributed to the roof.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoofGrid {
    /// Local `[x, z]` of the low corner of cell (0, 0).
    pub origin: [f64; 2],

    /// Cell size m.
    pub cell_size_m: f64,

    /// Nx.
    pub nx: usize,

    /// Nz.
    pub nz: usize,

    /// Row-major `ix * nz + iz`; `None` = no coverage (outside the roof silhouette).
    pub heights_m: Vec<Option<f64>>,
}

impl RoofGrid {
    /// Shape sanity — an inconsistent grid is skipped by consumers (attribution falls through to [`LosHitKind::Solid`]; the roof view paints nothing).
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.cell_size_m > 0.0
            && self.nx > 0
            && self.nz > 0
            && self.heights_m.len() == self.nx * self.nz
    }
}

impl RoofGrid {
    /// Nearest-cell surface height at local `(x, z)` — deliberately NOT interpolated: interpolation across `None` cells is undefined and would smear chimneys and dormer pits into their neighbors.
    #[must_use]
    pub fn height_at(&self, x: f64, z: f64) -> Option<f64> {
        let fx = (x - self.origin[0]) / self.cell_size_m;
        let fz = (z - self.origin[1]) / self.cell_size_m;
        if fx < 0.0 || fz < 0.0 {
            return None;
        }
        let (ix, iz) = (fx as usize, fz as usize);
        if ix >= self.nx || iz >= self.nz {
            return None;
        }
        self.heights_m[ix * self.nz + iz]
    }
}

/// Downsampled walkable-floor heightfield for ONE level, in the building's local frame (y up). Same shape as [`RoofGrid`] but different semantics: a covered cell means "there is floor slab here at this height" — the verbatim per-cell product of the plate scan, so partial mezzanines and double-height voids render exactly as measured. Optional — absent on pre-plate blueprints and on levels with no real slab (attic).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlateGrid {
    /// Local `[x, z]` of the low corner of cell (0, 0).
    pub origin: [f64; 2],

    /// Cell size m.
    pub cell_size_m: f64,

    /// Nx.
    pub nx: usize,

    /// Nz.
    pub nz: usize,

    /// Row-major `ix * nz + iz`; `None` = no floor here; `Some(y)` = local slab-surface height.
    pub heights_m: Vec<Option<f64>>,
}

impl PlateGrid {
    /// Shape sanity — an inconsistent grid is ignored by consumers.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.cell_size_m > 0.0
            && self.nx > 0
            && self.nz > 0
            && self.heights_m.len() == self.nx * self.nz
    }
}

impl PlateGrid {
    /// Nearest-cell floor height at local `(x, z)`; `None` outside the grid or over a void.
    #[must_use]
    pub fn height_at(&self, x: f64, z: f64) -> Option<f64> {
        let fx = (x - self.origin[0]) / self.cell_size_m;
        let fz = (z - self.origin[1]) / self.cell_size_m;
        if fx < 0.0 || fz < 0.0 {
            return None;
        }
        let (ix, iz) = (fx as usize, fz as usize);
        if ix >= self.nx || iz >= self.nz {
            return None;
        }
        self.heights_m[ix * self.nz + iz]
    }
}
