//! Role: the two-click ray capture, the sub-mode toggle, and the viewshed placement state.
//! Position: `line_of_sight` in `map_editing_tools`.
//! Signals & state: session-local measurement state; never the authored document.
//! Invariants: a shot is two points and nothing more; escape steps down from an in-progress capture to a placed result; a placement replaces, never appends.

use terrain_line_of_sight::viewshed::Viewshed;

// ── Tool-mode arbitration note ──────────────────────────────────────────────────────────────────
//
// Line of sight shares `ruler::EditorTool` (the `LoS` variant), the `should_begin_ruler`
// point-capture predicate and the `LeftGesture::Ruler` arm, so it needs no gesture variant of its
// own. The Mission Creator's commit site branches on `tool_mode.is_los()` to route a captured click
// into [`LosState::click`] instead of the ruler chain. See `ruler::EditorTool::captures_points`.

// ── The two-click capture state machine ──────────────────────────────────────────────────────────

/// A placed line-of-sight measurement: observer + target world points, each with its clicked-time
/// DEM ground elevation. Session-local overlay state (never the mission document). The verdict +
/// profile are DERIVED (recomputed by the overlay from the live DEM) rather than stored, so a panned
/// camera never shows a stale sample — the two points are the only state.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LosShot {
    /// Observer world x, metres.
    pub obs_x: f64,
    /// Observer world y, metres.
    pub obs_y: f64,
    /// Observer ground elevation at click time (metres ASL), `None` off DEM coverage.
    pub obs_z: Option<f64>,
    /// Target world x, metres.
    pub tgt_x: f64,
    /// Target world y, metres.
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

/// The LoS tool's capture phase + placed shot. Session-local overlay state (never the mission
/// document). Two clicks
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

    /// Commit a click at world `(x, y, z)` — the two-click capture:
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

    /// Escape — the two-step escalating dismissal, like the ruler's:
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

    /// Clear everything (tool-switch away from LoS is the dismissal's "second-Esc equivalent").
    /// Idempotent.
    pub fn clear(&mut self) {
        self.pending_obs = None;
        self.shot = None;
    }
}

// ── The line-of-sight tool's second mode: viewshed ────────────────────────────────────────────
//
// The one line-of-sight button (`visibility`) carries a sub-mode that toggles on a repeated click
// of the button — Ray (point to point) → Viewshed (disc) → Ray → … The button label and icon show
// the live sub-mode, and the map click semantics change with it:
//   * Ray sub-mode      — TWO clicks (observer, target) → a clear/blocked verdict + profile panel.
//   * Viewshed sub-mode — ONE click (the observer) → the whole disc is shaded (visible/hidden wash).
// A sub-mode on one button rather than a fourth toolbar button: the two are the same question
// ("what can be seen from here") at two scales — one ray vs the whole horizon — so they belong on
// one control. Switching sub-mode CLEARS
// the other sub-mode's overlay (a placed ray is dropped when you switch to viewshed and vice-versa),
// exactly as switching TOOLS clears the inactive tool.

/// The line-of-sight tool's sub-mode. `Ray` is the point-to-point sight line; `Viewshed` is the
/// one-observer disc raster. Toggled by re-clicking the LoS toolbar button. A shared,
/// native-testable enum (the toolbar reads it; the pointer commit branches on it), kept here beside
/// the states it selects between.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LosMode {
    /// Click observer, click target → clear/blocked + profile.
    #[default]
    Ray,
    /// Click one observer → shade the whole visible/hidden disc.
    Viewshed,
}

impl LosMode {
    /// The next sub-mode in the toggle cycle (Ray ⇆ Viewshed). Re-clicking the LoS button advances
    /// this while the LoS tool is already active; the FIRST click (from another tool) just activates
    /// LoS without advancing, so a fresh switch to LoS always lands on the mode it last showed.
    #[must_use]
    pub fn toggled(self) -> Self {
        match self {
            LosMode::Ray => LosMode::Viewshed,
            LosMode::Viewshed => LosMode::Ray,
        }
    }

    /// True for the viewshed sub-mode (the one-click disc).
    #[must_use]
    pub fn is_viewshed(self) -> bool {
        matches!(self, LosMode::Viewshed)
    }
}

/// The viewshed sub-mode's session-local overlay state (not the mission document, exactly like the
/// ruler chain + the LoS ray). A single placed observer and the raster computed from it. The raster
/// is stored (unlike the ray's derived-every-frame verdict) because the compute runs ONCE per
/// placement (the ~36 ms radial march is not a per-frame cost — see `compute_viewshed`); a pan
/// re-projects the SAME raster's world rect, it does not recompute. Re-placing (a new observer click)
/// replaces both the observer and the raster.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ViewshedState {
    /// The placed observer world point + its click-time ground Z, or `None` if none placed.
    pub observer: Option<(f64, f64, Option<f64>)>,
    /// The computed raster for the placed observer, or `None` until the host computes it. Held so the
    /// overlay/engine can re-project on pan without recomputing.
    pub raster: Option<Viewshed>,
}

impl ViewshedState {
    /// No observer placed and no raster.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// True when there is nothing placed or drawn.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.observer.is_none() && self.raster.is_none()
    }

    /// Place a new observer at world `(x, y, z)` — REPLACES any previous observer + raster (a viewshed
    /// is a single disc, not a chain). The raster is left `None` for the host to fill via
    /// [`compute_viewshed`](terrain_line_of_sight::viewshed::compute_viewshed); returns nothing because,
    /// unlike the ray, there is no "completed on the second click" event — one click IS the placement.
    pub fn place(&mut self, x: f64, y: f64, z: Option<f64>) {
        self.observer = Some((x, y, z));
        self.raster = None;
    }

    /// Store the host-computed raster for the current observer.
    pub fn set_raster(&mut self, vs: Viewshed) {
        self.raster = Some(vs);
    }

    /// Escape / dismissal — one step (there is no in-progress half-placement to abandon first, unlike
    /// the ray's two-click capture): clear the placed observer + raster. Returns whether it acted so
    /// the host `preventDefault`s only on a real dismissal.
    pub fn escape(&mut self) -> bool {
        if self.is_empty() {
            false
        } else {
            self.clear();
            true
        }
    }

    /// Clear everything (tool/sub-mode switch away). Idempotent.
    pub fn clear(&mut self) {
        self.observer = None;
        self.raster = None;
    }
}

#[cfg(test)]
#[path = "tests/capture.rs"]
mod tests;
