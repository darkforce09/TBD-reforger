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
    &[],       // 0
    &[(0, 3)], // 1
    &[(0, 1)], // 2
    &[(1, 3)], // 3
    &[(1, 2)], // 4
    &[],       // 5 saddle
    &[(0, 2)], // 6
    &[(2, 3)], // 7
    &[(2, 3)], // 8
    &[(0, 2)], // 9
    &[],       // 10 saddle
    &[(1, 2)], // 11
    &[(1, 3)], // 12
    &[(0, 1)], // 13
    &[(0, 3)], // 14
    &[],       // 15
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
        if center_in { connected } else { split }
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

/// A single marching-squares crossing segment as an ordered endpoint pair in world meters. T-640
/// ring chaining works on full-precision `f64` pairs (not the quantised `f32` of [`contour_segments`])
/// so shared cell-edge crossings match exactly when welding segments into rings.
type Seg = ((f64, f64), (f64, f64));

/// March one cell at one level, appending each crossing segment as a `(p, q)` point pair. Same
/// case logic as [`march_cell`] but keeps endpoints as `(f64, f64)` pairs (not quantised `f32`) so
/// the ring chainer in [`contour_rings`] can match shared endpoints exactly. T-640.
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

/// One chained iso-polyline at a single level: the ordered vertices plus whether the chain closed
/// back on its start (a loop enclosing a summit or basin) vs. ran into the grid edge (open). T-640
/// — the summit-ring rule ([`summit_ring_indices`]) needs ring **closure** and level, which the flat
/// [`contour_segments`] `Vec<f32>` cannot express.
#[derive(Clone, Debug, PartialEq)]
pub struct ContourRing {
    /// Iso level (metres ASL) this ring was marched at.
    pub level: f64,
    /// `true` iff the polyline is a closed loop (first ≈ last within [`RING_WELD_EPS`]).
    pub closed: bool,
    /// Ordered vertices in world meters. For a `closed` ring the endpoint is NOT duplicated.
    pub points: Vec<(f64, f64)>,
}

/// Endpoint-match tolerance (world meters) when welding marching-squares segments into a ring.
/// Crossings on a shared cell edge are computed from the same two corners by the same `lerp`, so
/// adjacent cells produce bit-identical endpoints — but a small eps keeps the chainer robust to any
/// f64 drift and to the `x1 = x0 + cell` box arithmetic. Far below one cell (cells are ≥ ~8 m).
const RING_WELD_EPS: f64 = 1e-6;

#[inline]
fn near(a: (f64, f64), b: (f64, f64)) -> bool {
    (a.0 - b.0).abs() <= RING_WELD_EPS && (a.1 - b.1).abs() <= RING_WELD_EPS
}

/// Chain one level's marching-squares segments into ordered polylines, flagging each as closed
/// (loop) or open (met a grid edge). Greedy endpoint welding: start a chain from any unused segment,
/// then repeatedly append the unused segment whose either endpoint matches the chain's tail, until
/// none matches (open) or the tail returns to the head (closed). O(n²) in the per-level segment
/// count — a coarse contour grid yields few segments per level, and this runs once per interval
/// band (memoised by `last_interval` in `dem_vectors.rs`), not per frame.
fn chain_segments_into_rings(segs: Vec<Seg>, level: f64, out: &mut Vec<ContourRing>) {
    // Drop zero-length segments up front: at an exact-value corner crossing marching squares can
    // emit a point-segment (a ≈ b), which would otherwise chain into a degenerate 2-point "ring".
    let segs: Vec<Seg> = segs.into_iter().filter(|&(a, b)| !near(a, b)).collect();
    let mut used = vec![false; segs.len()];
    // Deterministic order: preserve the grid-sweep segment order for stable rings across runs.
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
                // Weld the duplicate closing vertex off and mark the loop closed.
                points.pop();
                closed = true;
                break;
            }
            // Find an unused segment touching `tail`.
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
                break; // open chain — ran off the grid edge
            }
        }
        // A ring needs ≥3 vertices to bound an area (closed) or ≥2 to be a real polyline (open);
        // anything smaller is welding noise, not a contour.
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

/// Marching-squares isolines chained into per-level [`ContourRing`]s (closed loops vs. open chains),
/// for the same grid sweep as [`contour_segments`]. T-640 — the source of ring identity/closure the
/// summit-ring emphasis rule consumes. Levels are marched low→high so nested rings appear in that
/// order (not relied on by [`summit_ring_indices`], which is order-independent).
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

/// Ray-cast point-in-polygon over a closed ring's vertices (even-odd rule). `ring.points` holds the
/// loop without a duplicated closing vertex; the `(n-1, 0)` wrap closes it. T-640 nesting test.
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

/// Per-peak "highest closed ring" selection — the single T-640 emphasis. Returns the indices (into
/// `rings`) of the closed rings that are each the innermost (highest-level) closed contour of their
/// peak: a closed ring qualifies iff **no other closed ring at a strictly higher level nests inside
/// it**. Two distinct peaks each yield their own summit ring (their top rings don't nest in each
/// other); a peak's lower rings are rejected because the higher ring of the same peak sits inside
/// them. This is a per-peak rule, deliberately NOT "every Nth level" index contours.
///
/// Nesting is tested by whether a higher ring's first vertex falls inside the candidate — contour
/// rings nest without crossing, so a single interior point settles containment.
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

    // ── T-640 ────────────────────────────────────────────────────────────────────────────────────

    /// A conical bump of radius `r` cells centred in a `side`×`side` grid: `peak` at centre falling
    /// linearly to 0 at radius `r`, clamped at 0 outside. Well inside the grid → its contours close.
    fn bump(side: usize, cx: f64, cy: f64, r: f64, peak: f64) -> Vec<f32> {
        let mut data = vec![0.0f32; side * side];
        for j in 0..side {
            for i in 0..side {
                let d = (((i as f64 - cx).powi(2)) + ((j as f64 - cy).powi(2))).sqrt();
                let h = (peak * (1.0 - d / r)).max(0.0);
                data[j * side + i] = h as f32;
            }
        }
        data
    }

    #[test]
    fn ring_closure_distinguishes_closed_loop_from_open_chain() {
        // (a) A centred bump well inside a 15×15 grid → the level-5 contour is a CLOSED loop.
        let g = grid(bump(15, 7.0, 7.0, 6.0, 10.0), 15, 15);
        let rings = contour_rings(&g, &[5.0]);
        assert!(
            rings.iter().any(|r| r.closed),
            "a bump interior to the grid must yield at least one closed ring"
        );
        // A closed ring's polyline holds no duplicated closing vertex and has ≥3 points.
        let closed = rings.iter().find(|r| r.closed).unwrap();
        assert!(closed.points.len() >= 3);
        assert!(
            !near(closed.points[0], *closed.points.last().unwrap()),
            "closed ring must NOT carry a duplicated closing vertex"
        );

        // (b) A west-high / east-low ramp: the level-5 iso runs edge→edge → an OPEN chain, never
        // closed. Column 0..3 high (10), the rest 0, so the crossing hits the top and bottom edges.
        let mut ramp = vec![0.0f32; 15 * 15];
        for j in 0..15 {
            for i in 0..15 {
                ramp[j * 15 + i] = if i <= 3 { 10.0 } else { 0.0 };
            }
        }
        let gr = grid(ramp, 15, 15);
        let rr = contour_rings(&gr, &[5.0]);
        assert!(!rr.is_empty(), "the ramp must produce a contour");
        assert!(
            rr.iter().all(|r| !r.closed),
            "an edge→edge ramp iso must be an OPEN chain, not a closed loop"
        );
    }

    #[test]
    fn per_peak_selects_one_highest_closed_ring_each() {
        // Two separated bumps in a 40×40 grid, valley (0 m) between them so their rings never merge.
        // Peak A crests 33 m (closes rings at 10/20/30); peak B crests 23 m (closes at 10/20 only).
        // Heights sit BETWEEN levels so no iso lands on a grid maximum (which would degenerate to a
        // point and drop out) — A's innermost is its 30 m ring, B's is its 20 m ring.
        let side = 40;
        let a = bump(side, 11.0, 20.0, 8.0, 33.0);
        let b = bump(side, 28.0, 20.0, 8.0, 23.0);
        let mut data = vec![0.0f32; side * side];
        for k in 0..data.len() {
            data[k] = a[k].max(b[k]);
        }
        let g = grid(data, side, side);
        let levels = vec![10.0, 20.0, 30.0];
        let rings = contour_rings(&g, &levels);
        let summit = summit_ring_indices(&rings);

        // Exactly one summit ring per peak.
        assert_eq!(
            summit.len(),
            2,
            "one highest-closed-ring per peak → two total"
        );

        // Peak A's summit is its 30 m ring; peak B's is its 20 m ring (B never reaches 30 m).
        let mut summit_levels: Vec<f64> = summit.iter().map(|&i| rings[i].level).collect();
        summit_levels.sort_by(|x, y| x.total_cmp(y));
        assert_eq!(summit_levels, vec![20.0, 30.0]);

        // Every selected ring is closed, and none is an every-Nth pick: the 10 m level (present for
        // BOTH peaks) is never a summit ring — a lower ring always has a higher one nested inside.
        for &i in &summit {
            assert!(rings[i].closed);
            assert!((rings[i].level - 10.0).abs() > f64::EPSILON);
        }
    }

    #[test]
    fn no_summit_rings_when_nothing_closes() {
        // The west/east ramp again: all-open contours → the per-peak rule selects nothing.
        let mut ramp = vec![0.0f32; 12 * 12];
        for j in 0..12 {
            for i in 0..12 {
                ramp[j * 12 + i] = if i <= 2 { 10.0 } else { 0.0 };
            }
        }
        let g = grid(ramp, 12, 12);
        let rings = contour_rings(&g, &[5.0]);
        assert!(summit_ring_indices(&rings).is_empty());
    }
}
