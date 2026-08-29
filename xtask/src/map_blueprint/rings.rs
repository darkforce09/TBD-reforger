//! Multi-ring rectilinear boundary tracing over an occupancy [`PlanGrid`] — the replacement for
//! the per-row min/max `outline()` walk, which bridged across in-row voids (painting the open
//! double-height room as floor) and could not represent holes or disconnected pieces.
//!
//! Method: boundary following on CELL EDGES. Every edge between a covered cell and an
//! uncovered/out-of-bounds neighbor becomes a directed edge with the covered region on its LEFT;
//! stitching those into loops yields outer rings wound CCW and hole rings wound CW by
//! construction. At a 4-valent vertex (two covered cells touching diagonally) the walk takes the
//! LEFT-most turn, which keeps covered cells 4-connected — diagonal-touching cells trace as two
//! separate outer rings, never a pinched bow-tie. All work happens on the integer lattice
//! (float-exact determinism); coordinates scale by `cell` only at emission.

use map_engine_core::building_blueprint::FloorPolygon;

use super::march::r2;
use super::types::PlanGrid;

/// One traced connected piece in NORMALIZED meters (outer CCW, holes CW), plus its outer area.
pub struct RawPiece {
    pub outer: Vec<[f64; 2]>,
    pub holes: Vec<Vec<[f64; 2]>>,
    pub area_m2: f64,
}

pub struct TracedRings {
    /// Pieces sorted largest-outer-first (deterministic tiebreak on first vertex).
    pub pieces: Vec<RawPiece>,
    /// Rings dropped by the min-area filter (reported in the stages debug, never silent).
    pub dropped: usize,
}

impl TracedRings {
    /// The blueprint-contract products, in normalized frame: `footprintPolygon` (largest outer
    /// ring, empty when nothing traced) and the full `floorPolygons` list.
    pub fn contract(&self) -> (Vec<[f64; 2]>, Vec<FloorPolygon>) {
        let footprint = self
            .pieces
            .first()
            .map(|p| p.outer.clone())
            .unwrap_or_default();
        let polys = self
            .pieces
            .iter()
            .map(|p| FloorPolygon {
                outer: p.outer.clone(),
                holes: p.holes.clone(),
            })
            .collect();
        (footprint, polys)
    }
}

type V = (i64, i64);

/// Trace all boundary rings of `grid`. Rings with `|area| < min_ring_area_m2` are dropped (and
/// counted); a dropped outer ring drops its holes with it.
pub fn trace(grid: &PlanGrid, cell: f64, min_ring_area_m2: f64) -> TracedRings {
    let rings = stitch_rings(collect_edges(grid));

    // Classify by signed lattice area; drop sub-threshold rings (outer AND holes).
    let cell2 = cell * cell;
    let mut dropped = 0usize;
    let mut outers: Vec<(Vec<V>, i64)> = Vec::new(); // (ring, |area|·2)
    let mut holes: Vec<Vec<V>> = Vec::new();
    for ring in rings {
        let a2 = shoelace_x2(&ring); // CCW positive
        if (a2.unsigned_abs() as f64) * cell2 / 2.0 < min_ring_area_m2 {
            dropped += 1;
            continue;
        }
        if a2 > 0 {
            outers.push((ring, a2.abs()));
        } else {
            holes.push(ring);
        }
    }

    // Assign each hole to the smallest containing outer (a hole with no surviving outer drops).
    let mut hole_sets: Vec<Vec<Vec<V>>> = vec![Vec::new(); outers.len()];
    for hole in holes {
        let probe = hole_interior_probe(&hole);
        let mut best: Option<(usize, i64)> = None;
        for (oi, (outer, a2)) in outers.iter().enumerate() {
            if !point_in_ring(probe, outer) {
                continue;
            }
            if best.is_none_or(|(_, ba)| *a2 < ba) {
                best = Some((oi, *a2));
            }
        }
        match best {
            Some((oi, _)) => hole_sets[oi].push(hole),
            None => dropped += 1,
        }
    }

    let mut pieces: Vec<RawPiece> = outers
        .into_iter()
        .zip(hole_sets)
        .map(|((outer, a2), hs)| RawPiece {
            outer: scale_ring(&outer, cell),
            holes: hs.iter().map(|h| scale_ring(h, cell)).collect(),
            area_m2: (a2 as f64) * cell2 / 2.0,
        })
        .collect();
    pieces.sort_by(|a, b| {
        b.area_m2
            .partial_cmp(&a.area_m2)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                a.outer
                    .first()
                    .partial_cmp(&b.outer.first())
                    .expect("verts")
            })
    });
    TracedRings { pieces, dropped }
}

/// Directed boundary edges, covered side on the LEFT, in deterministic row-major cell order.
fn collect_edges(grid: &PlanGrid) -> Vec<(V, V)> {
    let (nx, nz) = (grid.nx as i64, grid.nz as i64);
    let covered = |ix: i64, iz: i64| -> bool {
        ix >= 0 && iz >= 0 && ix < nx && iz < nz && grid.get(ix as usize, iz as usize)
    };
    let mut edges = Vec::new();
    for ix in 0..nx {
        for iz in 0..nz {
            if !covered(ix, iz) {
                continue;
            }
            if !covered(ix, iz - 1) {
                edges.push(((ix, iz), (ix + 1, iz))); // south, walk +x
            }
            if !covered(ix + 1, iz) {
                edges.push(((ix + 1, iz), (ix + 1, iz + 1))); // east, walk +z
            }
            if !covered(ix, iz + 1) {
                edges.push(((ix + 1, iz + 1), (ix, iz + 1))); // north, walk −x
            }
            if !covered(ix - 1, iz) {
                edges.push(((ix, iz + 1), (ix, iz))); // west, walk −z
            }
        }
    }
    edges
}

/// Stitch directed edges into closed loops. Seed order = edge collection order (deterministic);
/// at 4-valent vertices prefer the LEFT turn (cross(dir_in, dir_out) > 0).
fn stitch_rings(edges: Vec<(V, V)>) -> Vec<Vec<V>> {
    use std::collections::HashMap;
    let mut by_start: HashMap<V, Vec<usize>> = HashMap::new();
    for (i, e) in edges.iter().enumerate() {
        by_start.entry(e.0).or_default().push(i);
    }
    let mut used = vec![false; edges.len()];
    let mut rings = Vec::new();

    for seed in 0..edges.len() {
        if used[seed] {
            continue;
        }
        let mut ring: Vec<V> = vec![edges[seed].0];
        let mut cur = seed;
        loop {
            used[cur] = true;
            let (a, b) = edges[cur];
            ring.push(b);
            if b == ring[0] {
                break;
            }
            let din = (b.0 - a.0, b.1 - a.1);
            let cands = by_start.get(&b).map(Vec::as_slice).unwrap_or(&[]);
            let next = cands
                .iter()
                .copied()
                .filter(|&i| !used[i])
                .max_by_key(|&i| {
                    let (na, nb) = edges[i];
                    let dout = (nb.0 - na.0, nb.1 - na.1);
                    din.0 * dout.1 - din.1 * dout.0 // left turn = positive cross
                })
                .expect("boundary edges always close into loops");
            cur = next;
        }
        ring.pop(); // drop the duplicated closing vertex — rings are emitted OPEN
        rings.push(canonical(decimate(ring)));
    }
    rings
}

/// Drop collinear vertices (incoming direction == outgoing direction). Exact on integers.
fn decimate(ring: Vec<V>) -> Vec<V> {
    let n = ring.len();
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let p = ring[(i + n - 1) % n];
        let v = ring[i];
        let q = ring[(i + 1) % n];
        if (v.0 - p.0, v.1 - p.1) != (q.0 - v.0, q.1 - v.1) {
            out.push(v);
        }
    }
    out
}

/// Rotate so the lexicographically smallest vertex leads — stable goldens.
fn canonical(ring: Vec<V>) -> Vec<V> {
    let Some(min_idx) = ring
        .iter()
        .enumerate()
        .min_by_key(|(_, v)| **v)
        .map(|(i, _)| i)
    else {
        return ring;
    };
    let mut out = Vec::with_capacity(ring.len());
    out.extend_from_slice(&ring[min_idx..]);
    out.extend_from_slice(&ring[..min_idx]);
    out
}

/// Twice the signed lattice area (positive = CCW).
fn shoelace_x2(ring: &[V]) -> i64 {
    let n = ring.len();
    (0..n)
        .map(|i| {
            let (x0, z0) = ring[i];
            let (x1, z1) = ring[(i + 1) % n];
            x0 * z1 - x1 * z0
        })
        .sum()
}

/// A point strictly inside a hole ring: the hole interior lies on the RIGHT of its CW walk, so
/// offset the first edge's midpoint half a cell to the right.
fn hole_interior_probe(hole: &[V]) -> (f64, f64) {
    let (a, b) = (hole[0], hole[1]);
    let d = ((b.0 - a.0) as f64, (b.1 - a.1) as f64);
    let mid = ((a.0 + b.0) as f64 / 2.0, (a.1 + b.1) as f64 / 2.0);
    (mid.0 + d.1 * 0.5, mid.1 - d.0 * 0.5) // right of d = (dz, −dx)
}

/// Even-odd point-in-ring on lattice coordinates.
fn point_in_ring(p: (f64, f64), ring: &[V]) -> bool {
    let n = ring.len();
    let mut inside = false;
    let mut j = n - 1;
    for i in 0..n {
        let (ax, az) = (ring[i].0 as f64, ring[i].1 as f64);
        let (bx, bz) = (ring[j].0 as f64, ring[j].1 as f64);
        if ((az > p.1) != (bz > p.1)) && (p.0 < (bx - ax) * (p.1 - az) / (bz - az) + ax) {
            inside = !inside;
        }
        j = i;
    }
    inside
}

fn scale_ring(ring: &[V], cell: f64) -> Vec<[f64; 2]> {
    ring.iter()
        .map(|&(x, z)| [r2(x as f64 * cell), r2(z as f64 * cell)])
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid_from(rows: &[&str]) -> PlanGrid {
        // rows[iz] is a string over ix: '#' covered. Row 0 = iz 0.
        let nz = rows.len();
        let nx = rows[0].len();
        let mut g = PlanGrid::new(nx, nz);
        for (iz, row) in rows.iter().enumerate() {
            for (ix, c) in row.chars().enumerate() {
                if c == '#' {
                    g.set(ix, iz, true);
                }
            }
        }
        g
    }

    fn lattice(ring: &[[f64; 2]], cell: f64) -> Vec<(i64, i64)> {
        ring.iter()
            .map(|p| ((p[0] / cell).round() as i64, (p[1] / cell).round() as i64))
            .collect()
    }

    #[test]
    fn full_rect_is_one_ccw_ring_of_four() {
        let t = trace(&grid_from(&["####", "####", "####"]), 0.1, 0.0);
        assert_eq!(t.pieces.len(), 1);
        assert_eq!(t.dropped, 0);
        let outer = lattice(&t.pieces[0].outer, 0.1);
        assert_eq!(outer, vec![(0, 0), (4, 0), (4, 3), (0, 3)]);
        assert!(t.pieces[0].holes.is_empty());
        assert!((t.pieces[0].area_m2 - 0.12).abs() < 1e-9);
    }

    #[test]
    fn l_shape_traces_six_vertices() {
        // 4 wide × 2 tall base, 2 wide × 2 tall tower on top-left.
        let t = trace(&grid_from(&["####", "####", "##..", "##.."]), 0.1, 0.0);
        assert_eq!(t.pieces.len(), 1);
        let outer = lattice(&t.pieces[0].outer, 0.1);
        assert_eq!(outer, vec![(0, 0), (4, 0), (4, 2), (2, 2), (2, 4), (0, 4)]);
    }

    #[test]
    fn donut_has_one_cw_hole() {
        let t = trace(
            &grid_from(&["#####", "#####", "##.##", "#####", "#####"]),
            0.1,
            0.0,
        );
        assert_eq!(t.pieces.len(), 1);
        assert_eq!(t.pieces[0].holes.len(), 1);
        let hole = lattice(&t.pieces[0].holes[0], 0.1);
        // CW (negative shoelace), canonical start at lexicographic min (2,2).
        assert_eq!(hole, vec![(2, 2), (2, 3), (3, 3), (3, 2)]);
        let a2: i64 = shoelace_x2(&hole);
        assert!(a2 < 0, "hole must wind CW, area×2 = {a2}");
    }

    #[test]
    fn disconnected_pieces_become_two_polygons() {
        let t = trace(&grid_from(&["##..##", "##..##"]), 0.1, 0.0);
        assert_eq!(t.pieces.len(), 2);
        assert!(t.pieces.iter().all(|p| p.holes.is_empty()));
        let (fp, polys) = t.contract();
        assert_eq!(polys.len(), 2);
        // Largest-first tie: equal areas — deterministic order by first vertex.
        assert_eq!(lattice(&fp, 0.1)[0], (0, 0));
    }

    #[test]
    fn single_cell_is_a_four_vertex_ring() {
        let t = trace(&grid_from(&["#"]), 0.1, 0.0);
        assert_eq!(t.pieces.len(), 1);
        assert_eq!(t.pieces[0].outer.len(), 4);
    }

    #[test]
    fn diagonal_touch_stays_two_separate_rings() {
        // Two cells sharing only the corner (1,1) — left-turn rule must NOT fuse them.
        let t = trace(&grid_from(&["#.", ".#"]), 0.1, 0.0);
        assert_eq!(t.pieces.len(), 2);
        assert!(t.pieces.iter().all(|p| p.outer.len() == 4));
    }

    #[test]
    fn min_area_drops_noise_rings_and_counts_them() {
        // 3×3 block plus an isolated single cell (0.01 m² at 0.1 cell).
        let t = trace(&grid_from(&["###..", "###..", "###.#"]), 0.1, 0.02);
        assert_eq!(t.pieces.len(), 1);
        assert_eq!(t.dropped, 1);
    }

    #[test]
    fn trace_is_deterministic() {
        let g = grid_from(&["#####", "#.###", "###.#", "#####"]);
        let a = trace(&g, 0.1, 0.0);
        let b = trace(&g, 0.1, 0.0);
        let ser = |t: &TracedRings| {
            t.pieces
                .iter()
                .map(|p| (p.outer.clone(), p.holes.clone()))
                .collect::<Vec<_>>()
        };
        assert_eq!(ser(&a), ser(&b));
    }
}
