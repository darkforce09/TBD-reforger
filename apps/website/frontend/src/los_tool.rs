//! T-643 — Line of Sight: a point-to-point ray. Operator-requested; the SECOND dead button the
//! wave-105 split left in `eden_toolbelt`'s [`ModeToolbar`] (`visibility`, next to the ruler T-642
//! enabled in wave 108). Click an OBSERVER, click a TARGET → a verdict (clear / blocked) plus the
//! terrain PROFILE between them, drawn as an inline overlay panel by the target.
//!
//! Ships BEFORE the viewshed (T-644, wave 110), which reuses the segment sampler: the ground walk is
//! [`map_engine_core::dem::sample::sample_segment`] (core, reusable), and only the OCCLUSION /
//! sight-line math lives here so the viewshed can raycast with the same profile without copying it.
//!
//! ── Module shape (mirrors `ruler_tool` / `eden_toolbelt`) ───────────────────────────────────────
//! Everything decidable is a **pure function or a pure state machine** ([`sight_line_height`],
//! [`occlusion`], [`LosState`], the formatters) so a native `cargo test -p website-frontend` proves
//! the geometry with no browser — this file is UNGATED (declared `mod los_tool;` with no
//! `#[cfg(target_arch = "wasm32")]` in `main.rs`), exactly so those tests run on the same command CI
//! uses. The Leptos [`LosOverlay`] compiles on native but renders nothing there (no engine, no
//! `window`) — the `ruler_tool::RulerOverlay` / `MapGridRefs` idiom.
//!
//! ── THE FOUR DECISIONS THIS TICKET LEFT OPEN (each made + justified where it lives) ────────────
//!
//! **Decision 1 — eye height.** Observer and target eyes both sit [`EYE_HEIGHT_OBSERVER_M`] /
//! [`EYE_HEIGHT_TARGET_M`] = 1.8 m above the ground (a standing soldier at each end). Named
//! constants, adjustable later (a future prone/vehicle preset flips these, not the math). The sight
//! line runs eye-to-eye; terrain in between must clear that line, not the ground endpoints.
//!
//! **Decision 2 — the profile is an INLINE overlay panel near the TARGET point.** A small SVG
//! elevation profile (the ground curve + the straight sight line over it) with the clear/blocked
//! verdict and, when blocked, the blocking-point distance. It is the DOM-overlay idiom (like the
//! ruler's on-line labels / `MapGridRefs`), keyed by WORLD COORDINATE (T-727) so re-placing the
//! target moves the right panel node and never retains a stale one.
//!
//! **Decision 3 — Esc mirrors the ruler's two-step.** First Esc drops the IN-PROGRESS capture (an
//! observer placed but no target yet); a second Esc — or switching the tool away — clears the placed
//! result. This reuses the ruler's EXISTING Esc entry point (the one keydown arm in `mission_editor`
//! that already reads `code().as_str()`), NOT a new `window` listener: T-726 (the window-Esc
//! pile-up) is pending, so both tools share one seam and the eventual T-726 fix covers both at once.
//!
//! **Decision 4 — the LoS result is SESSION overlay state, NOT doc state.** A sight check is a
//! MEASUREMENT, not mission content: it is session-local overlay state held app-side (like the
//! selection set + the ruler chain), NEVER the Y.Doc (`store.rs` gets NO LoS writes — see the
//! `no_los_doc_writes` pin). Reload → the map is clean; the compiled payload is byte-identical.
//!
//! ── The occlusion rule (the ticket's exact spec) ────────────────────────────────────────────────
//! The sight line runs from the observer's eye to the target's eye. At each sampled distance the
//! line's interpolated height is compared to the terrain elevation there; the segment is BLOCKED iff
//! any sampled terrain elevation is STRICTLY above the sight line (`terrain > line + EPS`, epsilon
//! [`OCCLUSION_EPS_M`] = 0.01 m). The FIRST such sample (nearest the observer) is the reported
//! blocking point. Earth curvature is IGNORED — at the 12.8 km Everon scale its drop (~13 m at the
//! far corner-to-corner, far less on a real sight) is within the DEM's own ±0.204 m-per-anchor noise
//! band times the sampling coarseness, and modelling it would be false precision for a planner.

#![allow(dead_code)] // the wasm host wires the live path; native `cargo test` proves the pure core.

use leptos::prelude::*;

use map_engine_core::dem::sample::ProfileSample;

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

// ── Tool-mode arbitration note ──────────────────────────────────────────────────────────────────
//
// LoS shares `ruler_tool::EditorTool` (the `LoS` variant) and the SAME `should_begin_ruler`
// point-capture predicate + `LG::Ruler` gesture arm — the "mode field on the ruler arm" the ticket
// sanctions, so no third `LeftGesture` variant is added to the un-owned `select_tool`. The commit
// site in `mission_editor` branches on `tool_mode.is_los()` to route a captured click into
// [`LosState::click`] instead of the ruler chain. See `ruler_tool::EditorTool::captures_points`.

// ── The two-click capture state machine (Decisions 3 + 4 live here) ─────────────────────────────

/// A placed line-of-sight measurement: observer + target world points, each with its clicked-time
/// DEM ground elevation. Session-local overlay state (Decision 4 — NOT the Y.Doc). The verdict +
/// profile are DERIVED (recomputed by the overlay from the live DEM) rather than stored, so a panned
/// camera never shows a stale sample — the two points are the only state.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LosShot {
    pub obs_x: f64,
    pub obs_y: f64,
    /// Observer ground elevation at click time (metres ASL), `None` off DEM coverage.
    pub obs_z: Option<f64>,
    pub tgt_x: f64,
    pub tgt_y: f64,
    /// Target ground elevation at click time, `None` off coverage.
    pub tgt_z: Option<f64>,
}

impl LosShot {
    /// Straight-line ground distance observer→target in world metres (the profile's total run).
    #[must_use]
    pub fn distance_m(&self) -> f64 {
        let dx = self.tgt_x - self.obs_x;
        let dy = self.tgt_y - self.obs_y;
        (dx * dx + dy * dy).sqrt()
    }
}

/// The LoS tool's capture phase + placed shot. Session-local overlay state (Decision 4). Two clicks
/// build one shot: the FIRST sets a pending observer; the SECOND completes the shot (and REPLACES any
/// previous one — a LoS check is a single ray, not a chain). A third click starts a fresh capture
/// (new observer), so the operator can re-aim without an explicit clear.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LosState {
    /// A placed observer awaiting its target (world x, y, z). `Some` between the first and second
    /// click; `None` once a shot completes or the capture is cleared.
    pub pending_obs: Option<(f64, f64, Option<f64>)>,
    /// The completed shot (observer + target), or `None` if none placed yet.
    pub shot: Option<LosShot>,
}

impl LosState {
    /// A fresh, empty state (nothing pending, nothing placed).
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// True when there is nothing to draw or dismiss (no pending observer and no placed shot).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.pending_obs.is_none() && self.shot.is_none()
    }

    /// Commit a click at world `(x, y, z)` — the two-click capture (Decisions 2/3):
    ///   * no pending observer → this click SETS the observer (and clears any previous shot, so a
    ///     new measure starts clean the moment the first point drops);
    ///   * a pending observer exists → this click SETS the target, completing the shot and clearing
    ///     `pending_obs`.
    ///
    /// Returns `true` when a full shot was just completed (the host uses this to know a verdict is
    /// now available — though the overlay recomputes it from the DEM regardless).
    pub fn click(&mut self, x: f64, y: f64, z: Option<f64>) -> bool {
        match self.pending_obs.take() {
            None => {
                // First point of a new capture: it becomes the observer; any old shot is retired so
                // the map shows only the measure being built.
                self.pending_obs = Some((x, y, z));
                self.shot = None;
                false
            }
            Some((ox, oy, oz)) => {
                // Second point: complete the shot.
                self.shot = Some(LosShot {
                    obs_x: ox,
                    obs_y: oy,
                    obs_z: oz,
                    tgt_x: x,
                    tgt_y: y,
                    tgt_z: z,
                });
                true
            }
        }
    }

    /// Escape — the two-step escalating dismissal (Decision 3, mirroring the ruler):
    ///   * a capture in progress (pending observer, no completed shot yet) → drop the pending
    ///     observer (first Esc abandons the half-placed measure);
    ///   * otherwise, a placed shot → clear it (second Esc / a placed-result dismissal).
    ///
    /// Returns `true` if it changed anything, so the host can `preventDefault` only on a real act
    /// (an Esc with no LoS falls through untouched — never swallowed).
    pub fn escape(&mut self) -> bool {
        if self.pending_obs.is_some() {
            // First Esc: abandon the in-progress capture. (A shot cannot coexist with a pending
            // observer — `click` clears `shot` when it sets a new observer — so this branch is the
            // pure "in progress" case.)
            self.pending_obs = None;
            true
        } else if self.shot.is_some() {
            // Second Esc (nothing pending): clear the placed result.
            self.shot = None;
            true
        } else {
            false
        }
    }

    /// Clear everything (tool-switch away from LoS is Decision 3's "second-Esc equivalent").
    /// Idempotent.
    pub fn clear(&mut self) {
        self.pending_obs = None;
        self.shot = None;
    }
}

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
    if !(hi > lo) {
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

// ── The leaked-state registry (the `ruler_tool::register_ruler_chain` idiom) ─────────────────────
//
// The overlay is mounted in the shared `view!` (outside `mission_editor`'s wasm block), but the live
// `LosState` is a leaked `Rc<RefCell<…>>` inside that block. The wasm host REGISTERS the state handle
// into a thread_local here (peer of `ruler_tool::register_ruler_chain` /
// `context_menu::set_menu_signal`), and the overlay reads a clone. thread_local + `Rc<RefCell<…>>`
// is `!Send`-safe (JS is single-threaded) and works on native too, so this stays ungated.

thread_local! {
    static LOS_STATE: std::cell::RefCell<Option<std::rc::Rc<std::cell::RefCell<LosState>>>> =
        const { std::cell::RefCell::new(None) };
    // The live DEM grid handle, so the overlay can rebuild the profile after a pan (the SAME 8 m
    // downsampled grid the ruler's per-vertex Z read uses — the reachable DEM in the editor). A
    // clone of the host's `DemGridHandle`. Untyped as `Rc<dyn Fn>` would be heavier than needed; the
    // overlay holds the sampler closure indirection instead (see `register_los_sampler`).
}

/// Register the host's leaked LoS state so [`LosOverlay`] can read it. Called once at mount by
/// `mission_editor` (peer of `ruler_tool::register_ruler_chain`).
pub fn register_los_state(state: std::rc::Rc<std::cell::RefCell<LosState>>) {
    LOS_STATE.with(|c| *c.borrow_mut() = Some(state));
}

/// A snapshot clone of the registered state (empty if none registered — e.g. native/pre-mount).
#[must_use]
pub fn read_registered_state() -> LosState {
    LOS_STATE.with(|c| {
        c.borrow()
            .as_ref()
            .map(|rc| rc.borrow().clone())
            .unwrap_or_default()
    })
}

// The DEM sampler the overlay uses to build the profile after a pan. Registered as a boxed closure
// so `los_tool` needs no compile-time dependency on the host's `DemGridHandle` type — the host wraps
// its grid handle into a `Fn(f64,f64)->Option<f64>` and hands it over (the same seam the profile
// tests exercise with a synthetic closure).
thread_local! {
    #[allow(clippy::type_complexity)]
    static LOS_SAMPLER: std::cell::RefCell<Option<std::rc::Rc<dyn Fn(f64, f64) -> Option<f64>>>> =
        const { std::cell::RefCell::new(None) };
}

/// Register the host's DEM point-sampler (world x,y → ground metres, `None` off coverage) so the
/// overlay can rebuild the profile as the camera pans. The host passes a closure over its live DEM
/// grid handle — the SAME `sample_grid_meters` the ruler's Z read uses.
pub fn register_los_sampler(sampler: std::rc::Rc<dyn Fn(f64, f64) -> Option<f64>>) {
    LOS_SAMPLER.with(|c| *c.borrow_mut() = Some(sampler));
}

/// A clone of the registered sampler (`None` if none registered — native/pre-mount).
#[must_use]
pub fn read_registered_sampler() -> Option<std::rc::Rc<dyn Fn(f64, f64) -> Option<f64>>> {
    LOS_SAMPLER.with(|c| c.borrow().clone())
}

// ── Overlay geometry constants ──────────────────────────────────────────────────────────────────

/// The DEM step (world metres) the overlay walks the profile at: `min(grid spacing, 8 m)` per the
/// ticket. The live 8 m downsampled grid means 8 m is the effective floor; a finer grid would use
/// its spacing. Kept ≤ the grid cell so no ridge between two samples is missed.
pub const PROFILE_STEP_M: f64 = 8.0;

/// The inline profile panel's chart box in CSS px (Decision 2 — a SMALL profile). Width/height of
/// the elevation curve area, excluding the header text.
pub const PANEL_W_PX: f64 = 180.0;
pub const PANEL_H_PX: f64 = 64.0;

/// T-643 — the Line-of-Sight overlay. ONE absolutely-positioned, `pointer-events-none` SVG spanning
/// the viewport drawing the observer→target sight line + endpoint dots + (when blocked) a marker at
/// the blocking point, PLUS an inline HTML profile panel anchored by the target (Decision 2). It
/// reads the live camera off the registered engine (`world_assets::camera_snapshot`, the same seam
/// the ruler / scale bar use), rebuilds the terrain profile via the registered DEM sampler, and
/// re-runs off the `cursor` (pan) + `debug_hud` (~1 Hz zoom) heartbeats + a `tick` bumped on every
/// state mutation — NO new rAF loop.
///
/// The state is read via [`read_registered_state`] — a cheap clone of the leaked host [`LosState`]
/// (session-local overlay state — Decision 4). Native builds render nothing (no engine, no
/// `window`); the geometry is proven by `project_shot` / `profile_chart` / `occlusion` above.
#[component]
pub fn LosOverlay(
    /// Pan heartbeat — the editor's pointer-move cursor write (drives the pan re-projection). Also
    /// the live cursor the rubber-band line draws to while an observer is placed but no target yet.
    cursor: RwSignal<Option<(f64, f64, Option<f64>)>>,
    /// ~1 Hz zoom heartbeat — the rAF debug sampler (a wheel-zoom with a still pointer re-projects
    /// within a second). `Option` so the mount can forward `Some(debug_hud)`.
    debug_hud: Option<RwSignal<String>>,
    /// Bumped by the host on every state mutation (click / Esc / tool-switch) so a click repaints
    /// even with a still pointer.
    tick: RwSignal<u64>,
) -> impl IntoView {
    // The overlay's three draw lists, ALL as `Vec` so each `<For each>` is a plain field access (no
    // turbofish inside the view macro): the placed shot (0 or 1 `ProjectedShot`), the rubber-band
    // line while capturing (0 or 1 `[x1,y1,x2,y2]`), and the profile-panel pairs (0 or 1
    // `(ProjectedShot, ProfileChart)`). All derived from the current state + camera + live DEM
    // sampler. One `derived()` call feeds three thin accessor closures below so the heavy compute
    // runs once per reactive tick.
    #[allow(clippy::type_complexity)]
    let derived = move || -> (
        Vec<ProjectedShot>,
        Vec<(f64, f64, f64, f64)>,
        Vec<(ProjectedShot, ProfileChart)>,
    ) {
        // Subscribe to all three heartbeats so the closure re-runs on pan (cursor), zoom (hud) and
        // any state edit (tick).
        let cur = cursor.get();
        if let Some(h) = debug_hud {
            let _ = h.get();
        }
        let _ = tick.get();
        let state = read_registered_state();
        if state.is_empty() {
            return (Vec::new(), Vec::new(), Vec::new());
        }
        #[cfg(target_arch = "wasm32")]
        {
            let Some((tx, ty, zoom)) = crate::world_assets::camera_snapshot() else {
                return (Vec::new(), Vec::new(), Vec::new());
            };
            let Some(win) = web_sys::window() else {
                return (Vec::new(), Vec::new(), Vec::new());
            };
            let vw = win.inner_width().ok().and_then(|v| v.as_f64()).unwrap_or(0.0);
            let vh = win.inner_height().ok().and_then(|v| v.as_f64()).unwrap_or(0.0);
            if vw <= 0.0 || vh <= 0.0 {
                return (Vec::new(), Vec::new(), Vec::new());
            }
            let cam = crate::select_tool::frozen_camera(vw, vh, tx, ty, zoom);
            let project = move |x: f64, y: f64| {
                let p = cam.project([x, y, 0.0]);
                (p[0], p[1])
            };
            // Rubber-band: a placed observer awaiting its target → line from the observer to the
            // live cursor, so the operator aims the target click.
            let mut rubber = Vec::new();
            if let (Some((ox, oy, _)), None) = (state.pending_obs, state.shot) {
                if let Some((cwx, cwy, _)) = cur {
                    let (x1, y1) = project(ox, oy);
                    let (x2, y2) = project(cwx, cwy);
                    rubber.push((x1, y1, x2, y2));
                }
            }
            // A completed shot → project it + build the profile panel from the live DEM sampler.
            let (mut shots, mut panels) = (Vec::new(), Vec::new());
            if let Some(shot) = state.shot {
                let profile = build_profile(&shot);
                let proj = project_shot(
                    &shot,
                    &profile,
                    EYE_HEIGHT_OBSERVER_M,
                    EYE_HEIGHT_TARGET_M,
                    &project,
                );
                let chart = profile_chart(
                    &profile,
                    EYE_HEIGHT_OBSERVER_M,
                    EYE_HEIGHT_TARGET_M,
                    PANEL_W_PX,
                    PANEL_H_PX,
                );
                shots.push(proj.clone());
                panels.push((proj, chart));
            }
            (shots, rubber, panels)
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = cur;
            (Vec::new(), Vec::new(), Vec::new())
        }
    };

    view! {
        // Full-bleed, non-interactive overlay. z-10 sits it in the same band as the ruler overlay /
        // MapGridRefs — over the map, under the chrome docks. `pointer-events-none` so it never eats
        // a map gesture (the click capture is the map's own pointer handlers, not this SVG).
        <div data-los-overlay class="pointer-events-none absolute inset-0 z-10">
            <svg class="absolute inset-0" width="100%" height="100%">
                // Rubber-band preview — observer → live cursor while awaiting the target (dashed).
                <For
                    each=move || derived().1
                    key=|rb| format!("{:.0}:{:.0}:{:.0}:{:.0}", rb.0, rb.1, rb.2, rb.3)
                    let:rb
                >
                    <line
                        x1=move || format!("{:.1}", rb.0)
                        y1=move || format!("{:.1}", rb.1)
                        x2=move || format!("{:.1}", rb.2)
                        y2=move || format!("{:.1}", rb.3)
                        class="stroke-primary/60"
                        stroke-width="1.5"
                        stroke-dasharray="4 4"
                    />
                </For>
                // The placed sight line + endpoint dots + blocking marker. Keyed by world coord
                // (T-727) so re-placing the target swaps the node cleanly. Colour encodes the verdict:
                // clear → success, blocked → error, unknown → neutral outline.
                <For
                    each=move || derived().0
                    key=|shot| shot.key.clone()
                    let:shot
                >
                    <line
                        x1=move || format!("{:.1}", shot.obs_px)
                        y1=move || format!("{:.1}", shot.obs_py)
                        x2=move || format!("{:.1}", shot.tgt_px)
                        y2=move || format!("{:.1}", shot.tgt_py)
                        class=los_line_class(shot.verdict)
                        stroke-width="1.5"
                    />
                    // Observer dot.
                    <circle
                        cx=move || format!("{:.1}", shot.obs_px)
                        cy=move || format!("{:.1}", shot.obs_py)
                        r="3"
                        class="fill-surface-container-lowest stroke-primary"
                        stroke-width="1.5"
                    />
                    // Target dot.
                    <circle
                        cx=move || format!("{:.1}", shot.tgt_px)
                        cy=move || format!("{:.1}", shot.tgt_py)
                        r="3"
                        class=los_dot_class(shot.verdict)
                        stroke-width="1.5"
                    />
                    // Blocking-point marker (only when blocked).
                    {shot.block_px.map(|(bx, by)| view! {
                        <circle
                            cx=format!("{bx:.1}")
                            cy=format!("{by:.1}")
                            r="4"
                            class="fill-error stroke-surface-container-lowest"
                            stroke-width="1.5"
                        />
                    })}
                </For>
            </svg>
            // The inline profile panel (Decision 2) — anchored by the TARGET point, keyed by world
            // coord. A small elevation chart (ground curve + sight line) with the verdict header.
            <For
                each=move || derived().2
                key=|(shot, _)| shot.key.clone()
                let:panel
            >
                {
                    let (shot, chart) = panel;
                    // Anchor the panel just above-right of the target; nudge so it doesn't sit on the
                    // dot. Positioned in CSS px from the projected target pixel.
                    let left = shot.tgt_px + 12.0;
                    let top = shot.tgt_py - PANEL_H_PX - 28.0;
                    let verdict_text = format_verdict(shot.verdict, shot.total_m);
                    view! {
                        <div
                            data-los-panel
                            class="absolute rounded-lg border border-white/10 bg-surface-container-lowest/85 px-2 py-1.5 shadow-xl backdrop-blur-xl"
                            style=format!("left:{left:.1}px;top:{top:.1}px;width:{:.0}px", PANEL_W_PX + 16.0)
                        >
                            <div class=los_header_class(shot.verdict)>
                                {verdict_text}
                            </div>
                            <svg
                                class="mt-1 block"
                                width=format!("{PANEL_W_PX:.0}")
                                height=format!("{PANEL_H_PX:.0}")
                            >
                                // The straight sight line (drawn first, under the ground).
                                <polyline
                                    points=chart.line.clone()
                                    fill="none"
                                    class=los_line_class(shot.verdict)
                                    stroke-width="1.5"
                                    stroke-dasharray="3 3"
                                />
                                // The terrain ground curve.
                                <polyline
                                    points=chart.ground.clone()
                                    fill="none"
                                    class="stroke-on-surface-variant"
                                    stroke-width="1.5"
                                />
                                // Blocking marker on the ground curve, when blocked.
                                {chart.block.map(|(bx, by)| view! {
                                    <circle
                                        cx=format!("{bx:.1}")
                                        cy=format!("{by:.1}")
                                        r="3"
                                        class="fill-error"
                                    />
                                })}
                            </svg>
                        </div>
                    }
                }
            </For>
        </div>
    }
}

/// The SVG stroke class for the sight LINE by verdict: clear → success, blocked → error, unknown →
/// neutral outline. A pure helper so the overlay closures stay terse and the mapping is one place.
#[must_use]
fn los_line_class(v: LosVerdict) -> &'static str {
    match v {
        LosVerdict::Clear => "stroke-success",
        LosVerdict::Blocked { .. } => "stroke-error",
        LosVerdict::Unknown => "stroke-outline",
    }
}

/// The target DOT fill/stroke class by verdict (mirrors the line colour on the fill).
#[must_use]
fn los_dot_class(v: LosVerdict) -> &'static str {
    match v {
        LosVerdict::Clear => "fill-success stroke-surface-container-lowest",
        LosVerdict::Blocked { .. } => "fill-error stroke-surface-container-lowest",
        LosVerdict::Unknown => "fill-outline stroke-surface-container-lowest",
    }
}

/// The panel HEADER text class by verdict (coloured verdict word).
#[must_use]
fn los_header_class(v: LosVerdict) -> &'static str {
    match v {
        LosVerdict::Clear => "font-mono text-code-md text-success",
        LosVerdict::Blocked { .. } => "font-mono text-code-md text-error",
        LosVerdict::Unknown => "font-mono text-code-md text-on-surface-variant",
    }
}

/// Build the terrain profile for a shot from the registered DEM sampler (the live path). Walks the
/// observer→target segment at [`PROFILE_STEP_M`] via [`sample_segment`], with the DEM sampler
/// injected from the host (`register_los_sampler`). Returns an empty profile when no sampler is
/// registered (native / pre-mount) — the overlay then renders no chart, and `occlusion` reads
/// [`LosVerdict::Unknown`].
///
/// The manifest is the full Everon coverage box (0..12800), matching the loaded DEM the ticket
/// names; the sampler itself gates finer coverage (returns `None` where the grid has no data), so
/// the profile is the covered subset either way.
#[must_use]
fn build_profile(shot: &LosShot) -> Vec<ProfileSample> {
    let Some(sampler) = read_registered_sampler() else {
        return Vec::new();
    };
    let manifest = everon_manifest();
    map_engine_core::dem::sample::sample_segment(
        &manifest,
        (shot.obs_x, shot.obs_y),
        (shot.tgt_x, shot.tgt_y),
        PROFILE_STEP_M,
        move |x, y| sampler(x, y),
    )
}

/// The Everon DEM coverage manifest (world box + raster dims) the live profile walk bounds itself
/// to. Only the world box + dims matter for [`sample_segment`] (it reads coverage via `in_coverage`
/// and calls the injected sampler for elevation), so the height range is the published Everon band.
/// Mirrors `packages/map-assets/everon/manifest.json` (the ±0.204 m-verified 6400² DEM).
#[must_use]
fn everon_manifest() -> map_engine_core::dem::sample::DemManifest {
    map_engine_core::dem::sample::DemManifest {
        min_x: 0.0,
        min_y: 0.0,
        max_x: 12_800.0,
        max_y: 12_800.0,
        width_px: 6400,
        height_px: 6400,
        flip_x: false,
        flip_z: false,
        height_min_m: -204.78,
        height_max_m: 375.53,
    }
}

// ── Tests: occlusion goldens, the sight-line math, formatters/verdict, the state machine, panel
//    keying + chart geometry, and the fired occlusion rule. Native — no browser/engine. ───────────

#[cfg(test)]
mod tests {
    use super::*;

    /// A profile sample builder for terse goldens.
    fn s(dist_m: f64, elev_m: f64) -> ProfileSample {
        ProfileSample { dist_m, elev_m }
    }

    // ── sight-line interpolation ──────────────────────────────────────────────────────────────────

    #[test]
    fn sight_line_interpolates_eye_to_eye() {
        // Observer eye 10 m, target eye 20 m, over 100 m: midpoint is 15 m.
        assert!((sight_line_height(0.0, 100.0, 10.0, 20.0) - 10.0).abs() < 1e-9);
        assert!((sight_line_height(50.0, 100.0, 10.0, 20.0) - 15.0).abs() < 1e-9);
        assert!((sight_line_height(100.0, 100.0, 10.0, 20.0) - 20.0).abs() < 1e-9);
        // Zero-length segment → the observer eye (no distance to interpolate over).
        assert!((sight_line_height(0.0, 0.0, 7.0, 99.0) - 7.0).abs() < 1e-9);
    }

    // ── occlusion goldens (the ticket's required table) ───────────────────────────────────────────

    /// Clear over flat ground: two endpoints at the same elevation. The line sits a full eye-height
    /// above the ground at both ends, so nothing blocks — CLEAR.
    #[test]
    fn occlusion_clear_flat() {
        let prof = [s(0.0, 100.0), s(50.0, 100.0), s(100.0, 100.0)];
        assert_eq!(
            occlusion(&prof, EYE_HEIGHT_OBSERVER_M, EYE_HEIGHT_TARGET_M),
            LosVerdict::Clear
        );
    }

    /// Blocked by a synthetic ridge: flat ends, a tall spike in the middle that pokes above the
    /// eye-to-eye line. The FIRST sample above the line is reported (the near side of the ridge).
    #[test]
    fn occlusion_blocked_by_ridge() {
        // Ground 100 m at the ends, a 150 m ridge at 400 m and 500 m; eyes at 101.8 m → the line is
        // ~101.8 m across (flat), so the 150 m ridge towers over it. First blocking sample at 400 m.
        let prof = [
            s(0.0, 100.0),
            s(200.0, 100.0),
            s(400.0, 150.0), // ridge near side — first above the line
            s(500.0, 150.0),
            s(800.0, 100.0),
        ];
        match occlusion(&prof, EYE_HEIGHT_OBSERVER_M, EYE_HEIGHT_TARGET_M) {
            LosVerdict::Blocked {
                blocking_dist_m,
                blocking_elev_m,
            } => {
                assert!(
                    (blocking_dist_m - 400.0).abs() < 1e-9,
                    "first ridge sample reported"
                );
                assert!((blocking_elev_m - 150.0).abs() < 1e-9);
            }
            other => panic!("expected Blocked, got {other:?}"),
        }
    }

    /// Grazing epsilon case: terrain that just TOUCHES the sight line reads CLEAR (strictly-above
    /// rule + epsilon), while terrain a hair over the epsilon reads BLOCKED. Proves the boundary.
    #[test]
    fn occlusion_grazing_epsilon() {
        // Flat eyes at 100 + 1.8 = 101.8 m; a mid sample exactly at the line height (101.8) grazes.
        let graze = [s(0.0, 100.0), s(50.0, 101.8), s(100.0, 100.0)];
        assert_eq!(
            occlusion(&graze, EYE_HEIGHT_OBSERVER_M, EYE_HEIGHT_TARGET_M),
            LosVerdict::Clear,
            "terrain exactly at the sight line grazes → CLEAR (not blocked)"
        );
        // Within the epsilon above → still CLEAR (float-noise guard).
        let within = [
            s(0.0, 100.0),
            s(50.0, 101.8 + OCCLUSION_EPS_M * 0.5),
            s(100.0, 100.0),
        ];
        assert_eq!(
            occlusion(&within, EYE_HEIGHT_OBSERVER_M, EYE_HEIGHT_TARGET_M),
            LosVerdict::Clear,
            "terrain within epsilon of the line → CLEAR"
        );
        // A hair MORE than epsilon above → BLOCKED.
        let over = [
            s(0.0, 100.0),
            s(50.0, 101.8 + OCCLUSION_EPS_M * 2.0),
            s(100.0, 100.0),
        ];
        assert!(
            occlusion(&over, EYE_HEIGHT_OBSERVER_M, EYE_HEIGHT_TARGET_M).is_blocked(),
            "terrain > line + epsilon → BLOCKED"
        );
    }

    /// Observer on a hill sees a valley: a high observer looks DOWN over intervening lower ground to
    /// a low target — the down-sloping sight line clears everything. CLEAR.
    #[test]
    fn occlusion_observer_on_hill_sees_valley() {
        // Observer at 300 m ground (eye 301.8), target at 100 m ground (eye 101.8); the ground dips
        // to 90 m in between — well under the descending line. CLEAR.
        let prof = [
            s(0.0, 300.0),
            s(250.0, 150.0),
            s(500.0, 90.0),
            s(750.0, 95.0),
            s(1000.0, 100.0),
        ];
        assert_eq!(
            occlusion(&prof, EYE_HEIGHT_OBSERVER_M, EYE_HEIGHT_TARGET_M),
            LosVerdict::Clear,
            "a high observer's descending line clears the lower valley"
        );
    }

    /// A short profile (0/1 sample — entirely off DEM coverage) is Unknown, never a fake Clear.
    #[test]
    fn occlusion_off_coverage_is_unknown() {
        assert_eq!(occlusion(&[], 1.8, 1.8), LosVerdict::Unknown);
        assert_eq!(occlusion(&[s(0.0, 50.0)], 1.8, 1.8), LosVerdict::Unknown);
    }

    /// A two-sample flat profile can never self-block on its own endpoints: the line sits an eye
    /// height above each ground endpoint, so a bare observer→target with nothing between is CLEAR.
    #[test]
    fn occlusion_endpoints_never_self_block() {
        let prof = [s(0.0, 42.0), s(500.0, 42.0)];
        assert_eq!(
            occlusion(&prof, EYE_HEIGHT_OBSERVER_M, EYE_HEIGHT_TARGET_M),
            LosVerdict::Clear
        );
        // Even a steep endpoint difference stays clear (the eyes are above both grounds).
        let steep = [s(0.0, 0.0), s(500.0, 300.0)];
        assert_eq!(
            occlusion(&steep, EYE_HEIGHT_OBSERVER_M, EYE_HEIGHT_TARGET_M),
            LosVerdict::Clear
        );
    }

    // ── formatter / verdict goldens ───────────────────────────────────────────────────────────────

    #[test]
    fn distance_formatter() {
        assert_eq!(format_distance(412.0), "412 m");
        assert_eq!(format_distance(999.4), "999 m");
        assert_eq!(format_distance(1000.0), "1.00 km");
        assert_eq!(format_distance(1240.0), "1.24 km");
    }

    #[test]
    fn verdict_formatter() {
        assert_eq!(
            format_verdict(LosVerdict::Clear, 1240.0),
            "LoS clear · 1.24 km"
        );
        assert_eq!(
            format_verdict(
                LosVerdict::Blocked {
                    blocking_dist_m: 412.0,
                    blocking_elev_m: 150.0
                },
                800.0
            ),
            "LoS blocked at 412 m"
        );
        assert_eq!(format_verdict(LosVerdict::Unknown, 0.0), "LoS —");
        // is_clear / is_blocked helpers.
        assert!(LosVerdict::Clear.is_clear() && !LosVerdict::Clear.is_blocked());
        assert!(LosVerdict::Blocked {
            blocking_dist_m: 1.0,
            blocking_elev_m: 1.0
        }
        .is_blocked());
        assert!(!LosVerdict::Unknown.is_clear() && !LosVerdict::Unknown.is_blocked());
    }

    // ── the two-click capture state machine (Decisions 2/3/4) ─────────────────────────────────────

    #[test]
    fn click_captures_observer_then_target() {
        let mut st = LosState::new();
        assert!(st.is_empty());
        // First click → observer pending, no shot yet, not "completed".
        assert!(!st.click(10.0, 20.0, Some(5.0)));
        assert_eq!(st.pending_obs, Some((10.0, 20.0, Some(5.0))));
        assert!(st.shot.is_none());
        assert!(!st.is_empty());
        // Second click → shot completed, pending cleared, returns true.
        assert!(st.click(110.0, 20.0, Some(8.0)));
        assert!(st.pending_obs.is_none());
        let shot = st.shot.expect("shot placed");
        assert_eq!(
            (shot.obs_x, shot.obs_y, shot.obs_z),
            (10.0, 20.0, Some(5.0))
        );
        assert_eq!(
            (shot.tgt_x, shot.tgt_y, shot.tgt_z),
            (110.0, 20.0, Some(8.0))
        );
        assert!((shot.distance_m() - 100.0).abs() < 1e-9);
    }

    #[test]
    fn third_click_starts_a_fresh_capture() {
        let mut st = LosState::new();
        st.click(0.0, 0.0, None);
        st.click(100.0, 0.0, None);
        assert!(st.shot.is_some());
        // A third click retires the old shot and starts a new observer.
        assert!(!st.click(500.0, 500.0, None));
        assert_eq!(st.pending_obs, Some((500.0, 500.0, None)));
        assert!(
            st.shot.is_none(),
            "the previous shot is retired on a new capture"
        );
    }

    /// Esc mirrors the ruler's two-step (Decision 3): first drops the in-progress capture, a second
    /// clears the placed result; an Esc with nothing acts false (falls through — never swallowed).
    #[test]
    fn escape_two_step_dismissal() {
        let mut st = LosState::new();
        assert!(!st.escape(), "nothing to dismiss → no act");
        // In-progress (observer placed, no target): first Esc drops it.
        st.click(0.0, 0.0, None);
        assert!(st.pending_obs.is_some() && st.shot.is_none());
        assert!(st.escape(), "first Esc acts");
        assert!(st.is_empty(), "first Esc drops the in-progress observer");
        // Placed shot: Esc clears it.
        st.click(0.0, 0.0, None);
        st.click(100.0, 0.0, None);
        assert!(st.shot.is_some());
        assert!(st.escape(), "Esc on a placed shot acts");
        assert!(st.is_empty(), "Esc clears the placed result");
        assert!(!st.escape(), "now empty → no act");
    }

    #[test]
    fn clear_is_idempotent_and_total() {
        let mut st = LosState::new();
        st.click(0.0, 0.0, None);
        st.click(1.0, 0.0, None);
        st.clear();
        assert!(st.is_empty());
        st.clear(); // idempotent
        assert!(st.is_empty());
    }

    // ── projection + panel keying (T-727) ─────────────────────────────────────────────────────────

    #[test]
    fn project_shot_maps_endpoints_and_derives_verdict() {
        let shot = LosShot {
            obs_x: 0.0,
            obs_y: 0.0,
            obs_z: Some(100.0),
            tgt_x: 100.0,
            tgt_y: 0.0,
            tgt_z: Some(100.0),
        };
        // Flat profile → clear; identity-ish projector (scale 2, offset 5/7).
        let profile = [s(0.0, 100.0), s(50.0, 100.0), s(100.0, 100.0)];
        let proj = project_shot(&shot, &profile, 1.8, 1.8, |x, y| {
            (x * 2.0 + 5.0, y * 2.0 + 7.0)
        });
        assert!((proj.obs_px - 5.0).abs() < 1e-9 && (proj.obs_py - 7.0).abs() < 1e-9);
        assert!((proj.tgt_px - 205.0).abs() < 1e-9 && (proj.tgt_py - 7.0).abs() < 1e-9);
        assert_eq!(proj.verdict, LosVerdict::Clear);
        assert!(proj.block_px.is_none(), "clear → no blocking marker");
        assert!((proj.total_m - 100.0).abs() < 1e-9);
    }

    #[test]
    fn project_shot_blocking_marker_on_the_line() {
        let shot = LosShot {
            obs_x: 0.0,
            obs_y: 0.0,
            obs_z: Some(100.0),
            tgt_x: 100.0,
            tgt_y: 0.0,
            tgt_z: Some(100.0),
        };
        // Ridge blocking at dist 50 (half-way) → marker at the line midpoint.
        let profile = [s(0.0, 100.0), s(50.0, 200.0), s(100.0, 100.0)];
        let proj = project_shot(&shot, &profile, 1.8, 1.8, |x, y| (x, y));
        assert!(proj.verdict.is_blocked());
        let (bx, by) = proj.block_px.expect("blocked → marker");
        assert!(
            (bx - 50.0).abs() < 1e-9 && (by - 0.0).abs() < 1e-9,
            "marker at the half-way pixel"
        );
    }

    /// Two DIFFERENT shots that share an identical VERDICT/label must get DIFFERENT keys — the
    /// T-727 world-coordinate keying (a `<For>` keyed on text would retain a stale panel).
    #[test]
    fn shot_keys_are_world_coords_not_verdict() {
        let a = LosShot {
            obs_x: 0.0,
            obs_y: 0.0,
            obs_z: None,
            tgt_x: 100.0,
            tgt_y: 0.0,
            tgt_z: None,
        };
        let b = LosShot {
            obs_x: 500.0,
            obs_y: 500.0,
            obs_z: None,
            tgt_x: 600.0,
            tgt_y: 500.0,
            tgt_z: None,
        };
        let flat = [s(0.0, 10.0), s(100.0, 10.0)];
        let pa = project_shot(&a, &flat, 1.8, 1.8, |x, y| (x, y));
        let pb = project_shot(&b, &flat, 1.8, 1.8, |x, y| (x, y));
        // Same verdict (both clear over a flat profile) but distinct world-coord keys.
        assert_eq!(pa.verdict, pb.verdict);
        assert_ne!(
            pa.key, pb.key,
            "T-727: distinct shots get distinct world-coord keys"
        );
    }

    #[test]
    fn world_key_quantises_and_distinguishes() {
        assert_eq!(
            world_key(10.02, 20.0, 30.0, 40.0),
            world_key(10.03, 20.0, 30.0, 40.0)
        );
        assert_ne!(
            world_key(10.0, 20.0, 30.0, 40.0),
            world_key(11.0, 20.0, 30.0, 40.0)
        );
    }

    // ── profile chart geometry (the inline panel) ─────────────────────────────────────────────────

    #[test]
    fn profile_chart_maps_into_the_box() {
        // A ramp profile 0..100 elev over 0..1000 dist, box 200×64.
        let prof = [s(0.0, 0.0), s(500.0, 50.0), s(1000.0, 100.0)];
        let chart = profile_chart(&prof, 1.8, 1.8, 200.0, 64.0);
        assert!(!chart.ground.is_empty(), "ground curve has points");
        assert!(!chart.line.is_empty(), "sight line has points");
        // The ground string has three "x,y" pairs; first x is 0, last x is the box width (200).
        let pts: Vec<&str> = chart.ground.split_whitespace().collect();
        assert_eq!(pts.len(), 3);
        assert!(pts[0].starts_with("0.0,"), "first ground x at box left");
        assert!(
            pts[2].starts_with("200.0,"),
            "last ground x at box right (200)"
        );
        // Ground rises left→right, so screen y DECREASES (inverted axis): y0 > y2.
        let y = |p: &str| p.split(',').nth(1).unwrap().parse::<f64>().unwrap();
        assert!(y(pts[0]) > y(pts[2]), "higher ground → smaller y (up)");
    }

    #[test]
    fn profile_chart_empty_when_too_short() {
        assert_eq!(
            profile_chart(&[], 1.8, 1.8, 200.0, 64.0),
            ProfileChart::default()
        );
        assert_eq!(
            profile_chart(&[s(0.0, 1.0)], 1.8, 1.8, 200.0, 64.0),
            ProfileChart::default()
        );
    }

    #[test]
    fn profile_chart_flat_profile_does_not_divide_by_zero() {
        // A perfectly flat profile (min == max ground, and eyes equal) still charts a mid line.
        let flat = [s(0.0, 50.0), s(100.0, 50.0)];
        let chart = profile_chart(&flat, 1.8, 1.8, 200.0, 64.0);
        assert!(!chart.ground.is_empty());
        // Every y is finite (no NaN from a zero span).
        for p in chart.ground.split_whitespace() {
            let y: f64 = p.split(',').nth(1).unwrap().parse().unwrap();
            assert!(y.is_finite(), "flat profile y must be finite, got {y}");
        }
    }

    #[test]
    fn profile_chart_marks_blocking_point() {
        let prof = [s(0.0, 100.0), s(50.0, 200.0), s(100.0, 100.0)];
        let chart = profile_chart(&prof, 1.8, 1.8, 200.0, 64.0);
        let (bx, _by) = chart.block.expect("blocked profile marks the block");
        assert!(
            (bx - 100.0).abs() < 1e-6,
            "block x at the half-way column (100 of 200)"
        );
    }

    // ── Decision 4 pin: LoS is session-local overlay state, NEVER doc/store writes ───────────────

    /// The LoS result is a MEASUREMENT, not mission content — Decision 4. This module must reference
    /// NO document mutation. Proven on scrubbed code (comments + strings blanked) so a mention in
    /// prose/tests can't satisfy it; combined with the compiler (this file imports no doc mutator)
    /// this is the "no store.rs writes" guarantee.
    #[test]
    fn no_los_doc_writes() {
        let code = crate::arsenal::class_r_scrub::live_code(include_str!("los_tool.rs"));
        for banned in [
            "MissionDocCore",
            "move_entities",
            "add_slot",
            "store.rs",
            "hydrate",
            "after_local_edit",
            "editor_ops",
        ] {
            assert!(
                !code.contains(banned),
                "Decision 4: los_tool must be session-local overlay state — found doc-write token \
                 `{banned}`; a LoS check must NEVER write the document"
            );
        }
    }

    // ── FIRE THE OCCLUSION RULE (perturb / fail / restore) ────────────────────────────────────────

    /// The occlusion rule genuinely discriminates: a ridge that pokes above the sight line reads
    /// BLOCKED, and asserting it is CLEAR fails; lowering that ridge below the line restores CLEAR.
    /// A rule that ignored the terrain (always Clear) would pass the perturbed assertion — so this
    /// proves the strictly-above comparison is load-bearing, not incidental.
    #[test]
    fn occlusion_rule_fires() {
        // Baseline: a 200 m ridge across a flat 100 m sight is BLOCKED.
        let blocked = [s(0.0, 100.0), s(50.0, 200.0), s(100.0, 100.0)];
        let v = occlusion(&blocked, EYE_HEIGHT_OBSERVER_M, EYE_HEIGHT_TARGET_M);
        assert!(
            v.is_blocked(),
            "baseline: a 200 m ridge blocks a flat sight"
        );
        // Perturb: CLAIM it is clear. That is FALSE (the ridge towers over the line), so an equality
        // against Clear must NOT hold — the rule fires.
        assert_ne!(
            v,
            LosVerdict::Clear,
            "the ridge MUST block — if this were Clear the occlusion test would be ignoring terrain"
        );
        // Restore: drop the ridge below the sight line (to 100 m, flat) → CLEAR again.
        let cleared = [s(0.0, 100.0), s(50.0, 100.0), s(100.0, 100.0)];
        assert_eq!(
            occlusion(&cleared, EYE_HEIGHT_OBSERVER_M, EYE_HEIGHT_TARGET_M),
            LosVerdict::Clear,
            "lowering the ridge below the line restores a clear sight"
        );
        // And the verdict genuinely VARIES with the terrain (not a constant): blocked vs clear differ.
        assert_ne!(
            occlusion(&blocked, EYE_HEIGHT_OBSERVER_M, EYE_HEIGHT_TARGET_M),
            occlusion(&cleared, EYE_HEIGHT_OBSERVER_M, EYE_HEIGHT_TARGET_M),
            "the occlusion verdict must depend on the terrain profile"
        );
    }
}
