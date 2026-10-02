//! Role: rotation.
//! Position: `doc/operations` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

/// Canonical rotate ladder deg value.
pub const ROTATE_LADDER_DEG: [f64; 4] = [0.0, 5.0, 15.0, 45.0];

/// Quantise `value` to the nearest multiple of `step`. `step <= 0` (the OFF rung) is a **passthrough** — the value is returned exactly, which is how "snap off" reads at the call site with no branch. Round-half-away-from-zero so a delta exactly between two cells lands on the farther one symmetrically for + and −. Non-finite `step` is treated as OFF.
#[must_use]
pub fn snap_value(value: f64, step: f64) -> f64 {
    if !step.is_finite() || step <= 0.0 {
        return value;
    }
    (value / step).round() * step
}

/// Quantise a ROTATION (degrees) at ladder rung `rung`, then normalise to `[0,360)` (the same range `update_slot_position` stores). Rung 0 = OFF = the exact bearing, still normalised.
#[must_use]
pub fn snap_rotate(deg: f64, rung: usize) -> f64 {
    let snapped = snap_value(deg, *ROTATE_LADDER_DEG.get(rung).unwrap_or(&0.0));
    norm_deg(snapped)
}

/// Normalise degrees into `[0,360)` — the canonical rotation range (matches `MissionDocCore::update_slot_position`, which does `((r % 360)+360)%360`). Non-finite → 0.
#[must_use]
pub fn norm_deg(deg: f64) -> f64 {
    if !deg.is_finite() {
        return 0.0;
    }
    ((deg % 360.0) + 360.0) % 360.0
}

/// Bearing to face using the supplied domain data.
#[must_use]
pub fn bearing_to_face(from_x: f64, from_y: f64, to_x: f64, to_y: f64) -> Option<f64> {
    let dx = to_x - from_x;
    let dy = to_y - from_y;
    if !dx.is_finite() || !dy.is_finite() || (dx == 0.0 && dy == 0.0) {
        return None;
    }
    Some(norm_deg(dx.atan2(dy).to_degrees()))
}
