//! Floor plate — the ScanFloorPlate port. The plate (a short window of downward entries around
//! the slab) is the level's REAL walkable footprint: a mezzanine void or a floor-to-ridge room
//! has no plate cells and stays void (operator decision). Boundary tracing lives in
//! [`super::rings`] — the old per-row min/max `outline()` bridged across in-row voids and was
//! deleted with the multi-ring tracer.

use super::params::Params;
use super::types::{PlanGrid, ScanMap};

/// Occupancy + per-cell floor height: `heights[ix * nz + iz]` is the TOPMOST y_down entry
/// inside the slab window (normalized frame), `None` where nothing landed in-window.
pub fn floor_plate(
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
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn plate_keeps_topmost_in_window_entry_and_marks_occupancy() {
        let mut y_down: ScanMap = HashMap::new();
        // Column (1, 1): roof at 8.0 (out of window), slab lip at 3.15 and 3.05 (both in).
        y_down.insert((1, 1), vec![8.0, 3.15, 3.05]);
        // Column (2, 1): only a roof entry — stays void.
        y_down.insert((2, 1), vec![8.0]);
        let p = Params::default();
        let (grid, heights) = floor_plate(&y_down, 4, 4, 3.1, &p);
        assert!(grid.get(1, 1));
        assert_eq!(heights[4 + 1], Some(3.15), "topmost in-window entry wins");
        assert!(!grid.get(2, 1));
        assert_eq!(heights[2 * 4 + 1], None);
        assert_eq!(grid.count(), 1);
    }
}
