//! **Role:** the floor plate. The plate (a short window of downward entries around the slab) is
//! the level's real walkable footprint: a mezzanine void or a floor-to-ridge room has no plate
//! cells and stays void. Boundary tracing lives in
//! [`crate::architectural_analysis::polygon_rings`].
//!
//! **Position:** called by `blueprint-from-voxels` per floor band; its grid feeds [`crate::architectural_analysis::polygon_rings`].
//! **Signals & state:** none; pure functions.
//! **Invariants:** a column with no downward entry inside the band window is never floor.

use crate::voxel_processing::analysis_parameters::Params;
use crate::voxel_processing::voxel_types::{PlanGrid, ScanMap};

/// Occupancy + per-cell floor height: `heights[ix * nz + iz]` is the TOPMOST y_down entry
/// inside the slab window (normalized frame), `None` where nothing landed in-window.
pub(crate) fn floor_plate(
    y_down: &ScanMap,
    nx: usize,
    nz: usize,
    slab_y: f64,
    p: &Params,
) -> (PlanGrid, Vec<Option<f64>>) {
    let mut grid = PlanGrid::new(nx, nz);
    let mut heights: Vec<Option<f64>> = vec![None; nx * nz];
    let (lo, hi) = (slab_y - p.plate_below_m, slab_y + p.plate_above_m);
    for (&(ix, iz), entries) in y_down {
        if ix >= nx || iz >= nz {
            continue;
        }
        let top = entries
            .iter()
            .copied()
            .filter(|&y| y >= lo && y <= hi)
            .fold(f64::NEG_INFINITY, f64::max);
        if top > f64::NEG_INFINITY {
            grid.set(ix, iz, true);
            heights[ix * nz + iz] = Some(top);
        }
    }
    (grid, heights)
}

#[cfg(test)]
#[path = "tests/floor_plates_tests.rs"]
mod tests;
