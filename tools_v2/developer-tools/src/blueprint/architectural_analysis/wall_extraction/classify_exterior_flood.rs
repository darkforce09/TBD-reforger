use super::*;

/// BFS flood fill from the grid border through non-wall cells; a wall touching the reached
/// outside region is exterior. Correct for L-shapes and courtyards, where the live bbox-extremes
/// test misclassifies.
pub(super) fn classify_exterior_flood(
    walls: &[WallSeg],
    nx: usize,
    nz: usize,
    cell: f64,
) -> Vec<bool> {
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

pub(super) fn grid_band(
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
pub(super) fn occupancy_at_row(
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

pub(super) fn mark_live(
    fwd: &[f64],
    opposing: &[f64],
    p: &Params,
    mut mark: impl FnMut(usize),
    cell: f64,
) {
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
pub(super) fn merge_wall_rects(rects: &mut Vec<[f64; 4]>, p: &Params) {
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
