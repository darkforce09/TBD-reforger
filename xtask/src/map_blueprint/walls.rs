//! Wall extraction — the reason the split exists.
//!
//! `segments` (default): per y-slice interval observations clustered across ~12 slices per band,
//! with three independent roof-phantom vetoes (roof mask, persistence, stationarity). The dump's
//! continuous interval endpoints across many slices are strictly more information than the two
//! rasterized heights the live pipeline compared, which is what kept fragmenting the 2nd floor.
//!
//! `grid`: faithful port of the live ScanAxisX/Z cell marking + greedy RectsFromGrid +
//! MergeWallRects + two-height AND — the M2 equivalence bridge and the A/B fallback.

use std::collections::HashMap;

use super::pair::{ascending, pair_consuming};
use super::params::Params;
use super::types::{MassRect, PlanGrid, ScanMap, VerticalScan, VoxelDump, WallSeg};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Algo {
    Segments,
    Grid,
}

#[derive(Debug)]
pub struct BandWalls {
    pub walls: Vec<WallSeg>,
    /// (segment, is_exterior) — exterior classification is algorithm-specific.
    pub exterior: Vec<bool>,
    pub masses: Vec<MassRect>,
    /// Diagnostic: raw observation / rect count before merging.
    pub raw_count: usize,
}

/// One cluster's fate during extraction — the attribution record behind every wall (or gap)
/// the viewer shows. Emitted into the `--debug-dir` stages JSON.
#[derive(Debug, serde::Serialize)]
pub struct ClusterDebug {
    /// "z-running" (constant x) or "x-running" (constant z).
    pub axis: &'static str,
    /// Fixed lattice index (iz for z-running, ix for x-running).
    pub fixed: usize,
    /// Cluster centerline along the interval axis, normalized meters.
    pub center: f64,
    pub thick: f64,
    pub rows_seen: usize,
    /// Roof-clipped slice rows available at this column (the persistence denominator).
    pub rows_avail: usize,
    /// Rows required: max(min_persist_rows, ceil(persistence_frac × rows_avail)).
    pub need: usize,
    pub drift: f64,
    /// "accepted" | "persistence" | "drift".
    pub verdict: &'static str,
}

/// Per-band extraction attribution for `--debug-dir` (never allocated on normal runs).
#[derive(Debug, Default, serde::Serialize)]
pub struct BandDebug {
    pub clusters: Vec<ClusterDebug>,
    pub graze_vetoed: usize,
    pub mass_cells: usize,
}

pub fn extract_band(
    dump: &VoxelDump,
    vert: &VerticalScan,
    band_lo: f64,
    band_hi: f64,
    algo: Algo,
    p: &Params,
    debug: Option<&mut BandDebug>,
) -> BandWalls {
    match algo {
        Algo::Segments => segments_band(dump, vert, band_lo, band_hi, p, debug),
        Algo::Grid => grid_band(dump, vert, band_lo, band_hi, p),
    }
}

// ── shared helpers ──────────────────────────────────────────────────────────────────────────────

/// Slice rows (iy) whose center height falls in the band's observation window.
fn slice_rows(band_lo: f64, band_hi: f64, cell: f64, ny: usize, p: &Params) -> Vec<usize> {
    let lo = band_lo + p.slice_lo_m;
    let hi = (band_lo + p.slice_hi_m).min(band_hi - p.slice_top_margin_m);
    let mut rows: Vec<usize> = (0..ny)
        .filter(|&iy| {
            let y = (iy as f64 + 0.5) * cell;
            y >= lo && y <= hi
        })
        .collect();
    if rows.is_empty() {
        // Degenerate short band: take the single row nearest the live low probe height.
        let target = band_lo + p.band_low_m;
        let iy = ((target / cell - 0.5).round().max(0.0) as usize).min(ny.saturating_sub(1));
        rows.push(iy);
    }
    rows
}

fn median(sorted_input: &mut [f64]) -> f64 {
    sorted_input.sort_by(f64::total_cmp);
    sorted_input[sorted_input.len() / 2]
}

// ── segments algorithm ──────────────────────────────────────────────────────────────────────────

struct Obs {
    row: usize,
    center: f64,
    thick: f64,
}

fn segments_band(
    dump: &VoxelDump,
    vert: &VerticalScan,
    band_lo: f64,
    band_hi: f64,
    p: &Params,
    mut debug: Option<&mut BandDebug>,
) -> BandWalls {
    let m = dump.meta();
    let cell = m.cell;
    let rows = slice_rows(band_lo, band_hi, cell, m.dims[1], p);
    let mut walls = Vec::new();
    let mut raw = 0usize;
    let mut vetoed = 0usize;
    let mut thick_hits: HashMap<(usize, usize), usize> = HashMap::new();

    // z-running walls (constant x) from x± marches; keyed (iy, iz), interval axis = x.
    let cols_z = collect_columns(
        &dump.x_pos,
        &dump.x_neg,
        &rows,
        m.dims[2],
        cell,
        p,
        &mut raw,
        &mut vetoed,
        &mut thick_hits,
        |center, k, row_y| roof_veto(vert, cell, center, (k as f64 + 0.5) * cell, row_y, p),
        |center, k, ny_thick| thick_cells(center, k, ny_thick, cell, true),
        |center, k| roof_clipped_rows(vert, &rows, cell, (center / cell) as usize, k, p),
        "z-running",
        debug.as_deref_mut(),
    );
    walls.extend(merge_columns(cols_z, cell, p, true));

    // x-running walls (constant z) from z± marches; keyed (ix, iy), interval axis = z.
    let cols_x = collect_columns_zaxis(
        dump,
        &rows,
        m.dims[0],
        p,
        &mut raw,
        &mut vetoed,
        &mut thick_hits,
        vert,
        debug.as_deref_mut(),
    );
    walls.extend(merge_columns(cols_x, cell, p, false));

    // Interior masses: cells persistently covered by over-thick intervals → greedy rects.
    // (Deliberately still GLOBALLY normalized — masses are stand-in furniture; the per-column
    // denominator below is a wall-acceptance change only.)
    let need = ((rows.len() as f64) * p.persistence_frac).ceil() as usize;
    let mut mass_grid = PlanGrid::new(vert.nx, vert.nz);
    for (&(ix, iz), &n) in &thick_hits {
        if n >= need && ix < vert.nx && iz < vert.nz {
            mass_grid.set(ix, iz, true);
        }
    }
    if let Some(dbg) = debug.as_deref_mut() {
        dbg.graze_vetoed = vetoed;
        dbg.mass_cells = mass_grid.count();
    }
    let masses = rects_from_grid(&mass_grid, cell)
        .into_iter()
        .map(|rect| MassRect { rect })
        .collect();

    let exterior = classify_exterior_flood(&walls, vert.nx, vert.nz, cell);
    BandWalls {
        walls,
        exterior,
        masses,
        raw_count: raw,
    }
}

/// Slice rows a wall column at plan cell `(ix, iz)` can actually occupy: rows whose center sits
/// clear below the local TOP surface (margin = `roof_graze_eps_m`, symmetric with the graze
/// veto, so rows the veto eats also leave the persistence denominator). No top surface → the
/// full window. This is what lets knee walls and gable-side walls under the roof plane pass
/// persistence on the rows they can exist in, instead of being judged against the whole band.
fn roof_clipped_rows(
    vert: &VerticalScan,
    rows: &[usize],
    cell: f64,
    ix: usize,
    iz: usize,
    p: &Params,
) -> usize {
    if ix >= vert.nx || iz >= vert.nz {
        return rows.len();
    }
    let Some(top) = vert.top_at(ix, iz) else {
        return rows.len();
    };
    rows.iter()
        .filter(|&&r| (r as f64 + 0.5) * cell <= top - p.roof_graze_eps_m)
        .count()
}

/// Gather per-scanline interval observations and cluster them into wall columns.
/// Generic over the axis via the closures; returns (fixed_index, along_center, thickness).
#[allow(clippy::too_many_arguments)]
fn collect_columns(
    pos: &ScanMap,
    neg: &ScanMap,
    rows: &[usize],
    n_fixed: usize,
    cell: f64,
    p: &Params,
    raw: &mut usize,
    vetoed: &mut usize,
    thick_hits: &mut HashMap<(usize, usize), usize>,
    veto: impl Fn(f64, usize, f64) -> bool,
    thick_mark: impl Fn(f64, usize, f64) -> Vec<(usize, usize)>,
    avail: impl Fn(f64, usize) -> usize,
    axis: &'static str,
    mut debug: Option<&mut BandDebug>,
) -> Vec<(usize, f64, f64)> {
    let empty: Vec<f64> = Vec::new();
    let mut out = Vec::new();
    for k in 0..n_fixed {
        let mut obs: Vec<Obs> = Vec::new();
        for &row in rows {
            let fwd = pos.get(&(row, k)).unwrap_or(&empty);
            let closing = neg.get(&(row, k)).map(|v| ascending(v)).unwrap_or_default();
            if fwd.is_empty() && closing.is_empty() {
                continue;
            }
            let row_y = (row as f64 + 0.5) * cell;
            for iv in pair_consuming(fwd, &closing, p) {
                *raw += 1;
                if iv.len() > p.wall_max_thickness_m {
                    for cellxy in thick_mark(iv.mid(), k, iv.len()) {
                        *thick_hits.entry(cellxy).or_insert(0) += 1;
                    }
                    continue;
                }
                if veto(iv.mid(), k, row_y) {
                    *vetoed += 1;
                    continue;
                }
                obs.push(Obs {
                    row,
                    center: iv.mid(),
                    thick: iv.len(),
                });
            }
        }
        out.extend(cluster_columns(
            &mut obs,
            k,
            rows.len(),
            &avail,
            axis,
            p,
            debug.as_deref_mut(),
        ));
    }
    out
}

/// The z± maps are keyed (ix, iy) — different key order than x±, so the generic walker above
/// cannot be reused verbatim; this mirror walks per-ix and swaps the veto cell lookup.
#[allow(clippy::too_many_arguments)]
fn collect_columns_zaxis(
    dump: &VoxelDump,
    rows: &[usize],
    nx: usize,
    p: &Params,
    raw: &mut usize,
    vetoed: &mut usize,
    thick_hits: &mut HashMap<(usize, usize), usize>,
    vert: &VerticalScan,
    mut debug: Option<&mut BandDebug>,
) -> Vec<(usize, f64, f64)> {
    let cell = dump.meta().cell;
    let empty: Vec<f64> = Vec::new();
    let mut out = Vec::new();
    for ix in 0..nx {
        let mut obs: Vec<Obs> = Vec::new();
        for &row in rows {
            let fwd = dump.z_pos.get(&(ix, row)).unwrap_or(&empty);
            let closing = dump
                .z_neg
                .get(&(ix, row))
                .map(|v| ascending(v))
                .unwrap_or_default();
            if fwd.is_empty() && closing.is_empty() {
                continue;
            }
            let row_y = (row as f64 + 0.5) * cell;
            for iv in pair_consuming(fwd, &closing, p) {
                *raw += 1;
                if iv.len() > p.wall_max_thickness_m {
                    for c in thick_cells(iv.mid(), ix, iv.len(), cell, false) {
                        *thick_hits.entry(c).or_insert(0) += 1;
                    }
                    continue;
                }
                if roof_veto(vert, cell, (ix as f64 + 0.5) * cell, iv.mid(), row_y, p) {
                    *vetoed += 1;
                    continue;
                }
                obs.push(Obs {
                    row,
                    center: iv.mid(),
                    thick: iv.len(),
                });
            }
        }
        out.extend(cluster_columns(
            &mut obs,
            ix,
            rows.len(),
            &|center: f64, fixed: usize| {
                roof_clipped_rows(vert, rows, cell, fixed, (center / cell) as usize, p)
            },
            "x-running",
            p,
            debug.as_deref_mut(),
        ));
    }
    out
}

/// Cluster one scanline's observations by center; keep clusters that persist across slices
/// without drifting (the two signals a sloped roof plane cannot fake). Persistence is judged
/// against the ROOF-CLIPPED row count at the cluster's own plan cell (`avail`), floored by
/// `min_persist_rows` so a normalized denominator cannot let 1–2-observation noise through.
fn cluster_columns(
    obs: &mut [Obs],
    fixed: usize,
    n_rows: usize,
    avail: &impl Fn(f64, usize) -> usize,
    axis: &'static str,
    p: &Params,
    mut debug: Option<&mut BandDebug>,
) -> Vec<(usize, f64, f64)> {
    obs.sort_by(|a, b| a.center.total_cmp(&b.center));
    let mut out = Vec::new();
    let mut i = 0;
    while i < obs.len() {
        let mut j = i + 1;
        while j < obs.len() && obs[j].center - obs[j - 1].center <= p.cluster_eps_m {
            j += 1;
        }
        let cluster = &obs[i..j];
        let mut rows_seen: Vec<usize> = cluster.iter().map(|o| o.row).collect();
        rows_seen.sort_unstable();
        rows_seen.dedup();
        let mut centers: Vec<f64> = cluster.iter().map(|o| o.center).collect();
        let med = median(&mut centers);
        let drift = cluster
            .iter()
            .map(|o| (o.center - med).abs())
            .fold(0.0, f64::max);
        let rows_avail = avail(med, fixed).min(n_rows);
        let floor_rows = p.min_persist_rows.min(n_rows).max(1);
        let need = (((rows_avail as f64) * p.persistence_frac).ceil() as usize).max(floor_rows);
        let persistent = rows_seen.len() >= need;
        let verdict = if !persistent {
            "persistence"
        } else if drift > p.max_drift_m {
            "drift"
        } else {
            "accepted"
        };
        if let Some(dbg) = debug.as_deref_mut() {
            let mut thicks: Vec<f64> = cluster.iter().map(|o| o.thick).collect();
            dbg.clusters.push(ClusterDebug {
                axis,
                fixed,
                center: med,
                thick: median(&mut thicks),
                rows_seen: rows_seen.len(),
                rows_avail,
                need,
                drift,
                verdict,
            });
        }
        if verdict == "accepted" {
            let mut thicks: Vec<f64> = cluster.iter().map(|o| o.thick).collect();
            out.push((fixed, med, median(&mut thicks)));
        }
        i = j;
    }
    out
}

/// Merge accepted columns into runs along the wall axis. Multiple OPEN chains, matched by
/// lateral position: two parallel walls interleave their columns in index order, so a single
/// running chain would flush on every alternation and starve both. `z_running` selects output
/// orientation: true → constant-x wall, columns keyed by iz; false → constant-z wall, keyed by ix.
fn merge_columns(
    mut cols: Vec<(usize, f64, f64)>,
    cell: f64,
    p: &Params,
    z_running: bool,
) -> Vec<WallSeg> {
    cols.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.total_cmp(&b.1)));
    let gap_cells = (p.run_gap_m / cell).floor() as usize + 1;
    let mut open: Vec<Vec<(usize, f64, f64)>> = Vec::new();
    let mut walls = Vec::new();

    let emit = |chain: Vec<(usize, f64, f64)>, walls: &mut Vec<WallSeg>| {
        let len = (chain.last().expect("non-empty").0 - chain[0].0 + 1) as f64 * cell;
        if len < p.wall_min_len_m {
            return;
        }
        let mut lats: Vec<f64> = chain.iter().map(|c| c.1).collect();
        let lat = median(&mut lats);
        let mut thicks: Vec<f64> = chain.iter().map(|c| c.2).collect();
        let thickness = median(&mut thicks);
        let a0 = chain[0].0 as f64 * cell;
        let a1 = (chain.last().expect("non-empty").0 + 1) as f64 * cell;
        let (start, end) = if z_running {
            ([lat, a0], [lat, a1])
        } else {
            ([a0, lat], [a1, lat])
        };
        walls.push(WallSeg {
            start,
            end,
            thickness,
        });
    };

    for col in cols {
        // Retire chains this column's index has passed beyond reach of.
        let mut i = 0;
        while i < open.len() {
            if col.0 > open[i].last().expect("non-empty").0 + gap_cells {
                emit(open.remove(i), &mut walls);
            } else {
                i += 1;
            }
        }
        let slot = open.iter_mut().find(|ch| {
            let mut lats: Vec<f64> = ch.iter().map(|c| c.1).collect();
            (col.1 - median(&mut lats)).abs() <= p.run_lateral_m
                && ch.last().expect("non-empty").0 < col.0
        });
        match slot {
            Some(ch) => ch.push(col),
            None => open.push(vec![col]),
        }
    }
    for chain in open {
        emit(chain, &mut walls);
    }
    walls
}

/// Roof-graze veto: the observation's slice height coincides with the cell's TOP surface and
/// that surface is sloped like a roof plane — the ray grazed the roof itself. A real attic wall
/// UNDER the roof keeps a clear top-minus-slice margin and survives (the first cut vetoed on
/// "roof anywhere overhead in the band" and killed genuine gable-end walls).
fn roof_veto(vert: &VerticalScan, cell: f64, x: f64, z: f64, slice_y: f64, p: &Params) -> bool {
    let ix = (x / cell) as usize;
    let iz = (z / cell) as usize;
    if ix >= vert.nx || iz >= vert.nz {
        return false;
    }
    let Some(top) = vert.top_at(ix, iz) else {
        return false;
    };
    if (top - slice_y).abs() > p.roof_graze_eps_m {
        return false;
    }
    let s = vert.slope_at(ix, iz);
    s >= p.roof_slope_lo && s <= p.roof_slope_hi
}

/// Plan cells covered by an over-thick interval, for the mass grid.
/// `x_axis`: interval runs along x (fixed iz = k) or along z (fixed ix = k).
fn thick_cells(mid: f64, k: usize, len: f64, cell: f64, x_axis: bool) -> Vec<(usize, usize)> {
    let half = len * 0.5;
    let c0 = (((mid - half) / cell).floor().max(0.0)) as usize;
    let c1 = ((mid + half) / cell).floor() as usize;
    (c0..=c1)
        .map(|c| if x_axis { (c, k) } else { (k, c) })
        .collect()
}

/// BFS flood fill from the grid border through non-wall cells; a wall touching the reached
/// outside region is exterior. Correct for L-shapes and courtyards, where the live bbox-extremes
/// test misclassifies.
fn classify_exterior_flood(walls: &[WallSeg], nx: usize, nz: usize, cell: f64) -> Vec<bool> {
    let mut solid = PlanGrid::new(nx, nz);
    let mut wall_cells: Vec<Vec<(usize, usize)>> = Vec::with_capacity(walls.len());
    for w in walls {
        let mut cells = Vec::new();
        let half = (w.thickness * 0.5 + cell * 0.5).max(cell * 0.5);
        let (x0, x1) = (
            w.start[0].min(w.end[0]) - half,
            w.start[0].max(w.end[0]) + half,
        );
        let (z0, z1) = (
            w.start[1].min(w.end[1]) - half,
            w.start[1].max(w.end[1]) + half,
        );
        let (cx0, cx1) = (
            (x0 / cell).floor().max(0.0) as usize,
            ((x1 / cell).ceil() as usize).min(nx),
        );
        let (cz0, cz1) = (
            (z0 / cell).floor().max(0.0) as usize,
            ((z1 / cell).ceil() as usize).min(nz),
        );
        for ix in cx0..cx1 {
            for iz in cz0..cz1 {
                solid.set(ix, iz, true);
                cells.push((ix, iz));
            }
        }
        wall_cells.push(cells);
    }

    let mut reached = vec![false; nx * nz];
    let mut queue: Vec<(usize, usize)> = Vec::new();
    for ix in 0..nx {
        for iz in [0, nz - 1] {
            if !solid.get(ix, iz) {
                queue.push((ix, iz));
            }
        }
    }
    for iz in 0..nz {
        for ix in [0, nx - 1] {
            if !solid.get(ix, iz) {
                queue.push((ix, iz));
            }
        }
    }
    while let Some((ix, iz)) = queue.pop() {
        let idx = ix * nz + iz;
        if reached[idx] {
            continue;
        }
        reached[idx] = true;
        let push = |jx: i64, jz: i64, queue: &mut Vec<(usize, usize)>| {
            if jx >= 0 && jz >= 0 && (jx as usize) < nx && (jz as usize) < nz {
                let (jx, jz) = (jx as usize, jz as usize);
                if !solid.get(jx, jz) && !reached[jx * nz + jz] {
                    queue.push((jx, jz));
                }
            }
        };
        push(ix as i64 + 1, iz as i64, &mut queue);
        push(ix as i64 - 1, iz as i64, &mut queue);
        push(ix as i64, iz as i64 + 1, &mut queue);
        push(ix as i64, iz as i64 - 1, &mut queue);
    }

    wall_cells
        .iter()
        .map(|cells| {
            cells.iter().any(|&(ix, iz)| {
                let neigh = [
                    (ix.wrapping_sub(1), iz),
                    (ix + 1, iz),
                    (ix, iz.wrapping_sub(1)),
                    (ix, iz + 1),
                ];
                neigh
                    .iter()
                    .any(|&(jx, jz)| jx < nx && jz < nz && reached[jx * nz + jz])
            })
        })
        .collect()
}

// ── grid algorithm (live port) ──────────────────────────────────────────────────────────────────

fn grid_band(
    dump: &VoxelDump,
    _vert: &VerticalScan,
    band_lo: f64,
    _band_hi: f64,
    p: &Params,
) -> BandWalls {
    let m = dump.meta();
    let cell = m.cell;
    let (nx, nz) = (m.dims[0], m.dims[2]);
    let ny = m.dims[1];
    let row_at = |y: f64| ((y / cell - 0.5).round().max(0.0) as usize).min(ny.saturating_sub(1));

    let lo = occupancy_at_row(dump, row_at(band_lo + p.band_low_m), nx, nz, cell, p);
    let hi = occupancy_at_row(dump, row_at(band_lo + p.band_high_m), nx, nz, cell, p);
    let mut grid = lo;
    for i in 0..grid.cells.len() {
        if !hi.cells[i] {
            grid.cells[i] = false;
        }
    }

    let rects = rects_from_grid(&grid, cell);
    let raw = rects.len();
    // Live extent test: exterior = rect within 0.3 m of the occupancy extremes.
    let (mut out_min_x, mut out_min_z, mut out_max_x, mut out_max_z) =
        (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for r in &rects {
        out_min_x = out_min_x.min(r[0]);
        out_min_z = out_min_z.min(r[1]);
        out_max_x = out_max_x.max(r[2]);
        out_max_z = out_max_z.max(r[3]);
    }

    let mut wall_rects = Vec::new();
    let mut masses = Vec::new();
    for r in rects {
        let (w, d) = (r[2] - r[0], r[3] - r[1]);
        if w.min(d) > p.wall_max_thickness_m {
            masses.push(MassRect { rect: r });
            continue;
        }
        if w.max(d) < p.min_feature_m {
            continue;
        }
        wall_rects.push(r);
    }
    merge_wall_rects(&mut wall_rects, p);

    let mut walls = Vec::new();
    let mut exterior = Vec::new();
    for r in &wall_rects {
        let (w, d) = (r[2] - r[0], r[3] - r[1]);
        let (cx, cz) = ((r[0] + r[2]) * 0.5, (r[1] + r[3]) * 0.5);
        let is_ext = (r[0] - out_min_x) < 0.3
            || (out_max_x - r[2]) < 0.3
            || (r[1] - out_min_z) < 0.3
            || (out_max_z - r[3]) < 0.3;
        let seg = if w >= d {
            WallSeg {
                start: [r[0], cz],
                end: [r[2], cz],
                thickness: w.min(d),
            }
        } else {
            WallSeg {
                start: [cx, r[1]],
                end: [cx, r[3]],
                thickness: w.min(d),
            }
        };
        walls.push(seg);
        exterior.push(is_ext);
    }
    BandWalls {
        walls,
        exterior,
        masses,
        raw_count: raw,
    }
}

/// ScanAxisX + ScanAxisZ cell marking at one slice row, verbatim live semantics: non-consuming
/// nearest-opposing pairing, unmatched forward face spans 0.09 m, unmatched opposing face marks
/// exactly its own cell.
fn occupancy_at_row(
    dump: &VoxelDump,
    row: usize,
    nx: usize,
    nz: usize,
    cell: f64,
    p: &Params,
) -> PlanGrid {
    let mut grid = PlanGrid::new(nx, nz);
    let empty: Vec<f64> = Vec::new();

    for iz in 0..nz {
        let fwd = dump.x_pos.get(&(row, iz)).unwrap_or(&empty);
        let opposing = dump
            .x_neg
            .get(&(row, iz))
            .map(|v| ascending(v))
            .unwrap_or_default();
        mark_live(
            fwd,
            &opposing,
            p,
            |c| {
                if c < nx {
                    grid.set(c, iz, true);
                }
            },
            cell,
        );
    }
    for ix in 0..nx {
        let fwd = dump.z_pos.get(&(ix, row)).unwrap_or(&empty);
        let opposing = dump
            .z_neg
            .get(&(ix, row))
            .map(|v| ascending(v))
            .unwrap_or_default();
        mark_live(
            fwd,
            &opposing,
            p,
            |c| {
                if c < nz {
                    grid.set(ix, c, true);
                }
            },
            cell,
        );
    }
    grid
}

fn mark_live(fwd: &[f64], opposing: &[f64], p: &Params, mut mark: impl FnMut(usize), cell: f64) {
    for &a in fwd {
        let mut b_best: Option<f64> = None;
        for &b in opposing {
            if b > a - p.pair_behind_m && b - a <= p.max_pair_m {
                b_best = Some(match b_best {
                    Some(cur) if cur <= b => cur,
                    _ => b,
                });
            }
        }
        let b = b_best.unwrap_or(a + cell * 0.9);
        let c0 = (a / cell).max(0.0) as usize;
        let c1 = (b / cell).max(0.0) as usize;
        for c in c0..=c1 {
            mark(c);
        }
    }
    for &b in opposing {
        let matched = fwd
            .iter()
            .any(|&a| b > a - p.pair_behind_m && b - a <= p.max_pair_m);
        if !matched && b >= 0.0 {
            mark((b / cell) as usize);
        }
    }
}

/// Greedy maximal-rect decomposition (RectsFromGrid port). Rects in normalized meters.
pub fn rects_from_grid(grid: &PlanGrid, cell: f64) -> Vec<[f64; 4]> {
    let (nx, nz) = (grid.nx, grid.nz);
    let mut used = vec![false; nx * nz];
    let mut rects = Vec::new();
    for ix in 0..nx {
        for iz in 0..nz {
            let idx = ix * nz + iz;
            if !grid.cells[idx] || used[idx] {
                continue;
            }
            let mut end_x = ix;
            while end_x + 1 < nx && grid.get(end_x + 1, iz) && !used[(end_x + 1) * nz + iz] {
                end_x += 1;
            }
            let mut end_z = iz;
            'grow: while end_z + 1 < nz {
                for cx in ix..=end_x {
                    let cidx = cx * nz + end_z + 1;
                    if !grid.cells[cidx] || used[cidx] {
                        break 'grow;
                    }
                }
                end_z += 1;
            }
            for cx in ix..=end_x {
                for cz in iz..=end_z {
                    used[cx * nz + cz] = true;
                }
            }
            rects.push([
                ix as f64 * cell,
                iz as f64 * cell,
                (end_x + 1) as f64 * cell,
                (end_z + 1) as f64 * cell,
            ]);
        }
    }
    rects
}

/// MergeWallRects port: fixed-point merge of collinear neighbors; the union must stay wall-shaped.
fn merge_wall_rects(rects: &mut Vec<[f64; 4]>, p: &Params) {
    let mut merged = true;
    let mut guard = 0;
    while merged && guard < 64 {
        merged = false;
        guard += 1;
        'outer: for i in 0..rects.len() {
            for j in (i + 1)..rects.len() {
                let (a, b) = (rects[i], rects[j]);
                let gap_x = a[0].max(b[0]) - a[2].min(b[2]);
                let gap_z = a[1].max(b[1]) - a[3].min(b[3]);
                let overlap_x = gap_x < -p.merge_overlap_m;
                let overlap_z = gap_z < -p.merge_overlap_m;
                let joinable =
                    (overlap_x && gap_z <= p.merge_gap_m) || (overlap_z && gap_x <= p.merge_gap_m);
                if !joinable {
                    continue;
                }
                let u = [
                    a[0].min(b[0]),
                    a[1].min(b[1]),
                    a[2].max(b[2]),
                    a[3].max(b[3]),
                ];
                if (u[2] - u[0]).min(u[3] - u[1]) > p.wall_max_thickness_m {
                    continue;
                }
                rects[i] = u;
                rects.remove(j);
                merged = true;
                break 'outer;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map_blueprint::{slabs, synth};

    fn vertical(d: &VoxelDump) -> VerticalScan {
        let m = d.meta();
        let mut p = Params::default();
        p.min_floor_y = -0.5 - m.origin[1];
        slabs::analyze(&d.y_down, m.dims, m.cell, m.span[1], &p)
    }

    fn band(d: &VoxelDump, v: &VerticalScan, algo: Algo) -> BandWalls {
        let p = Params::default();
        let lo = v.floors[0];
        let hi = v.eave.max(lo + p.top_band_min_m);
        extract_band(d, v, lo, hi, algo, &p, None)
    }

    #[test]
    fn box_room_yields_four_walls_both_algos() {
        let d = synth::box_room(6.0, 4.0, 2.6, 0.15);
        let v = vertical(&d);
        for algo in [Algo::Segments, Algo::Grid] {
            let bw = band(&d, &v, algo);
            assert_eq!(bw.walls.len(), 4, "{algo:?}: {:?}", bw.walls);
            assert!(bw.masses.is_empty(), "{algo:?} masses: {:?}", bw.masses);
            for w in &bw.walls {
                assert!(
                    w.thickness > 0.05 && w.thickness < 0.35,
                    "{algo:?} thickness {w:?}"
                );
            }
        }
    }

    #[test]
    fn segments_box_walls_are_centerline_accurate() {
        let d = synth::box_room(6.0, 4.0, 2.6, 0.15);
        let v = vertical(&d);
        let bw = band(&d, &v, Algo::Segments);
        // West wall local x in [0, 0.15] → normalized center 0.6 + 0.075.
        let west = bw
            .walls
            .iter()
            .find(|w| (w.start[0] - w.end[0]).abs() < 1e-9 && w.start[0] < 1.0)
            .expect("west wall");
        assert!(
            (west.start[0] - 0.675).abs() <= 0.05,
            "centerline {}",
            west.start[0]
        );
        let all_ext = bw.exterior.iter().all(|e| *e);
        assert!(all_ext, "single box: every wall exterior");
    }

    #[test]
    fn doorway_splits_wall_and_does_not_bridge() {
        let d = synth::box_with_door(6.0, 4.0, 2.6, 0.15, 2.4, 0.9);
        let v = vertical(&d);
        let bw = band(&d, &v, Algo::Segments);
        // The south wall (z-const at z≈0.675) must appear as exactly 2 runs with a ~0.9 m gap.
        let south: Vec<_> = bw
            .walls
            .iter()
            .filter(|w| (w.start[1] - w.end[1]).abs() < 1e-9 && w.start[1] < 1.0)
            .collect();
        assert_eq!(south.len(), 2, "south wall runs: {:?}", bw.walls);
        let mut xs: Vec<(f64, f64)> = south
            .iter()
            .map(|w| (w.start[0].min(w.end[0]), w.start[0].max(w.end[0])))
            .collect();
        xs.sort_by(|a, b| a.0.total_cmp(&b.0));
        let gap = xs[1].0 - xs[0].1;
        assert!((gap - 0.9).abs() <= 0.25, "doorway gap {gap}");
    }

    #[test]
    fn gable_second_band_emits_gable_ends_and_zero_roof_phantoms() {
        let d = synth::gable_box(6.0, 4.0, 2.6, 4.2, 0.15);
        let v = vertical(&d);
        let p = Params::default();
        // Second band: eave slab does not exist — synthesize the attic band [eave, ridge].
        let bw = extract_band(&d, &v, v.eave, v.ridge, Algo::Segments, &p, None);
        let z_running: Vec<_> = bw
            .walls
            .iter()
            .filter(|w| (w.start[0] - w.end[0]).abs() < 1e-9)
            .collect();
        let x_running: Vec<_> = bw
            .walls
            .iter()
            .filter(|w| (w.start[1] - w.end[1]).abs() < 1e-9)
            .collect();
        assert_eq!(z_running.len(), 2, "gable ends: {:?}", bw.walls);
        assert!(
            x_running.is_empty(),
            "sloped roof planes must not become walls: {x_running:?}"
        );
    }

    #[test]
    fn steep_graze_grid_phantoms_segments_clean() {
        // The regression pair that justifies the default: a steep roof graze drifts too little
        // for the live two-height AND to catch (same 0.1 m cell at both probe heights) but far
        // too much for the stationarity veto across the segments slice window. The graze sits at
        // x ≈ 3.0-3.2 normalized — well clear of the real walls at 0.675 and 6.525 — so the
        // assertion isolates the phantom region while both algos keep the real geometry.
        let d = synth::steep_graze();
        let v = vertical(&d);
        let p = Params::default();
        assert_eq!(v.floors.len(), 2, "two-story synth: {:?}", v.floors);
        let lo = v.floors[1];
        let hi = v.eave.max(lo + p.top_band_min_m);
        let in_phantom_zone = |w: &WallSeg| {
            (w.start[0] - w.end[0]).abs() < 1e-9 && w.start[0] > 2.5 && w.start[0] < 3.7
        };
        let grid = extract_band(&d, &v, lo, hi, Algo::Grid, &p, None);
        let seg = extract_band(&d, &v, lo, hi, Algo::Segments, &p, None);
        assert!(
            grid.walls.iter().any(in_phantom_zone),
            "the graze must fool the live AND (else this regression tests nothing): {:?}",
            grid.walls
        );
        assert!(
            !seg.walls.iter().any(in_phantom_zone),
            "segments must reject the graze: {:?}",
            seg.walls
        );
    }

    /// The phantom counter-guard for the per-column denominator: a column whose roof-clipped
    /// window is tiny cannot be captured by 1–2 stationary noise observations —
    /// `min_persist_rows` (3) floors the requirement.
    #[test]
    fn sparse_noise_fails_min_persist_rows_floor() {
        let p = Params::default();
        let avail = |_c: f64, _k: usize| 3usize; // heavily roof-clipped column
        let mk = |rows: &[usize]| -> Vec<Obs> {
            rows.iter()
                .map(|&row| Obs {
                    row,
                    center: 1.0,
                    thick: 0.1,
                })
                .collect()
        };
        // Two stationary observations: need = max(3, ceil(3·0.6) = 2) = 3 → rejected.
        let mut obs = mk(&[0, 1]);
        let cols = cluster_columns(&mut obs, 5, 16, &avail, "z-running", &p, None);
        assert!(cols.is_empty(), "2 rows must fail the absolute floor");
        // Three observations → accepted.
        let mut obs = mk(&[0, 1, 2]);
        let cols = cluster_columns(&mut obs, 5, 16, &avail, "z-running", &p, None);
        assert_eq!(cols.len(), 1);
    }
}
