//! Role: decide and phrase the terrain half of a sight line.
//! Position: `editing/tools/line_of_sight` in the map engine.
//! Signals & state: session-local measurement state; never the authored document.
//! Invariants: terrain blocks only when it rises strictly above the eye-to-eye line by more than the epsilon; the nearest such sample is the reported block.

use crate::spatial::los::terrain::sampler::ProfileSample;

// ── Decision 1 — eye-height constants (named, adjustable later) ──────────────────────────────────

/// Observer eye height above the ground, metres — a standing soldier (Decision 1). The sight line
/// starts here, not at the observer's feet. A later prone/vehicle preset changes THIS, not the math.
pub const EYE_HEIGHT_OBSERVER_M: f64 = 1.8;

/// Target eye height above the ground, metres — the point being observed is also a standing soldier
/// (Decision 1). The sight line ends here. Kept as its own constant (not shared with the observer)
/// so an asymmetric preset — e.g. spotting a prone target — is a one-line change.
pub const EYE_HEIGHT_TARGET_M: f64 = 1.8;

/// Occlusion epsilon, metres: terrain counts as blocking only when it rises STRICTLY above the sight
/// line by more than this (`terrain > line + EPS`). Guards the grazing case — terrain that just
/// touches the line (float noise, or a ridge exactly at eye level) reads CLEAR, not blocked — so a
/// hair of sampling jitter never flips a clear sight to blocked.
pub const OCCLUSION_EPS_M: f64 = 0.01;

// ── Pure occlusion core (native-tested) ─────────────────────────────────────────────────────────

/// The interpolated height of the sight line at along-segment distance `d`. The line runs from
/// `(0, eye0)` to `(total, eye1)` in (distance, elevation) space — `eye0`/`eye1` are the observer's
/// and target's EYE elevations (ground + eye height), `total` the segment length. Linear
/// interpolation: `eye0 + (eye1 − eye0) · d / total`. A zero-length segment (`total ≤ 0`) is defined
/// as `eye0` (no distance to interpolate over).
#[must_use]
pub fn sight_line_height(d: f64, total: f64, eye0: f64, eye1: f64) -> f64 {
    if total <= 0.0 {
        return eye0;
    }
    eye0 + (eye1 - eye0) * (d / total)
}

/// The verdict of a line-of-sight check between an observer and a target.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LosVerdict {
    /// The sight line clears the terrain end-to-end.
    Clear,
    /// Terrain rises above the sight line. `blocking_dist_m` is the along-segment distance (from the
    /// observer) of the FIRST blocking sample; `blocking_elev_m` the terrain elevation there.
    Blocked {
        blocking_dist_m: f64,
        blocking_elev_m: f64,
    },
    /// No usable profile — the segment is entirely off DEM coverage (fewer than two samples), so the
    /// sight cannot be judged. An honest "unknown", never a fake CLEAR (the em-dash policy).
    Unknown,
}

impl LosVerdict {
    /// True only for a definite CLEAR (convenience for the button/overlay tint).
    #[must_use]
    pub fn is_clear(self) -> bool {
        matches!(self, LosVerdict::Clear)
    }
    /// True for a definite BLOCKED.
    #[must_use]
    pub fn is_blocked(self) -> bool {
        matches!(self, LosVerdict::Blocked { .. })
    }
}

/// Decide occlusion over a terrain `profile` (from [`sample_segment`]): the observer stands at the
/// profile's FIRST sample, the target at its LAST, each with its eye height added. The sight line
/// runs eye-to-eye; the segment is BLOCKED iff any sampled terrain elevation is STRICTLY above the
/// line by more than [`OCCLUSION_EPS_M`], and the FIRST such sample (nearest the observer) is
/// reported.
///
/// The profile's own endpoints ARE tested, but they can never self-block: at distance 0 the line
/// sits `EYE_HEIGHT_OBSERVER_M` ABOVE the ground sample, and at `total` it sits
/// `EYE_HEIGHT_TARGET_M` above — both well over the epsilon — so a two-sample flat profile reads
/// CLEAR (as it must). A profile with fewer than two samples is [`LosVerdict::Unknown`] (off
/// coverage). `eye_obs`/`eye_tgt` are passed in (defaulting to the module constants at the call
/// site) so a preset can vary them without touching this function.
#[must_use]
pub fn occlusion(profile: &[ProfileSample], eye_obs: f64, eye_tgt: f64) -> LosVerdict {
    if profile.len() < 2 {
        return LosVerdict::Unknown;
    }
    // Observer/target EYE elevations = their ground sample + eye height. The line spans [0, total]
    // in distance; `total` is the last sample's distance (the segment length the sampler recorded).
    let eye0 = profile[0].elev_m + eye_obs;
    let total = profile[profile.len() - 1].dist_m;
    let eye1 = profile[profile.len() - 1].elev_m + eye_tgt;

    for s in profile {
        let line = sight_line_height(s.dist_m, total, eye0, eye1);
        // STRICTLY above the line (Decision / spec): terrain must exceed the line by > EPS to block.
        // Grazing (terrain == line, or within EPS) stays CLEAR.
        if s.elev_m > line + OCCLUSION_EPS_M {
            return LosVerdict::Blocked {
                blocking_dist_m: s.dist_m,
                blocking_elev_m: s.elev_m,
            };
        }
    }
    LosVerdict::Clear
}

// ── Formatting (native-tested goldens) ──────────────────────────────────────────────────────────

/// Format an along-segment distance for the LoS readout: sub-1000 m as whole metres (`"412 m"`),
/// ≥1000 m as km with two decimals (`"1.24 km"`). Same shape as the ruler's `format_leg_distance`
/// so the two tools' distances read alike.
#[must_use]
pub fn format_distance(m: f64) -> String {
    if m >= 1000.0 {
        format!("{:.2} km", m / 1000.0)
    } else {
        format!("{} m", m.round() as i64)
    }
}

/// The one-line verdict string for the status bar / panel header (Decision 2):
///   * clear      → `"LoS clear · 1.24 km"` (the total sight distance)
///   * blocked    → `"LoS blocked at 412 m"` (the first blocking distance)
///   * unknown    → `"LoS —"` (off coverage; the em-dash, never a fake verdict)
///
/// `total_m` is the full observer→target distance (shown on a clear sight so the operator reads how
/// far the clear line runs).
#[must_use]
pub fn format_verdict(v: LosVerdict, total_m: f64) -> String {
    match v {
        LosVerdict::Clear => format!("LoS clear · {}", format_distance(total_m)),
        LosVerdict::Blocked {
            blocking_dist_m, ..
        } => format!("LoS blocked at {}", format_distance(blocking_dist_m)),
        LosVerdict::Unknown => "LoS —".to_string(),
    }
}

#[cfg(test)]
#[path = "tests/terrain_verdict.rs"]
mod tests;
