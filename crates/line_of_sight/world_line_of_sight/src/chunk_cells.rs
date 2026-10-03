//! The chunk cells a segment crosses, in order.
//!
//! **Role:** [`cells_on_segment`] walks the grid of square chunks under a 2-D segment cell by cell
//! (a digital differential analyser), first to last.
//! **Position:** the world occluder's chunk walk ([`crate::raycast`]) lists the chunks a query
//! crosses through it, in the chunk size the host streams (512 m in the browser).
//! **Signals & state:** none; a pure function.
//! **Invariants:** the cells equal the brute-force rasteriser's, ordered by the segment's entry
//! into each; cells off the terrain are skipped while the walk continues; a zero-length segment
//! yields its own cell.

/// Cells `(cx, cy)` of `cell_m`-sized chunks the 2-D segment `a→b` crosses, first to last,
/// endpoints inclusive, restricted to `0 ≤ cx < cols`, `0 ≤ cy < rows` (cells off the terrain are
/// skipped, the walk continues). A zero-length segment yields its own cell.
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

#[cfg(test)]
#[path = "tests/chunk_cells_tests.rs"]
mod tests;
