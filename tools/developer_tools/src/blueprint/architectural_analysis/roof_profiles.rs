//! Roof heightfield emission: downsample `VerticalScan.top` (the y− top-surface field) into the
//! blueprint's optional `RoofGrid`. Every choice leans CLEAR — the parity hard gate is zero
//! model-blocked/engine-clear pairs and a phantom roof is worse than no roof:
//! - **min** aggregation biases the surface low, so engine-clear rays skimming just above the
//!   real roof keep positive clearance;
//! - a coarse cell must reach `roof_min_coverage` of its fine block (default: FULLY covered) or
//!   it is `None` — the surface never reaches past the true silhouette (the dump's air PAD
//!   guarantees silhouette-straddling cells fail, and truncated edge blocks can never pass);
//! - surfaces below `floors[0] + roof_min_above_floor_m` (stoops, terraces) are dropped.

use website_map_engine::world::architecture::blueprint::footprint::RoofGrid;

use super::march::r2;
use super::params::Params;
use super::types::{DumpMeta, VerticalScan};

/// Downsample the top-surface field into a local-frame [`RoofGrid`]; `None` when nothing
/// roof-like survives (the blueprint then simply omits `roof`).
pub fn build(vert: &VerticalScan, meta: &DumpMeta, p: &Params) -> Option<RoofGrid> {
    let k = ((p.roof_cell_m / meta.cell).round() as usize).max(1);
    let (cnx, cnz) = (vert.nx.div_ceil(k), vert.nz.div_ceil(k));
    let oy = meta.origin[1];
    let floor_local = vert.floors.first().copied().unwrap_or(0.0) + oy;

    let mut heights: Vec<Option<f64>> = vec![None; cnx * cnz];
    for cx in 0..cnx {
        for cz in 0..cnz {
            let mut lo = f64::INFINITY;
            let mut covered = 0usize;
            for ix in cx * k..((cx + 1) * k).min(vert.nx) {
                for iz in cz * k..((cz + 1) * k).min(vert.nz) {
                    if let Some(t) = vert.top_at(ix, iz) {
                        covered += 1;
                        lo = lo.min(t);
                    }
                }
            }
            // Fine cells past the grid edge count as uncovered: k*k is always the full block.
            if (covered as f64) < p.roof_min_coverage * (k * k) as f64 || covered == 0 {
                continue;
            }
            let h = r2(lo + oy);
            if h < floor_local + p.roof_min_above_floor_m {
                continue;
            }
            heights[cx * cnz + cz] = Some(h);
        }
    }

    for _ in 0..p.roof_erode_cells {
        heights = erode(&heights, cnx, cnz);
    }

    heights.iter().any(Option::is_some).then(|| RoofGrid {
        origin: [meta.origin[0], meta.origin[2]],
        // r2: 3 × 0.1 must land in the contract as 0.3, not 0.30000000000000004.
        cell_size_m: r2(k as f64 * meta.cell),
        nx: cnx,
        nz: cnz,
        heights_m: heights,
    })
}

/// One 4-neighbor erosion pass: a cell survives only when all four neighbors are covered
/// (out-of-range counts as `None`, so the outermost covered ring always erodes).
fn erode(h: &[Option<f64>], nx: usize, nz: usize) -> Vec<Option<f64>> {
    let get = |ix: i64, iz: i64| -> Option<f64> {
        if ix < 0 || iz < 0 || ix >= nx as i64 || iz >= nz as i64 {
            None
        } else {
            h[ix as usize * nz + iz as usize]
        }
    };
    let mut out = vec![None; h.len()];
    for ix in 0..nx as i64 {
        for iz in 0..nz as i64 {
            if get(ix, iz).is_some()
                && get(ix - 1, iz).is_some()
                && get(ix + 1, iz).is_some()
                && get(ix, iz - 1).is_some()
                && get(ix, iz + 1).is_some()
            {
                out[ix as usize * nz + iz as usize] = get(ix, iz);
            }
        }
    }
    out
}

#[cfg(test)]
#[path = "../tests/roof/tests.rs"]
mod tests;
