//! Floor plate + outline — ScanFloorPlate / OutlineFromGrid ports. The plate (a short window of
//! downward entries around the slab) is the level's REAL walkable footprint: a mezzanine void or
//! a floor-to-ridge room has no plate and falls outside the level polygon (operator decision).

use super::params::Params;
use super::types::{PlanGrid, ScanMap};

pub fn floor_plate(y_down: &ScanMap, nx: usize, nz: usize, slab_y: f64, p: &Params) -> PlanGrid {
    let mut grid = PlanGrid::new(nx, nz);
    let (lo, hi) = (slab_y - p.plate_below_m, slab_y + p.plate_above_m);
    for (&(ix, iz), entries) in y_down {
        if ix >= nx || iz >= nz {
            continue;
        }
        if entries.iter().any(|&y| y >= lo && y <= hi) {
            grid.set(ix, iz, true);
        }
    }
    grid
}

/// Axis-aligned outline from per-row solid extremes (handles L/T/U shapes, no holes): right edge
/// walks per-row maxX transitions in +z order, left edge walks minX back down. HYSTERESIS:
/// extreme changes under `outline_jump_cells` are collision jitter, not corners. Output points in
/// normalized meters (cell-edge coordinates, matching the live port).
pub fn outline(grid: &PlanGrid, cell: f64, p: &Params) -> Vec<[f64; 2]> {
    let (nx, nz) = (grid.nx, grid.nz);
    let mut row_min = vec![-1i64; nz];
    let mut row_max = vec![-1i64; nz];
    for iz in 0..nz {
        for ix in 0..nx {
            if grid.get(ix, iz) {
                if row_min[iz] < 0 {
                    row_min[iz] = ix as i64;
                }
                row_max[iz] = ix as i64;
            }
        }
    }

    let mut poly: Vec<[f64; 2]> = Vec::new();
    let push = |x: f64, z: f64, poly: &mut Vec<[f64; 2]>| poly.push([x, z]);

    let mut prev_max: i64 = -1000;
    let mut last_z = 0.0;
    for (iz, &rm) in row_max.iter().enumerate() {
        if rm < 0 {
            continue;
        }
        if (rm - prev_max).abs() >= p.outline_jump_cells || prev_max == -1000 {
            push((rm + 1) as f64 * cell, iz as f64 * cell, &mut poly);
            prev_max = rm;
        }
        last_z = (iz + 1) as f64 * cell;
    }
    if prev_max != -1000 {
        push((prev_max + 1) as f64 * cell, last_z, &mut poly);
    }

    let mut prev_min: i64 = -1000;
    let mut first_z = 0.0;
    for (iz, &rm) in row_min.iter().enumerate().rev() {
        if rm < 0 {
            continue;
        }
        if (rm - prev_min).abs() >= p.outline_jump_cells || prev_min == -1000 {
            push(rm as f64 * cell, (iz + 1) as f64 * cell, &mut poly);
            prev_min = rm;
        }
        first_z = iz as f64 * cell;
    }
    if prev_min != -1000 {
        push(prev_min as f64 * cell, first_z, &mut poly);
    }
    poly
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l_shape_outline_has_corner_and_ignores_jitter() {
        // 20x20 grid: full rows 0..10, half rows (x<10) 10..20, plus 1-cell jitter on row 5.
        let mut g = PlanGrid::new(20, 20);
        for iz in 0..20 {
            for ix in 0..20 {
                let solid = if iz < 10 { true } else { ix < 10 };
                g.set(ix, iz, solid);
            }
        }
        g.set(19, 5, false); // jitter: rowMax 18 on one row — a 1-cell jump, must NOT be a corner
        let p = Params::default();
        let poly = outline(&g, 0.1, &p);
        let has_step = poly
            .iter()
            .any(|pt| (pt[0] - 1.0).abs() < 1e-9 && pt[1] >= 0.9);
        assert!(has_step, "L-step at x=1.0 missing: {poly:?}");
        assert!(
            !poly.iter().any(|pt| (pt[0] - 1.9).abs() < 1e-9),
            "jitter row leaked into the outline: {poly:?}"
        );
        assert!(
            poly.len() <= 8,
            "hysteresis keeps it terse, got {}: {poly:?}",
            poly.len()
        );
    }
}
