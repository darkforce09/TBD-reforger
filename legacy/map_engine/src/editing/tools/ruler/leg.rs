//! Role: a ruler vertex, the leg between two of them, and how each quantity reads.
//! Position: `editing/tools/ruler` in the map engine.
//! Signals & state: session-local measurement state; never the authored document.
//! Invariants: bearing is grid north measured clockwise; a leg with no elevation at either end reports no rise rather than a zero one. Every readout says what unit it is in.

/// A ruler vertex: world metres `(x, y)` plus an optional DEM elevation `z` (metres ASL, `None`
/// off-coverage). `z` is sampled at click time from the same grid the CUR-Z read-out uses so a
/// vertex records the ground it was placed on even after the camera pans away.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RulerPoint {
    pub x: f64,
    pub y: f64,
    pub z: Option<f64>,
}

impl RulerPoint {
    #[must_use]
    pub fn new(x: f64, y: f64, z: Option<f64>) -> Self {
        Self { x, y, z }
    }
}

/// Euclidean world-metre distance between two vertices (the leg's ground run — the horizontal
/// distance the bearing and slope are measured over). Plain 2-D; Z is the rise, not part of the run.
#[must_use]
pub fn distance_m(a: RulerPoint, b: RulerPoint) -> f64 {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    (dx * dx + dy * dy).sqrt()
}

/// Bearing A→B in **degrees clockwise from north**, the map convention (`mortar.rs` shares it):
/// world +Y is north (north-up, `flipY:false`), world +X is east, so bearing =
/// `atan2(east, north) = atan2(dx, dy)` wrapped to `[0, 360)`. The cardinals fall out exactly —
/// due north `(dx=0, dy>0) → 0.0`, east `→ 90`, south `→ 180`, west `→ 270` — and a zero-length leg
/// (`dx=dy=0`) is defined as `0.0` (a degenerate point has no direction; `atan2(0,0)` is `0` anyway).
#[must_use]
pub fn bearing_deg(a: RulerPoint, b: RulerPoint) -> f64 {
    let dx = b.x - a.x; // east
    let dy = b.y - a.y; // north
    let deg = dx.atan2(dy).to_degrees();
    // atan2 → (−180, 180]; wrap to [0, 360). rem_euclid keeps it in range for any input.
    deg.rem_euclid(360.0)
}

/// Δelevation A→B in metres (signed: `b.z − a.z`), or `None` if EITHER vertex is off DEM coverage.
/// An all-or-nothing gate rather than treating a missing sample as 0 — a leg either has a real rise
/// or it declines to guess (Decision 2 / the CUR-Z em-dash policy).
#[must_use]
pub fn delta_elev_m(a: RulerPoint, b: RulerPoint) -> Option<f64> {
    match (a.z, b.z) {
        (Some(za), Some(zb)) => Some(zb - za),
        _ => None,
    }
}

/// Slope over a leg as a **percentage** (`rise / run × 100`), or `None` when Δelevation is unknown
/// (off-coverage) or the run is ~0 (a vertical/degenerate leg has no meaningful grade). Run is the
/// horizontal ground distance ([`distance_m`]), so this is the true grade a vehicle would climb.
#[must_use]
pub fn slope_pct(a: RulerPoint, b: RulerPoint) -> Option<f64> {
    let rise = delta_elev_m(a, b)?;
    let run = distance_m(a, b);
    if run < 1e-6 {
        return None;
    }
    Some(rise / run * 100.0)
}

/// One measured leg — the fully-resolved numbers for the segment from vertex `i` to vertex `i+1`.
/// Built once by [`RulerChain::legs`] so the overlay (on-line labels) and the status bar (last-leg
/// + total) read the SAME figures — a label on the map can never disagree with the bar.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Leg {
    pub from: RulerPoint,
    pub to: RulerPoint,
    /// Ground run in metres.
    pub dist_m: f64,
    /// Bearing clockwise from north, degrees.
    pub bearing_deg: f64,
    /// Signed Δelevation in metres, `None` off DEM coverage.
    pub delta_elev_m: Option<f64>,
    /// Grade as a percentage, `None` off-coverage or on a ~0-run leg.
    pub slope_pct: Option<f64>,
}

impl Leg {
    #[must_use]
    pub(super) fn between(from: RulerPoint, to: RulerPoint) -> Self {
        Self {
            from,
            to,
            dist_m: distance_m(from, to),
            bearing_deg: bearing_deg(from, to),
            delta_elev_m: delta_elev_m(from, to),
            slope_pct: slope_pct(from, to),
        }
    }

    /// The mid-point of the leg in world metres — where the on-line label anchors (Decision 1).
    #[must_use]
    pub fn midpoint(&self) -> (f64, f64) {
        (
            (self.from.x + self.to.x) / 2.0,
            (self.from.y + self.to.y) / 2.0,
        )
    }

    /// The compact one-line leg readout, e.g. `"412 m · 073.2° · +8 m (2%)"`. The elevation clause
    /// is dropped entirely off DEM coverage (Decision 2) so the string is never padded with a fake
    /// rise. Bearing is zero-padded to three integer digits + one decimal (`073.2°`), the map idiom.
    #[must_use]
    pub fn label(&self) -> String {
        let base = format!(
            "{} · {}",
            format_leg_distance(self.dist_m),
            format_bearing(self.bearing_deg)
        );
        match (self.delta_elev_m, self.slope_pct) {
            (Some(dz), Some(sp)) => {
                format!("{base} · {} ({})", format_delta_elev(dz), format_slope(sp))
            }
            // Δelev known but run ~0 (no grade): show the rise without a percentage.
            (Some(dz), None) => format!("{base} · {}", format_delta_elev(dz)),
            _ => base,
        }
    }
}

/// Format a leg distance: sub-1000 m as whole metres (`"412 m"`), ≥1000 m as km with two decimals
/// (`"1.24 km"`) — the per-leg twin of the Σ-total format so a long leg and the total read alike.
#[must_use]
pub fn format_leg_distance(m: f64) -> String {
    if m >= 1000.0 {
        format!("{:.2} km", m / 1000.0)
    } else {
        format!("{} m", m.round() as i64)
    }
}

/// Format a bearing as `NNN.N°` — three integer digits, one decimal, clockwise from north
/// (`073.2°`, `090.0°`, `000.0°`). Zero-padding the integer part keeps a column of bearings aligned
/// and matches the six-figure military-grid idiom the map already speaks. A value landing on exactly
/// `360.0` after rounding (e.g. `359.97°`) wraps to `000.0` so it never prints the out-of-range 360.
#[must_use]
pub fn format_bearing(deg: f64) -> String {
    // Round to one decimal first, THEN wrap, so 359.97 → 360.0 → 000.0 (never "360.0").
    let mut d = (deg * 10.0).round() / 10.0;
    if d >= 360.0 {
        d -= 360.0;
    }
    format!("{d:05.1}°") // width 5 = "NNN.N" (3 int + dot + 1 dec), zero-padded.
}

/// Format a signed Δelevation, whole metres with an explicit sign: `"+8 m"`, `"-3 m"`, `"+0 m"`.
/// The sign is always shown so a climb and a descent are unmistakable at a glance.
#[must_use]
pub fn format_delta_elev(dz: f64) -> String {
    format!("{:+} m", dz.round() as i64)
}

/// Format a slope as an UNSIGNED whole-percent MAGNITUDE: `"2%"`, `"12%"`, `"5%"`. The ticket's
/// leg-label format is `"+8 m (2%)"` — the DIRECTION is already carried by the signed Δelevation
/// (`+8 m` / `-3 m`), so the grade in parentheses is a magnitude, not re-signed (that would be
/// redundant, and a descent's `-3 m (-5%)` reads worse than `-3 m (5%)`). Sub-1% rounds to `0%`
/// (a flat leg reads flat). Absolute value, so a −4.7% descent prints `5%`.
#[must_use]
pub fn format_slope(pct: f64) -> String {
    format!("{}%", pct.abs().round() as i64)
}

/// Format the running total: `"Σ 1.24 km"` (≥1000 m, two decimals) or `"Σ 850 m"` (sub-km, whole).
/// The Σ sigil marks it as the accumulated distance in the status bar, distinct from a single leg.
#[must_use]
pub fn format_total(total_m: f64) -> String {
    if total_m >= 1000.0 {
        format!("Σ {:.2} km", total_m / 1000.0)
    } else {
        format!("Σ {} m", total_m.round() as i64)
    }
}

// ── Tool-mode arbitration (how the third mode enters the gesture machine) ───────────────────────

#[cfg(test)]
#[path = "tests/leg.rs"]
mod tests;
