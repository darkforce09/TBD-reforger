//! Role: contours.
//! Position: `terrain/relief` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::terrain::dem::grid::DemVectorGrid;

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

const CASE_EDGES: [&[(usize, usize)]; 16] = [
    &[],
    &[(0, 3)],
    &[(0, 1)],
    &[(1, 3)],
    &[(1, 2)],
    &[],
    &[(0, 2)],
    &[(2, 3)],
    &[(2, 3)],
    &[(0, 2)],
    &[],
    &[(1, 2)],
    &[(1, 3)],
    &[(0, 1)],
    &[(0, 3)],
    &[],
];

#[inline]
fn lerp(va: f64, ax: f64, ay: f64, vb: f64, bx: f64, by: f64, level: f64) -> (f64, f64) {
    let t = (level - va) / (vb - va);
    (ax + t * (bx - ax), ay + t * (by - ay))
}

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
        },
        if b1 != b2 {
            Some(lerp(v10, x1, y0, v11, x1, y1, level))
        } else {
            None
        },
        if b2 != b3 {
            Some(lerp(v11, x1, y1, v01, x0, y1, level))
        } else {
            None
        },
        if b3 != b0 {
            Some(lerp(v01, x0, y1, v00, x0, y0, level))
        } else {
            None
        },
    ]
}

fn saddle_edges(c: u8, center_in: bool) -> [(usize, usize); 2] {
    let connected = [(0, 1), (2, 3)];
    let split = [(0, 3), (1, 2)];
    if c == 5 {
        if center_in { connected } else { split }
    } else if center_in {
        split
    } else {
        connected
    }
}

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

type Seg = ((f64, f64), (f64, f64));

fn march_cell_pairs(cell: &Cell, level: f64, out: &mut Vec<Seg>) {
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
            out.push((p, q));
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

/// Contour ring.
#[derive(Clone, Debug, PartialEq)]
pub struct ContourRing {
    /// Iso level (metres ASL) this ring was marched at.
    pub level: f64,

    /// `true` iff the polyline is a closed loop (first ≈ last within [`RING_WELD_EPS`]).
    pub closed: bool,

    /// Ordered vertices in world meters. For a `closed` ring the endpoint is NOT duplicated.
    pub points: Vec<(f64, f64)>,
}

const RING_WELD_EPS: f64 = 1e-6;

#[inline]
fn near(a: (f64, f64), b: (f64, f64)) -> bool {
    (a.0 - b.0).abs() <= RING_WELD_EPS && (a.1 - b.1).abs() <= RING_WELD_EPS
}

fn chain_segments_into_rings(segs: Vec<Seg>, level: f64, out: &mut Vec<ContourRing>) {
    let segs: Vec<Seg> = segs.into_iter().filter(|&(a, b)| !near(a, b)).collect();
    let mut used = vec![false; segs.len()];

    for start in 0..segs.len() {
        if used[start] {
            continue;
        }
        used[start] = true;
        let (head, mut tail) = segs[start];
        let mut points = vec![head, tail];
        let mut closed = false;
        loop {
            if near(tail, head) {
                points.pop();
                closed = true;
                break;
            }

            let mut advanced = false;
            for k in 0..segs.len() {
                if used[k] {
                    continue;
                }
                let (a, b) = segs[k];
                let next = if near(a, tail) {
                    Some(b)
                } else if near(b, tail) {
                    Some(a)
                } else {
                    None
                };
                if let Some(n) = next {
                    used[k] = true;
                    tail = n;
                    points.push(n);
                    advanced = true;
                    break;
                }
            }
            if !advanced {
                break;
            }
        }

        let min_pts = if closed { 3 } else { 2 };
        if points.len() < min_pts {
            continue;
        }
        out.push(ContourRing {
            level,
            closed,
            points,
        });
    }
}

/// Contour rings.
#[must_use]
pub fn contour_rings(grid: &DemVectorGrid, levels: &[f64]) -> Vec<ContourRing> {
    let mut rings: Vec<ContourRing> = Vec::new();
    if grid.cols < 2 || grid.rows < 2 || levels.is_empty() {
        return rings;
    }
    let cols = grid.cols;
    let mut sorted = levels.to_vec();
    sorted.sort_by(|a, b| a.total_cmp(b));

    for &level in &sorted {
        let mut segs: Vec<Seg> = Vec::new();
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
                if level <= lo || level > hi {
                    continue;
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
                march_cell_pairs(&cell, level, &mut segs);
            }
        }
        chain_segments_into_rings(segs, level, &mut rings);
    }
    rings
}

fn point_in_ring(pt: (f64, f64), ring: &ContourRing) -> bool {
    let p = &ring.points;
    let n = p.len();
    if n < 3 {
        return false;
    }
    let (px, py) = pt;
    let mut inside = false;
    let mut j = n - 1;
    for i in 0..n {
        let (xi, yi) = p[i];
        let (xj, yj) = p[j];
        if ((yi > py) != (yj > py)) && (px < (xj - xi) * (py - yi) / (yj - yi) + xi) {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// Nesting is tested by whether a higher ring's first vertex falls inside the candidate — contour rings nest without crossing, so a single interior point settles containment.
#[must_use]
pub fn summit_ring_indices(rings: &[ContourRing]) -> Vec<usize> {
    let mut out = Vec::new();
    for (i, cand) in rings.iter().enumerate() {
        if !cand.closed || cand.points.len() < 3 {
            continue;
        }
        let mut has_higher_inside = false;
        for (k, other) in rings.iter().enumerate() {
            if k == i || !other.closed || other.level <= cand.level {
                continue;
            }
            if let Some(&probe) = other.points.first()
                && point_in_ring(probe, cand)
            {
                has_higher_inside = true;
                break;
            }
        }
        if !has_higher_inside {
            out.push(i);
        }
    }
    out
}

/// Marching-squares isolines for many levels in ONE grid sweep. Mirror of `contourSegments` (`contours.ts:114`). Output is interleaved `[x0,y0,x1,y1]` per segment.
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
                continue;
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
#[path = "tests/contours_tests.rs"]
mod tests;
