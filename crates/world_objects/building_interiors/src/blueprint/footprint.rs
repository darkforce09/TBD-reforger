//! A blueprint's footprint, vertical profile, roof grid and floor plate.
//!
//! **Role:** the plan footprint and box ([`OverallFootprint`]), the heights ([`VerticalProfile`]),
//! the roof height grid sampled by the attribution ([`RoofGrid`]) and a level's floor plate
//! ([`PlateGrid`]).
//! **Position:** held by [`crate::blueprint::structure`]; read by the sight-line attribution and the
//! section drawings.
//! **Signals & state:** none; plain data.
//! **Invariants:** a roof grid lookup off the grid or on an empty cell is `None`.

use crate::blueprint::structure::BBox2D;
use serde::Deserialize;
use serde::Serialize;

/// Vertical elevation and roof structure metadata for macro line-of-sight.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerticalProfile {
    /// Vertical offset of the prefab pivot, metres; the scan pipeline writes 0.
    pub pivot_elevation_offset_m: f64,

    /// Depth the mesh reaches below the pivot (local y 0), metres; at least 0.
    pub foundation_skirt_depth_m: f64,

    /// Top of the mesh above the pivot (local y), metres.
    pub total_height_m: f64,

    /// Local height of the eave, metres; the roof view shades from eave to ridge.
    pub eave_height_m: f64,

    /// Local height of the roof ridge, metres.
    pub ridge_height_m: f64,

    /// Local height of the chimney top, metres; `None` without a chimney (JSON key omitted).
    pub chimney_height_m: Option<f64>,

    /// Free-form roof shape name (`gable_with_dormers`, `with_chimney`, `scanned`).
    pub roof_type: String,
}

/// Plan footprint of the whole building in its local frame: outline, box and area.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OverallFootprint {
    /// Outline ring of `[x, z]` points in the local frame, metres (the ground level's
    /// traced footprint); JSON `polygon2D`.
    pub polygon2_d: Vec<[f64; 2]>,

    /// Axis-aligned box around `polygon2_d` ([`BBox2D`]); JSON `boundingBox2D`.
    pub bounding_box2_d: BBox2D,

    /// Covered ground floor area, square metres.
    pub footprint_sq_m: f64,
}

/// Downsampled top-surface heightfield in the building's local frame (y up). Optional — the roof view paints it, and a structural hit near its surface is attributed to the roof.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoofGrid {
    /// Local `[x, z]` of the low corner of cell (0, 0).
    pub origin: [f64; 2],

    /// Edge length of one square grid cell, metres; positive on a valid grid.
    pub cell_size_m: f64,

    /// Number of cells along local X.
    pub nx: usize,

    /// Number of cells along local Z.
    pub nz: usize,

    /// Row-major `ix * nz + iz`; `None` = no coverage (outside the roof silhouette).
    pub heights_m: Vec<Option<f64>>,
}

impl RoofGrid {
    /// Shape sanity — an inconsistent grid is skipped by consumers (attribution falls through to [`crate::blueprint::sight_line::LosHitKind::Solid`]; the roof view paints nothing).
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

    /// Edge length of one square grid cell, metres; positive on a valid grid.
    pub cell_size_m: f64,

    /// Number of cells along local X.
    pub nx: usize,

    /// Number of cells along local Z.
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
