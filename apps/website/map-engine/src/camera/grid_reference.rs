//! Role: the map furniture's grid reference — the text printed on a map pane's edge labels.
//! Position: `camera` in the map engine.
//! Signals & state: none; pure formatting over world metres.
//! Invariants: ONE convention. A second spelling of "which grid square is this" that disagreed
//! with the on-screen labels would be a confident wrong answer, which is worse than none.

/// The drawn grid's line spacing in world metres — the procedural 1 km grid. Grid-reference labels
/// enumerate lines at world multiples of this, so a label can never drift off the line it names.
pub const GRID_STEP_M: f64 = 1000.0;

/// Arma grid reference for a world coordinate: 3-digit **hundreds-of-metres**, wrapping every
/// 100 km (`floor(m / 100) mod 1000`, zero-padded). E.g. `6400 m → 064`, `12000 m → 120`,
/// `0 m → 000`. This is the half a single axis contributes to a six-figure military grid.
/// Negative or non-finite yields `000` — the off-terrain guard.
#[must_use]
pub fn grid_ref_3digit(world_m: f64) -> String {
    if !world_m.is_finite() || world_m < 0.0 {
        return "000".to_string();
    }
    let hundreds = (world_m / 100.0).floor() as i64;
    let wrapped = hundreds.rem_euclid(1000);
    format!("{wrapped:03}")
}

/// The world coordinates of grid lines (multiples of [`GRID_STEP_M`]) within `[lo, hi]` world
/// metres, inclusive — the eastings/northings whose lines cross a map-pane edge. `lo`/`hi` are the
/// visible world span of that edge; the returned values are exactly the drawn line positions, so
/// labelling them can never drift from the grid.
#[must_use]
pub fn grid_lines_in_range(lo: f64, hi: f64) -> Vec<f64> {
    let (lo, hi) = if lo <= hi { (lo, hi) } else { (hi, lo) };
    if !lo.is_finite() || !hi.is_finite() {
        return Vec::new();
    }
    let first_k = (lo / GRID_STEP_M).ceil() as i64;
    let last_k = (hi / GRID_STEP_M).floor() as i64;
    if last_k < first_k {
        return Vec::new();
    }
    (first_k..=last_k).map(|k| k as f64 * GRID_STEP_M).collect()
}
