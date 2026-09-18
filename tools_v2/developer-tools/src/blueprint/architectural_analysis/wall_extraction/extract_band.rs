use super::*;

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

/// Slice rows (iy) whose center height falls in the band's observation window.
pub(super) fn slice_rows(
    band_lo: f64,
    band_hi: f64,
    cell: f64,
    ny: usize,
    p: &Params,
) -> Vec<usize> {
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

pub(super) fn median(sorted_input: &mut [f64]) -> f64 {
    sorted_input.sort_by(f64::total_cmp);
    sorted_input[sorted_input.len() / 2]
}

pub(super) fn segments_band(
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
    if let Some(dbg) = debug {
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
pub(super) fn roof_clipped_rows(
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
pub(super) fn collect_columns(
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
pub(super) fn collect_columns_zaxis(
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
pub(super) fn cluster_columns(
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
pub(super) fn merge_columns(
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
pub(super) fn roof_veto(
    vert: &VerticalScan,
    cell: f64,
    x: f64,
    z: f64,
    slice_y: f64,
    p: &Params,
) -> bool {
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
pub(super) fn thick_cells(
    mid: f64,
    k: usize,
    len: f64,
    cell: f64,
    x_axis: bool,
) -> Vec<(usize, usize)> {
    let half = len * 0.5;
    let c0 = (((mid - half) / cell).floor().max(0.0)) as usize;
    let c1 = ((mid + half) / cell).floor() as usize;
    (c0..=c1)
        .map(|c| if x_axis { (c, k) } else { (k, c) })
        .collect()
}
