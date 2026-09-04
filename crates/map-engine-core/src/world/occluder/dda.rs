//! T-090.12.3 — which 512 m chunks a segment crosses, in order: a 2-D Amanatides–Woo walk over
//! the chunk grid on the horizontal plane. The occluder works in the engine frame
//! `[x, y_up, z_north]`, whose horizontal pair `(x, z_north)` is the map's `(x, y_north)` — the
//! same numbers the converter partitioned on (`floor(coord / 512)`).

/// Cells `(cx, cy)` of `cell_m`-sized chunks the 2-D segment `a→b` crosses, first to last,
/// endpoints inclusive, restricted to `0 ≤ cx < cols`, `0 ≤ cy < rows` (cells off the terrain
/// are skipped, the walk continues). A zero-length segment yields its own cell.
#[must_use]
pub fn cells_on_segment(
    a: [f64; 2],
    b: [f64; 2],
    cell_m: f64,
    cols: i64,
    rows: i64,
) -> Vec<(i64, i64)> {
    let mut out: Vec<(i64, i64)> = Vec::new();
    if cell_m.is_nan() || cell_m <= 0.0 || cols <= 0 || rows <= 0 {
        return out;
    }
    let cell = |v: f64| (v / cell_m).floor() as i64;
    let (mut cx, mut cy) = (cell(a[0]), cell(a[1]));
    let (ex, ey) = (cell(b[0]), cell(b[1]));
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    let step_x: i64 = if dx > 0.0 {
        1
    } else if dx < 0.0 {
        -1
    } else {
        0
    };
    let step_y: i64 = if dy > 0.0 {
        1
    } else if dy < 0.0 {
        -1
    } else {
        0
    };
    // Parametric distance (in units of the segment) to the next cell boundary per axis, and
    // the distance between boundaries.
    let (mut t_max_x, t_delta_x) = if step_x == 0 {
        (f64::INFINITY, f64::INFINITY)
    } else {
        let next = if step_x > 0 {
            (cx + 1) as f64 * cell_m
        } else {
            cx as f64 * cell_m
        };
        ((next - a[0]) / dx, (cell_m / dx).abs())
    };
    let (mut t_max_y, t_delta_y) = if step_y == 0 {
        (f64::INFINITY, f64::INFINITY)
    } else {
        let next = if step_y > 0 {
            (cy + 1) as f64 * cell_m
        } else {
            cy as f64 * cell_m
        };
        ((next - a[1]) / dy, (cell_m / dy).abs())
    };
    let push = |out: &mut Vec<(i64, i64)>, cx: i64, cy: i64| {
        if cx >= 0 && cx < cols && cy >= 0 && cy < rows {
            out.push((cx, cy));
        }
    };
    push(&mut out, cx, cy);
    // Bound the walk: never more steps than the Manhattan cell distance.
    let max_steps = (ex - cx).abs() + (ey - cy).abs();
    let mut steps = 0i64;
    while (cx, cy) != (ex, ey) && steps < max_steps {
        if t_max_x < t_max_y {
            t_max_x += t_delta_x;
            cx += step_x;
        } else {
            t_max_y += t_delta_y;
            cy += step_y;
        }
        push(&mut out, cx, cy);
        steps += 1;
    }
    out
}

/// Brute-force reference: every cell whose square the segment touches, ordered by the
/// parametric entry of the segment into the cell (ties by `(cx, cy)`).
#[must_use]
pub fn cells_on_segment_reference(
    a: [f64; 2],
    b: [f64; 2],
    cell_m: f64,
    cols: i64,
    rows: i64,
) -> Vec<(i64, i64)> {
    let mut hits: Vec<(f64, i64, i64)> = Vec::new();
    for cy in 0..rows {
        for cx in 0..cols {
            let lo = [cx as f64 * cell_m, cy as f64 * cell_m];
            let hi = [(cx + 1) as f64 * cell_m, (cy + 1) as f64 * cell_m];
            // 2-D slab test, half-open on the far edge like the floor partition.
            let mut t0 = 0.0f64;
            let mut t1 = 1.0f64;
            let mut miss = false;
            for k in 0..2 {
                let d = b[k] - a[k];
                if d.abs() < 1e-15 {
                    if a[k] < lo[k] || a[k] >= hi[k] {
                        miss = true;
                    }
                    continue;
                }
                let (mut ta, mut tb) = ((lo[k] - a[k]) / d, (hi[k] - a[k]) / d);
                if ta > tb {
                    core::mem::swap(&mut ta, &mut tb);
                }
                t0 = t0.max(ta);
                t1 = t1.min(tb);
            }
            if miss || t0 > t1 {
                continue;
            }
            // Exclude cells only touched at their far (exclusive) edge.
            let mid_t = 0.5 * (t0 + t1);
            let p = [a[0] + mid_t * (b[0] - a[0]), a[1] + mid_t * (b[1] - a[1])];
            if p[0] < lo[0] || p[0] >= hi[0] || p[1] < lo[1] || p[1] >= hi[1] {
                continue;
            }
            hits.push((t0, cx, cy));
        }
    }
    hits.sort_by(|x, y| x.0.total_cmp(&y.0).then(x.1.cmp(&y.1)).then(x.2.cmp(&y.2)));
    hits.into_iter().map(|(_, cx, cy)| (cx, cy)).collect()
}
