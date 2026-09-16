//! Role: the indexed pick and marquee checked against a brute-force oracle.
//! Position: `editing/tools/selection` in the map engine.
//! Signals & state: a frozen camera snapshot and the app-side selected-id set; never the document.
//! Invariants: the oracle is a linear scan over the same rows, so an index that disagrees with it is wrong — not merely different.

use crate::data::store::SlotSoa;
use crate::spatial::indexing::point_index::PointIndex;

use super::gesture::GRID_CELL_M;
use super::pick::{box_nearest, d2_to};

/// Class-S self-check for the marquee (S3 parity, peer of [`pick_selfcheck`]): `PointIndex::pick_rect`
/// must return the SAME id SET as a brute-force box scan over the same seeded SoA, for a battery of
/// world boxes (each seed ± a spread of half-extents). Set-equality (sorted handle compare), so
/// grid vs row order is not a false negative. Runs in-browser over the real seeded SoA.
#[must_use]
pub fn marquee_selfcheck(soa: &SlotSoa) -> bool {
    let n = soa.ids.len();
    if n == 0 {
        return true;
    }
    let idx = PointIndex::build(soa.xs.clone(), soa.ys.clone(), GRID_CELL_M);
    let halfs = [0.0_f64, 5.0, 64.0, 512.0];
    for i in 0..n {
        let (sx, sy) = (f64::from(soa.xs[i]), f64::from(soa.ys[i]));
        for &h in &halfs {
            let (min_x, min_y, max_x, max_y) = (sx - h, sy - h, sx + h, sy + h);
            let mut via_index = idx.pick_rect(min_x, min_y, max_x, max_y);
            let mut via_brute: Vec<u32> = (0..n as u32)
                .filter(|&j| {
                    let (x, y) = (f64::from(soa.xs[j as usize]), f64::from(soa.ys[j as usize]));
                    x >= min_x && x <= max_x && y >= min_y && y <= max_y
                })
                .collect();
            via_index.sort_unstable();
            via_brute.sort_unstable();
            if via_index != via_brute {
                return false;
            }
        }
    }
    true
}

/// Class-S self-check (S3): the `PointIndex` box-nearest used by [`pick`] must agree with a
/// brute-force box scan over the SAME points, for every seed and a spread of ± offsets as the query.
/// Compared by resulting **nearest distance** (bit-exact f64), so an exactly-equidistant tie — where
/// grid-order and row-order could pick different handles — is not a false negative; for the
/// non-degenerate random seeds the handles coincide anyway. Runs in-browser over the real seeded SoA.
#[must_use]
pub fn pick_selfcheck(soa: &SlotSoa) -> bool {
    let n = soa.ids.len();
    if n == 0 {
        return true;
    }
    let idx = PointIndex::build(soa.xs.clone(), soa.ys.clone(), GRID_CELL_M);
    let offsets = [0.0_f64, 3.0, -3.0, 40.0, -40.0];
    let r = 64.0_f64; // world box half-size for the parity probe
    for i in 0..n {
        let (sx, sy) = (f64::from(soa.xs[i]), f64::from(soa.ys[i]));
        for &ox in &offsets {
            for &oy in &offsets {
                let (qx, qy) = (sx + ox, sy + oy);
                let via_index = box_nearest(&idx, soa, qx, qy, r);
                let via_brute = box_nearest_brute(soa, qx, qy, r);
                let ok = match (via_index, via_brute) {
                    (None, None) => true,
                    (Some(a), Some(b)) => d2_to(soa, a, qx, qy) == d2_to(soa, b, qx, qy),
                    _ => false,
                };
                if !ok {
                    return false;
                }
            }
        }
    }
    true
}

/// Brute-force box-nearest oracle: a linear scan over every slot within the ±`r` box. The Class-S
/// reference for [`box_nearest`].
fn box_nearest_brute(soa: &SlotSoa, qx: f64, qy: f64, r: f64) -> Option<u32> {
    let mut best: Option<(f64, u32)> = None;
    for i in 0..soa.ids.len() {
        let (x, y) = (f64::from(soa.xs[i]), f64::from(soa.ys[i]));
        if x >= qx - r && x <= qx + r && y >= qy - r && y <= qy + r {
            let d2 = d2_to(soa, i as u32, qx, qy);
            if best.is_none_or(|(bd, _)| d2 < bd) {
                best = Some((d2, i as u32));
            }
        }
    }
    best.map(|(_, i)| i)
}

// ── smoke bridge ────────────────────────────────────────────────────────────────────────────────
