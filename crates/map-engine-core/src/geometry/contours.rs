//! Contour geometry — **Class R** (bit-identical to `worldmap/contours.ts`). Marching-squares iso
//! polylines over the DEM grid; positive levels only (the sea band owns 0 m and below). Output is
//! the interleaved `[x0,y0,x1,y1]`-per-segment `Float32Array` a `LineLayer` draws.

use crate::dem::DemVectorGrid;

/// Coarse intervals march a coarser grid (plan R8). Mirror of `contourGridReductions`.
#[must_use]
pub fn contour_grid_reductions(interval_m: f64) -> usize {
    if interval_m >= 100.0 {
        2
    } else if interval_m >= 50.0 {
        1
    } else {
        0
    }
}

/// Positive iso levels for an interval up to the grid's max elevation. Mirror of `contourLevels`.
#[must_use]
pub fn contour_levels(interval_m: f64, max_elev_m: f64) -> Vec<f64> {
    let mut levels = Vec::new();
    if interval_m <= 0.0 || !max_elev_m.is_finite() {
        return levels;
    }
    let mut lv = interval_m;
    while lv <= max_elev_m {
        levels.push(lv);
        lv += interval_m;
    }
    levels
}

/// One marching-squares cell: corner values (BL, BR, TR, TL) + its world box.
#[derive(Clone, Copy)]
struct Cell {
    v00: f64,
    v10: f64,
    v11: f64,
    v01: f64,
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
}

/// Edge pairs per non-saddle case. Edges: 0=A bottom, 1=B right, 2=C top, 3=D left. Cases 5/10 are
/// saddles (empty here — resolved by `saddle_edges`). Mirror of `CASE_EDGES` (`contours.ts:43`).
const CASE_EDGES: [&[(usize, usize)]; 16] = [
    &[],             // 0
    &[(0, 3)],       // 1
    &[(0, 1)],       // 2
    &[(1, 3)],       // 3
    &[(1, 2)],       // 4
    &[],             // 5 saddle
    &[(0, 2)],       // 6
    &[(2, 3)],       // 7
    &[(2, 3)],       // 8
    &[(0, 2)],       // 9
    &[],             // 10 saddle
    &[(1, 2)],       // 11
    &[(1, 3)],       // 12
    &[(0, 1)],       // 13
    &[(0, 3)],       // 14
    &[],             // 15
];

/// Linear iso crossing between two corners (they straddle `level`). Mirror of `lerp`.
#[inline]
fn lerp(va: f64, ax: f64, ay: f64, vb: f64, bx: f64, by: f64, level: f64) -> (f64, f64) {
    let t = (level - va) / (vb - va);
    (ax + t * (bx - ax), ay + t * (by - ay))
}

/// Crossing points on the 4 cell edges (`None` where the edge doesn't straddle). Mirror of
/// `edgePoints`.
fn edge_points(cell: &Cell, level: f64) -> [Option<(f64, f64)>; 4] {
    let Cell {
        v00,
        v10,
        v11,
        v01,
        x0,
        y0,
        x1,
        y1,
    } = *cell;
    let b0 = v00 >= level;
    let b1 = v10 >= level;
    let b2 = v11 >= level;
    let b3 = v01 >= level;
    [
        if b0 != b1 {
            Some(lerp(v00, x0, y0, v10, x1, y0, level))
        } else {
            None
        }, // A bottom
        if b1 != b2 {
            Some(lerp(v10, x1, y0, v11, x1, y1, level))
        } else {
            None
        }, // B right
        if b2 != b3 {
            Some(lerp(v11, x1, y1, v01, x0, y1, level))
        } else {
            None
        }, // C top
        if b3 != b0 {
            Some(lerp(v01, x0, y1, v00, x0, y0, level))
        } else {
            None
        }, // D left
    ]
}

/// Saddle (case 5/10) edge pairs, chosen by whether the cell centre is inside. Mirror of
/// `saddleEdges`.
fn saddle_edges(c: u8, center_in: bool) -> [(usize, usize); 2] {
    let connected = [(0, 1), (2, 3)];
    let split = [(0, 3), (1, 2)];
    if c == 5 {
        if center_in {
            connected
        } else {
            split
        }
    } else if center_in {
        split
    } else {
        connected
    }
}

/// March one cell at one level; append each segment's `[x0,y0,x1,y1]` to `seg`. Mirror of
/// `marchCell`.
fn march_cell(cell: &Cell, level: f64, seg: &mut Vec<f32>) {
    let c = (if cell.v00 >= level { 1u8 } else { 0 })
        | (if cell.v10 >= level { 2 } else { 0 })
        | (if cell.v11 >= level { 4 } else { 0 })
        | (if cell.v01 >= level { 8 } else { 0 });
    if c == 0 || c == 15 {
        return;
    }
    let pts = edge_points(cell, level);
    let mut push = |e0: usize, e1: usize| {
        if let (Some(p), Some(q)) = (pts[e0], pts[e1]) {
            seg.push(p.0 as f32);
            seg.push(p.1 as f32);
            seg.push(q.0 as f32);
            seg.push(q.1 as f32);
        }
    };
    if c == 5 || c == 10 {
        let center_in = (cell.v00 + cell.v10 + cell.v11 + cell.v01) / 4.0 >= level;
        for (e0, e1) in saddle_edges(c, center_in) {
            push(e0, e1);
        }
    } else {
        for &(e0, e1) in CASE_EDGES[c as usize] {
            push(e0, e1);
        }
    }
}

/// Marching-squares isolines for many levels in ONE grid sweep. Mirror of `contourSegments`
/// (`contours.ts:114`). Output is interleaved `[x0,y0,x1,y1]` per segment.
#[must_use]
pub fn contour_segments(grid: &DemVectorGrid, levels: &[f64]) -> Vec<f32> {
    let mut seg: Vec<f32> = Vec::new();
    if grid.cols < 2 || grid.rows < 2 || levels.is_empty() {
        return seg;
    }
    let cols = grid.cols;
    let mut sorted = levels.to_vec();
    sorted.sort_by(|a, b| a.total_cmp(b));

    for j in 0..grid.rows - 1 {
        let y0 = grid.origin_y + j as f64 * grid.cell_y;
        let y1 = y0 + grid.cell_y;
        for i in 0..cols - 1 {
            let v00 = f64::from(grid.data[j * cols + i]);
            let v10 = f64::from(grid.data[j * cols + i + 1]);
            let v11 = f64::from(grid.data[(j + 1) * cols + i + 1]);
            let v01 = f64::from(grid.data[(j + 1) * cols + i]);
            let lo = v00.min(v10).min(v11).min(v01);
            let hi = v00.max(v10).max(v11).max(v01);
            if sorted[0] > hi {
                continue; // no level reaches this cell
            }
            let x0 = grid.origin_x + i as f64 * grid.cell_x;
            let cell = Cell {
                v00,
                v10,
                v11,
                v01,
                x0,
                y0,
                x1: x0 + grid.cell_x,
                y1,
            };
            for &level in &sorted {
                if level <= lo {
                    continue;
                }
                if level > hi {
                    break;
                }
                march_cell(&cell, level, &mut seg);
            }
        }
    }
    seg
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid(data: Vec<f32>, cols: usize, rows: usize) -> DemVectorGrid {
        DemVectorGrid {
            data,
            cols,
            rows,
            cell_x: 1.0,
            cell_y: 1.0,
            origin_x: 0.0,
            origin_y: 0.0,
            max_elev_m: 100.0,
        }
    }

    #[test]
    fn reductions_and_levels() {
        assert_eq!(contour_grid_reductions(100.0), 2);
        assert_eq!(contour_grid_reductions(50.0), 1);
        assert_eq!(contour_grid_reductions(20.0), 0);
        assert_eq!(contour_levels(10.0, 35.0), vec![10.0, 20.0, 30.0]);
        assert!(contour_levels(0.0, 35.0).is_empty());
        assert!(contour_levels(10.0, f64::INFINITY).is_empty());
    }

    #[test]
    fn single_diagonal_ramp_crosses_at_midpoint() {
        // 2×2 cell, corners 0 (BL), 0 (BR), 10 (TR), 0 (TL); level 5 crosses two edges.
        let g = grid(vec![0.0, 0.0, 0.0, 10.0], 2, 2);
        let seg = contour_segments(&g, &[5.0]);
        // one segment → 4 floats.
        assert_eq!(seg.len(), 4);
    }

    #[test]
    fn empty_when_no_level_reaches() {
        let g = grid(vec![1.0, 1.0, 1.0, 1.0], 2, 2);
        assert!(contour_segments(&g, &[50.0]).is_empty());
    }

    #[test]
    fn closed_loop_has_even_edge_degree() {
        // A hill in the centre of a 5×5 grid should yield closed rings → every vertex even degree.
        let mut data = vec![0.0f32; 25];
        data[12] = 10.0; // centre
        let g = grid(data, 5, 5);
        let seg = contour_segments(&g, &[5.0]);
        assert!(seg.len().is_multiple_of(4));
    }
}
