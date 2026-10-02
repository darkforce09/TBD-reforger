//! Role: project a placed shot and its elevation profile into screen space.
//! Position: `editing/tools/line_of_sight` in the map engine.
//! Signals & state: session-local measurement state; never the authored document.
//! Invariants: panel identity is the shot's world endpoints quantised to 0.1 m, so re-aiming never retains a stale panel; the chart spans both the ground and the sight line.

use crate::spatial::los::terrain::sampler::ProfileSample;

use super::capture::LosShot;
use super::object_verdict::ObjectVerdict;
use super::terrain_verdict::{LosVerdict, occlusion};

// ── The DOM/SVG overlay projection helpers (native-tested; the overlay draws from these) ─────────
//
// RENDERING LANE: a LoS shot is TWO points + a small profile panel — a GPU lane would be far more
// machinery than the geometry warrants. The ruler proves DOM/SVG overlay is the house idiom for
// transient camera-projected geometry at this scale, so LoS draws the same way: one absolutely
// positioned SVG (the observer→target line + endpoint dots) plus one inline profile panel by the
// target. Reactive off the same cursor/heartbeat channel the ruler + scale bar use — NO new rAF loop.

/// The projected geometry of a placed LoS shot, ready to draw: the line's screen endpoints, the two
/// world-key anchors, and the verdict. Built by [`project_shot`] from the shot + a world→pixel
/// projector + the live profile.
#[derive(Clone, Debug, PartialEq)]
pub struct ProjectedShot {
    /// Observer screen pixel.
    pub obs_px: f64,
    pub obs_py: f64,
    /// Target screen pixel (the profile panel anchors here — Decision 2).
    pub tgt_px: f64,
    pub tgt_py: f64,
    /// The clear/blocked verdict.
    pub verdict: LosVerdict,
    /// Full observer→target ground distance (metres).
    pub total_m: f64,
    /// The blocking point's screen pixel, `None` unless blocked. A marker dot on the line.
    pub block_px: Option<(f64, f64)>,
    /// T-090.12.5 — the object layer's verdict (`NotLoaded` until the wasm overlay attaches
    /// it through `los_world::apply_objects`, which also moves `block_px` to the nearer block).
    pub objects: ObjectVerdict,
    /// Stable key: the shot's two world endpoints quantised to 0.1 m (T-727) — ties the DOM nodes to
    /// WHERE the shot is, so re-placing the target never retains a stale panel.
    pub key: String,
}

/// A key string from a world coordinate pair, quantised to 0.1 m so float noise between frames does
/// not churn the key. Same rule as `ruler_tool::world_key` (kept local so `los_tool` has no
/// cross-tool dependency).
#[must_use]
pub fn world_key(ax: f64, ay: f64, bx: f64, by: f64) -> String {
    format!(
        "{}:{}:{}:{}",
        (ax * 10.0).round() as i64,
        (ay * 10.0).round() as i64,
        (bx * 10.0).round() as i64,
        (by * 10.0).round() as i64,
    )
}

/// Project a placed shot to screen space via a world→pixel projector (the live `OrthoCamera::project`
/// on wasm; injected here so this is pure + native-testable). `profile` is the terrain profile
/// between the two points (from [`sample_segment`]); the verdict and blocking point are derived from
/// it with the given eye heights. `project` takes world `(x, y)` → screen `(px, py)`.
#[must_use]
pub fn project_shot<F>(
    shot: &LosShot,
    profile: &[ProfileSample],
    eye_obs: f64,
    eye_tgt: f64,
    project: F,
) -> ProjectedShot
where
    F: Fn(f64, f64) -> (f64, f64),
{
    let (obs_px, obs_py) = project(shot.obs_x, shot.obs_y);
    let (tgt_px, tgt_py) = project(shot.tgt_x, shot.tgt_y);
    let verdict = occlusion(profile, eye_obs, eye_tgt);
    let total_m = shot.distance_m();
    // The blocking point (if any) projected onto the line by its along-fraction of the total run.
    let block_px = match verdict {
        LosVerdict::Blocked {
            blocking_dist_m, ..
        } if total_m > 0.0 => {
            let t = (blocking_dist_m / total_m).clamp(0.0, 1.0);
            Some((
                obs_px + (tgt_px - obs_px) * t,
                obs_py + (tgt_py - obs_py) * t,
            ))
        }
        _ => None,
    };
    ProjectedShot {
        obs_px,
        obs_py,
        tgt_px,
        tgt_py,
        verdict,
        total_m,
        block_px,
        objects: ObjectVerdict::NotLoaded,
        key: world_key(shot.obs_x, shot.obs_y, shot.tgt_x, shot.tgt_y),
    }
}

/// Build the inline profile panel's polyline points (a small SVG elevation chart — Decision 2). The
/// profile's (dist, elev) samples are mapped into a `w × h` px box: distance → x across the width,
/// elevation → y (inverted, higher = up) scaled to the profile's own min/max with a small margin.
/// Returns `(ground_points, line_points)` as `"x,y x,y …"` SVG polyline strings — the ground curve
/// and the straight sight line over it — plus the y of the blocking marker if blocked. Empty strings
/// when there is nothing to chart (<2 samples).
///
/// This is the panel's whole geometry, pure + native-tested: the component is a thin `<svg>` wrapper
/// that drops these strings into two `<polyline>`s.
#[must_use]
pub fn profile_chart(
    profile: &[ProfileSample],
    eye_obs: f64,
    eye_tgt: f64,
    w: f64,
    h: f64,
) -> ProfileChart {
    if profile.len() < 2 {
        return ProfileChart::default();
    }
    let total = profile[profile.len() - 1].dist_m;
    let eye0 = profile[0].elev_m + eye_obs;
    let eye1 = profile[profile.len() - 1].elev_m + eye_tgt;

    // Vertical range spans BOTH the ground and the sight line (the line's eyes can sit above the
    // highest ground), so the whole picture fits. A tiny pad avoids clipping the extremes at the box
    // edge; a flat profile (min == max) gets a 1 m band so it draws as a mid-height line, not a
    // divide-by-zero.
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    for s in profile {
        lo = lo.min(s.elev_m);
        hi = hi.max(s.elev_m);
    }
    lo = lo.min(eye0).min(eye1);
    hi = hi.max(eye0).max(eye1);
    // Not `hi <= lo`: a NaN bound must also take the fallback band, and `<=` reads false for NaN.
    if hi.partial_cmp(&lo) != Some(std::cmp::Ordering::Greater) {
        lo -= 0.5;
        hi += 0.5;
    }
    let pad = (hi - lo) * 0.08;
    lo -= pad;
    hi += pad;
    let span = hi - lo;

    // Map (dist, elev) → (px, py). x across [0, w] by distance fraction; y inverted in [0, h].
    let map = |dist: f64, elev: f64| -> (f64, f64) {
        let x = if total > 0.0 { dist / total * w } else { 0.0 };
        let y = h - (elev - lo) / span * h;
        (x, y)
    };

    let ground: Vec<String> = profile
        .iter()
        .map(|s| {
            let (x, y) = map(s.dist_m, s.elev_m);
            format!("{x:.1},{y:.1}")
        })
        .collect();
    // The straight sight line: two points, eye-to-eye.
    let (lx0, ly0) = map(0.0, eye0);
    let (lx1, ly1) = map(total, eye1);
    let line = format!("{lx0:.1},{ly0:.1} {lx1:.1},{ly1:.1}");

    // Blocking marker y (on the ground curve at the blocking distance), if blocked.
    let block = match occlusion(profile, eye_obs, eye_tgt) {
        LosVerdict::Blocked {
            blocking_dist_m,
            blocking_elev_m,
        } => Some(map(blocking_dist_m, blocking_elev_m)),
        _ => None,
    };

    ProfileChart {
        ground: ground.join(" "),
        line,
        block,
    }
}

/// The pure geometry of the inline profile panel (Decision 2) — two SVG polyline strings + an
/// optional blocking marker. Built by [`profile_chart`]; the [`LosOverlay`] drops it into `<svg>`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ProfileChart {
    /// The terrain ground curve as an SVG `points` string (`"x,y x,y …"`).
    pub ground: String,
    /// The straight sight line as an SVG `points` string (two points).
    pub line: String,
    /// Screen `(x, y)` of the blocking point inside the chart box, `None` unless blocked.
    pub block: Option<(f64, f64)>,
}

#[cfg(test)]
#[path = "tests/projection.rs"]
mod tests;
