//! T-642 — Ruler: a persistent measuring polyline with per-leg distance, bearing and slope plus a
//! running total. Operator-requested; one of the two dead buttons the wave-105 split left in
//! `eden_toolbelt`'s [`ModeToolbar`] (`straighten`). NO PRIOR ART — 3den's ruler is a two-point
//! measure with a 20 s notification; this design is ours: click a **chain** of points; each leg
//! carries distance AND bearing AND Δelevation/slope, a running total accumulates, and the whole
//! thing PERSISTS on the map until dismissed.
//!
//! THE RULE THIS TICKET EXISTS TO HONOUR: removing `disabled` from the button without the tool
//! working is worse than the current honest stub — it turns a truthful "soon" into a lie (the corpus
//! has two dead-control cautionary tales). So the tool works end-to-end BEFORE the button enables:
//! the state machine + math below are the load-bearing core, natively `cargo test`-proven, and the
//! button/overlay are thin wrappers over them.
//!
//! ── Module shape (mirrors `eden_toolbelt` / `select_tool`) ──────────────────────────────────────
//! Everything measurable is a **pure function or a pure state machine** ([`RulerChain`], [`Leg`],
//! the formatters) so a native `cargo test -p website-frontend` proves the geometry with no browser
//! — this file is UNGATED (declared `mod ruler_tool;` with no `#[cfg(target_arch = "wasm32")]` in
//! `main.rs`), exactly so those tests run on the same command CI uses. The Leptos [`RulerOverlay`]
//! component compiles on native but renders nothing there (no engine, no `window`) — the
//! `MapGridRefs` idiom — because its geometry is already proven by the pure layer it draws from.
//!
//! ── THE FOUR DECISIONS THIS TICKET LEFT OPEN (each made + justified where it lives) ────────────
//!
//! **Decision 1 — leg labels ON THE LINE + running total in the status bar.** Each leg's readout is
//! drawn mid-leg, horizontal, over the segment (the spot-height idiom — a label welded to the thing
//! it names), so a leg can never be read against the wrong segment. The RUNNING TOTAL and the
//! LAST-LEG readout additionally go in the status bar's readout section (see
//! [`RulerChain::status_readout`], mounted by `eden_toolbelt::StatusBar`) — the place the operator
//! already reads CUR/OBJ/SEL, so the summary lands where the eye is without hunting the map.
//!
//! **Decision 2 — per-leg slope/Δelevation SHOWN.** The DEM makes it free: the same
//! `dem::downsample::sample_grid_meters` the CUR-Z read-out samples (`mission_editor` cursor path)
//! gives each vertex an elevation, so a leg carries `+8 m (2%)` at no extra fetch. When a vertex is
//! off DEM coverage the leg simply omits the slope clause (distance + bearing still show) rather
//! than printing a fake `0 m` — an honest gap, matching the CUR-Z em-dash policy.
//!
//! **Decision 3 — dismissal.** Esc clears the IN-PROGRESS chain (the un-committed tail);a second Esc
//! — or switching the tool back to Select — clears the PLACED ruler. Double-click ENDS the chain and
//! KEEPS it placed. This is the [`RulerChain::press`] / [`double_click`](RulerChain::double_click) /
//! [`escape`](RulerChain::escape) transition set, and it is what makes a ruler both easy to finish
//! (dbl-click) and easy to abandon in two escalating steps (Esc, Esc).
//!
//! **Decision 4 — rulers do NOT survive save/reload.** A ruler is a MEASUREMENT, not mission
//! content: it is session-local OVERLAY state held here (app-side, like the selection set), NOT the
//! Y.Doc (`store.rs` gets NO ruler writes — see the `no_ruler_doc_writes` pin). Reload → the map is
//! clean. This keeps the compiled payload byte-identical to pre-ruler and keeps the tool from
//! silently bloating every saved mission.

#![allow(dead_code)] // the wasm host wires the live path; native `cargo test` proves the pure core.

use leptos::prelude::*;

// ── Pure geometry + formatting (native-tested) ──────────────────────────────────────────────────

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
    fn between(from: RulerPoint, to: RulerPoint) -> Self {
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

/// The active editor tool. `Select` is the default (the whole T-036 pick/marquee/move machine);
/// `Ruler` re-purposes the LMB for measuring (this ticket). LoS (`straighten`'s neighbour) is
/// T-643's — deliberately absent here, its button stays disabled until wave 109.
///
/// This is a shared, native-testable enum (the `eden_toolbelt` button reads it via a signal; the
/// `mission_editor` pointer handlers branch on it). Keeping it here — not in the wasm-only
/// `select_tool` — is what lets `cargo test -p website-frontend` prove the arbitration below.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EditorTool {
    #[default]
    Select,
    Ruler,
}

impl EditorTool {
    #[must_use]
    pub fn is_ruler(self) -> bool {
        matches!(self, EditorTool::Ruler)
    }
}

/// Should an LMB `pointerdown` open a RULER gesture rather than the Select machine's `Pending`?
///
/// The whole tool-mode arbitration in one predicate — and the answer to the ticket's binding
/// constraints (the wave-106 T-723 findings):
///   * **(c) button 0 only** — a ruler vertex is a LEFT click; middle/right stay pan / context-menu.
///     `button != 0` ⇒ never a ruler press (so the host's MMB-pan and RMB-menu are untouched).
///   * the tool must be `Ruler` — under `Select` this is always `false` and the existing
///     Pending→Move|Marquee path is entirely unchanged.
///
/// The host uses this at `pointerdown` to choose `LeftGesture::Ruler` vs `LeftGesture::Pending`, and
/// the ruler branch it opens is a SEPARATE `LG` arm that never falls into the armed-placement
/// pointerup branch — constraint **(a)** (that branch is gated on a palette place, which a ruler
/// click never arms) — and whose pointerdown-written gesture is always taken/cleared by the ruler
/// arms of pointermove/up/cancel — constraint **(b)**.
#[must_use]
pub fn should_begin_ruler(tool: EditorTool, button: i16) -> bool {
    tool.is_ruler() && button == 0
}

// ── The chain state machine (Decision 3 lives here) ─────────────────────────────────────────────

/// The persistent ruler polyline + its placement phase. Session-local overlay state (Decision 4 —
/// NOT the Y.Doc); the wasm host holds one leaked `RefCell<RulerChain>` beside the selection set.
///
/// A chain is a list of committed vertices plus a `drawing` flag. While `drawing`, the NEXT click
/// appends a vertex and a live "rubber-band" leg to the cursor previews the leg-to-be (the overlay
/// draws it from the live cursor; the chain only stores committed vertices, so the preview needs no
/// state here). `double_click` ends the chain (clears `drawing`, keeps the points). `escape`
/// escalates: first it drops the in-progress tail (ends drawing), then a second Esc clears the
/// placed points entirely.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RulerChain {
    /// Committed vertices in click order.
    pub points: Vec<RulerPoint>,
    /// True while the operator is still adding points (the tool is "armed" for the next click).
    pub drawing: bool,
}

impl RulerChain {
    /// A fresh, empty chain (nothing placed, not drawing).
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// True when there is nothing to draw or dismiss (no points and not mid-draw).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    /// Commit a click at world `(x, y, z)` — the primary interaction (Decision 3). Appends the
    /// vertex and marks the chain `drawing` (so the next click continues the chain and the overlay
    /// shows the rubber-band leg to the cursor). Starting a fresh chain and extending an existing
    /// one are the same op — the only difference is whether `points` was empty.
    pub fn press(&mut self, x: f64, y: f64, z: Option<f64>) {
        self.points.push(RulerPoint::new(x, y, z));
        self.drawing = true;
    }

    /// End the chain but KEEP it placed (Decision 3 — the double-click contract). Clears `drawing`
    /// so no further click extends it; the points stay on the map until dismissed. A no-op on an
    /// empty chain. A double-click also fires two `press`es first (the browser emits pointer events
    /// before `dblclick`); the host de-dupes the coincident final vertex — see `end_dedup_epsilon`.
    pub fn double_click(&mut self) {
        self.drawing = false;
    }

    /// Escape — the two-step escalating dismissal (Decision 3):
    ///   * while `drawing` → drop the in-progress tail: stop drawing but KEEP the committed points
    ///     (so a mis-aimed final click is undone without losing the whole measure), UNLESS only a
    ///     single lone vertex exists (no leg yet) in which case there is nothing worth keeping and
    ///     the chain clears outright;
    ///   * when NOT drawing (a placed ruler) → clear the placed points entirely.
    ///
    /// Returns `true` if it changed anything (so the host can `preventDefault` only on a real act).
    pub fn escape(&mut self) -> bool {
        if self.is_empty() {
            return false;
        }
        if self.drawing {
            // A single un-legged vertex is not worth "keeping" — first Esc clears it.
            if self.points.len() <= 1 {
                self.points.clear();
            }
            self.drawing = false;
        } else {
            self.points.clear();
        }
        true
    }

    /// Clear everything (tool-switch back to Select is Decision 3's "second Esc equivalent"). Idempotent.
    pub fn clear(&mut self) {
        self.points.clear();
        self.drawing = false;
    }

    /// Drop the LAST committed vertex if it is within `eps` world metres of the point BEFORE it —
    /// the double-click de-dupe. A dblclick emits `pointerdown/up` (→ `press`) then `dblclick`
    /// (→ `double_click`); the two coincident presses would otherwise leave a zero-length final leg.
    /// The host calls this from the `dblclick` handler, before `double_click`, so the kept chain ends
    /// on the real penultimate vertex. Returns `true` if a duplicate tail was removed.
    pub fn dedup_tail(&mut self, eps: f64) -> bool {
        let n = self.points.len();
        if n < 2 {
            return false;
        }
        if distance_m(self.points[n - 2], self.points[n - 1]) <= eps {
            self.points.pop();
            true
        } else {
            false
        }
    }

    /// The committed legs (`points.len() − 1` of them; empty for 0/1 vertices). Each [`Leg`] carries
    /// distance/bearing/Δelev/slope, computed once so the overlay and the status bar agree.
    #[must_use]
    pub fn legs(&self) -> Vec<Leg> {
        self.points
            .windows(2)
            .map(|w| Leg::between(w[0], w[1]))
            .collect()
    }

    /// Total ground distance over all committed legs (metres). 0 for a 0/1-vertex chain.
    #[must_use]
    pub fn total_m(&self) -> f64 {
        self.legs().iter().map(|l| l.dist_m).sum()
    }

    /// The status-bar readout string (Decision 1 — the summary that rides beside CUR/OBJ/SEL), or
    /// `None` when there is nothing to summarise (no legs yet). Shows the running total and the
    /// LAST leg's readout, e.g. `"Σ 1.24 km · last 412 m · 073.2° · +8 m (2%)"`. `None` while a lone
    /// first vertex sits un-legged (the total is meaningless with no leg).
    #[must_use]
    pub fn status_readout(&self) -> Option<String> {
        let legs = self.legs();
        let last = legs.last()?;
        Some(format!(
            "{} · last {}",
            format_total(self.total_m()),
            last.label()
        ))
    }
}

// ── The DOM/SVG overlay (cheapest correct lane at ruler scale — a handful of points) ─────────────
//
// RENDERING LANE (the ticket's open call): the chain is a few points, so a GPU lane (a new wgpu
// pipeline + upload) would be far more machinery than the geometry warrants. The selection MARQUEE
// proves DOM overlay is the house idiom for transient camera-projected geometry at this scale
// (`MapGridRefs` also draws camera-projected labels from a pure geometry layer with no GPU), so the
// ruler draws as ONE absolutely-positioned SVG overlay: a polyline through the projected vertices +
// per-leg mid-line `<text>` labels. Reactive off the same cursor/heartbeat channel the scale bar +
// grid refs use — NO new rAF loop.
//
// LABEL KEYING (wave-107 T-727 trap): the `<For>` over legs is keyed by the leg's WORLD-COORDINATE
// endpoints, NOT by the label text. Wave-107 found a `<For>` keyed on text retained a stale DOM
// position when two legs shared a label string (`"412 m · 073.2°"` twice), because Leptos reused the
// node for the "same" key at the wrong place. Keying on the world coords means a leg's node is tied
// to WHERE it is, so moving/re-placing a vertex re-positions the right node and identical labels on
// different legs never collide.

/// A projected leg ready to draw: screen-pixel endpoints + mid-point (for the label) + the label
/// text and a WORLD-coordinate key (Decision 1 / the T-727 keying fix). Built by [`project_legs`].
#[derive(Clone, Debug, PartialEq)]
pub struct ProjectedLeg {
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
    pub mid_x: f64,
    pub mid_y: f64,
    pub label: String,
    /// Stable key: the leg's two world endpoints quantised to 0.1 m. Ties the DOM node to WHERE the
    /// leg is, so identical label strings on different legs never share a `<For>` key (T-727).
    pub key: String,
}

/// A key string from a world coordinate pair, quantised to 0.1 m so tiny float noise between frames
/// does not churn the key (which would drop + re-create the node every frame). 0.1 m is far below a
/// visible pixel at any editor zoom, so two genuinely distinct vertices never collide.
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

/// Project a chain's committed legs to screen space via a world→pixel projector (the live
/// `OrthoCamera::project` on wasm; injected here so this is pure + native-testable). Each leg gets
/// its endpoints, mid-point, label and a world-coordinate key. `project` takes world `(x, y)` and
/// returns screen `(px, py)`.
#[must_use]
pub fn project_legs<F>(chain: &RulerChain, project: F) -> Vec<ProjectedLeg>
where
    F: Fn(f64, f64) -> (f64, f64),
{
    chain
        .legs()
        .iter()
        .map(|leg| {
            let (x1, y1) = project(leg.from.x, leg.from.y);
            let (x2, y2) = project(leg.to.x, leg.to.y);
            let (mx, my) = leg.midpoint();
            let (mid_x, mid_y) = project(mx, my);
            ProjectedLeg {
                x1,
                y1,
                x2,
                y2,
                mid_x,
                mid_y,
                label: leg.label(),
                key: world_key(leg.from.x, leg.from.y, leg.to.x, leg.to.y),
            }
        })
        .collect()
}

/// A projected vertex dot — a small ring at each committed point so the operator sees the exact
/// clicked positions. Keyed by world coordinate (T-727) like the legs.
#[derive(Clone, Debug, PartialEq)]
pub struct ProjectedVertex {
    pub px: f64,
    pub py: f64,
    pub key: String,
}

/// Project the committed vertices to screen dots (same projector as [`project_legs`]).
#[must_use]
pub fn project_vertices<F>(chain: &RulerChain, project: F) -> Vec<ProjectedVertex>
where
    F: Fn(f64, f64) -> (f64, f64),
{
    chain
        .points
        .iter()
        .map(|p| {
            let (px, py) = project(p.x, p.y);
            ProjectedVertex {
                px,
                py,
                key: world_key(p.x, p.y, p.x, p.y),
            }
        })
        .collect()
}

// ── The leaked-chain registry (the `context_menu::set_menu_signal` / `editor_ops` idiom) ─────────
//
// The overlay component is mounted in the shared `view!` (outside `mission_editor`'s wasm block),
// but the live `RulerChain` is a leaked `Rc<RefCell<…>>` inside that block. Rather than thread a
// closure through the component tree, the wasm host REGISTERS the chain handle into a thread_local
// here (the same handoff pattern `context_menu::set_menu_signal` and `editor_ops::set_*_signal`
// use), and the overlay reads a clone of it. thread_local + `Rc<RefCell<…>>` is `!Send`-safe (JS is
// single-threaded) and works on native too, so this stays in the ungated module.

thread_local! {
    static RULER_CHAIN: std::cell::RefCell<Option<std::rc::Rc<std::cell::RefCell<RulerChain>>>> =
        const { std::cell::RefCell::new(None) };
}

/// Register the host's leaked ruler chain so [`RulerOverlay`] can read it. Called once at mount by
/// `mission_editor` (peer of `context_menu::set_menu_signal`).
pub fn register_ruler_chain(chain: std::rc::Rc<std::cell::RefCell<RulerChain>>) {
    RULER_CHAIN.with(|c| *c.borrow_mut() = Some(chain));
}

/// A snapshot clone of the registered chain (empty if none registered — e.g. native/pre-mount).
#[must_use]
pub fn read_registered_chain() -> RulerChain {
    RULER_CHAIN.with(|c| {
        c.borrow()
            .as_ref()
            .map(|rc| rc.borrow().clone())
            .unwrap_or_default()
    })
}

/// T-642 — the ruler overlay. ONE absolutely-positioned, `pointer-events-none` SVG spanning the
/// viewport, drawing the placed polyline + per-leg on-line labels (Decision 1) + vertex dots + the
/// live rubber-band leg to the cursor while drawing. It reads the live camera off the registered
/// engine (`world_assets::camera_snapshot`, the same seam the scale bar / grid refs use) and re-runs
/// off the `cursor` (pan) + `debug_hud` (~1 Hz zoom) heartbeats — NO new rAF loop. `tick` is bumped
/// by the host on every chain mutation (press / Esc / dbl-click) so a click that does not move the
/// pointer still repaints.
///
/// The chain itself is read via [`read_registered_chain`] — a cheap clone of the leaked host
/// `RulerChain` (session-local overlay state — Decision 4). Native builds render an empty overlay
/// (no engine, no `window`); the geometry is proven by `project_legs`/`world_key` above.
#[component]
pub fn RulerOverlay(
    /// Pan heartbeat — the editor's pointer-move cursor write (drives the pan re-projection). Also
    /// the live cursor world point the rubber-band leg draws to while the chain is `drawing`.
    cursor: RwSignal<Option<(f64, f64, Option<f64>)>>,
    /// ~1 Hz zoom heartbeat — the rAF debug sampler (a wheel-zoom with a still pointer still
    /// re-projects within a second). `Option` so the mount can forward `Some(debug_hud)`.
    debug_hud: Option<RwSignal<String>>,
    /// Bumped by the host on every chain mutation so a click repaints even with a still pointer.
    tick: RwSignal<u64>,
) -> impl IntoView {
    // (legs, vertices, rubber-band [(x1,y1,x2,y2)]) projected through the live camera for the
    // current chain + camera state. The rubber-band is the un-committed leg from the last vertex to
    // the live cursor while `drawing` (a single-element vec, or empty).
    #[allow(clippy::type_complexity)]
    let projected =
        move || -> (Vec<ProjectedLeg>, Vec<ProjectedVertex>, Vec<(f64, f64, f64, f64)>) {
            // Subscribe to all three heartbeats so the closure re-runs on pan (cursor), zoom (hud)
            // and any chain edit (tick).
            let cur = cursor.get();
            if let Some(h) = debug_hud {
                let _ = h.get();
            }
            let _ = tick.get();
            let chain = read_registered_chain();
            if chain.is_empty() {
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
                // The canvas is full-bleed (like MapGridRefs), so the camera viewport IS the whole
                // window; build it exactly as `select_tool::frozen_camera` does.
                let cam = crate::select_tool::frozen_camera(vw, vh, tx, ty, zoom);
                let project = move |x: f64, y: f64| {
                    let p = cam.project([x, y, 0.0]);
                    (p[0], p[1])
                };
                // Rubber-band: last committed vertex → live cursor, only while drawing and with a
                // live on-map cursor. It previews the leg-to-be so the operator aims the next click.
                let mut rubber = Vec::new();
                if chain.drawing {
                    if let (Some(last), Some((cwx, cwy, _))) = (chain.points.last().copied(), cur) {
                        let (x1, y1) = project(last.x, last.y);
                        let (x2, y2) = project(cwx, cwy);
                        rubber.push((x1, y1, x2, y2));
                    }
                }
                (
                    project_legs(&chain, &project),
                    project_vertices(&chain, &project),
                    rubber,
                )
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                let _ = (chain, cur);
                (Vec::new(), Vec::new(), Vec::new())
            }
        };
    view! {
        // Full-bleed, non-interactive SVG. z-10 sits it in the same overlay band as MapGridRefs so
        // it draws over the map but under the chrome docks; `pointer-events-none` so it never eats a
        // map gesture (the click-chain capture is the map's own pointer handlers, not this SVG).
        <svg
            data-ruler-overlay
            class="pointer-events-none absolute inset-0 z-10"
            width="100%"
            height="100%"
        >
            // Placed polyline legs — one <line> per committed leg, keyed by world coordinate (T-727).
            <For
                each=move || projected().0
                key=|l| l.key.clone()
                let:l
            >
                <line
                    x1=move || format!("{:.1}", l.x1)
                    y1=move || format!("{:.1}", l.y1)
                    x2=move || format!("{:.1}", l.x2)
                    y2=move || format!("{:.1}", l.y2)
                    class="stroke-primary"
                    stroke-width="1.5"
                />
            </For>
            // Rubber-band preview — the un-committed leg from the last vertex to the live cursor
            // (dashed, to read as provisional vs the solid committed legs). One or zero lines.
            <For
                each=move || projected().2
                key=|r| format!("{:.0}:{:.0}:{:.0}:{:.0}", r.0, r.1, r.2, r.3)
                let:r
            >
                <line
                    x1=move || format!("{:.1}", r.0)
                    y1=move || format!("{:.1}", r.1)
                    x2=move || format!("{:.1}", r.2)
                    y2=move || format!("{:.1}", r.3)
                    class="stroke-primary/60"
                    stroke-width="1.5"
                    stroke-dasharray="4 4"
                />
            </For>
            // Vertex dots — a small ring at each committed point, keyed by world coordinate.
            <For
                each=move || projected().1
                key=|v| v.key.clone()
                let:v
            >
                <circle
                    cx=move || format!("{:.1}", v.px)
                    cy=move || format!("{:.1}", v.py)
                    r="3"
                    class="fill-surface-container-lowest stroke-primary"
                    stroke-width="1.5"
                />
            </For>
            // On-line leg labels (Decision 1) — horizontal, centred on the leg mid-point, keyed by
            // world coordinate so an identical label string on two legs never shares a node (T-727).
            <For
                each=move || projected().0
                key=|l| l.key.clone()
                let:l
            >
                <text
                    x=move || format!("{:.1}", l.mid_x)
                    y=move || format!("{:.1}", l.mid_y - 4.0)
                    text-anchor="middle"
                    class="fill-primary font-mono text-code-md"
                    // A faint halo so the label reads over any basemap (paint-order: stroke first).
                    stroke="rgba(0,0,0,0.55)"
                    stroke-width="3"
                    style="paint-order:stroke"
                >
                    {l.label.clone()}
                </text>
            </For>
        </svg>
    }
}

// ── Tests: the chain state machine, per-leg math goldens, formatter goldens, label keying, and the
//    fired bearing rule. Native (`cargo test -p website-frontend`) — no browser/engine. ───────────

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64) -> RulerPoint {
        RulerPoint::new(x, y, None)
    }
    fn pz(x: f64, y: f64, z: f64) -> RulerPoint {
        RulerPoint::new(x, y, Some(z))
    }

    // ── tool-mode arbitration (button filter + Select passthrough) ──────────────────────────────

    #[test]
    fn should_begin_ruler_button_and_tool_gating() {
        // Ruler tool + LEFT button → a ruler press.
        assert!(should_begin_ruler(EditorTool::Ruler, 0));
        // Ruler tool + non-left buttons → NOT a ruler press (MMB pan / RMB menu untouched) — (c).
        assert!(
            !should_begin_ruler(EditorTool::Ruler, 1),
            "middle stays pan"
        );
        assert!(
            !should_begin_ruler(EditorTool::Ruler, 2),
            "right stays context menu"
        );
        // Select tool → never a ruler press, on any button (the Select machine is unchanged).
        assert!(!should_begin_ruler(EditorTool::Select, 0));
        assert!(!should_begin_ruler(EditorTool::Select, 1));
        assert_eq!(
            EditorTool::default(),
            EditorTool::Select,
            "Select is the default tool"
        );
        assert!(EditorTool::Ruler.is_ruler() && !EditorTool::Select.is_ruler());
    }

    // ── distance / bearing / slope goldens ──────────────────────────────────────────────────────

    #[test]
    fn distance_is_euclidean_world_m() {
        assert!((distance_m(p(0.0, 0.0), p(3.0, 4.0)) - 5.0).abs() < 1e-9);
        assert!((distance_m(p(100.0, 100.0), p(100.0, 100.0))).abs() < 1e-12); // zero-length
        assert!((distance_m(p(0.0, 0.0), p(1000.0, 0.0)) - 1000.0).abs() < 1e-9);
    }

    /// The cardinal edge cases the ticket names — 0 / 90 / 180 / 270 — plus the wrap. Bearing is
    /// clockwise from north with world +Y = north, +X = east.
    #[test]
    fn bearing_cardinal_edges_and_wrap() {
        let o = p(1000.0, 1000.0);
        assert!(
            (bearing_deg(o, p(1000.0, 2000.0)) - 0.0).abs() < 1e-9,
            "due north = 0"
        );
        assert!(
            (bearing_deg(o, p(2000.0, 1000.0)) - 90.0).abs() < 1e-9,
            "due east = 90"
        );
        assert!(
            (bearing_deg(o, p(1000.0, 0.0)) - 180.0).abs() < 1e-9,
            "due south = 180"
        );
        assert!(
            (bearing_deg(o, p(0.0, 1000.0)) - 270.0).abs() < 1e-9,
            "due west = 270"
        );
        // Intercardinal + the [0,360) wrap: NW is 315, not −45.
        assert!(
            (bearing_deg(o, p(0.0, 2000.0)) - 315.0).abs() < 1e-9,
            "NW = 315 (wrap, not -45)"
        );
        assert!(
            (bearing_deg(o, p(2000.0, 2000.0)) - 45.0).abs() < 1e-9,
            "NE = 45"
        );
        // A zero-length leg has no direction → defined 0.
        assert!((bearing_deg(o, o) - 0.0).abs() < 1e-12);
    }

    #[test]
    fn slope_and_delta_elev_goldens() {
        // +8 m over 400 m run = +2% (the ticket's worked example).
        let a = pz(0.0, 0.0, 100.0);
        let b = pz(400.0, 0.0, 108.0);
        assert_eq!(delta_elev_m(a, b), Some(8.0));
        assert!((slope_pct(a, b).unwrap() - 2.0).abs() < 1e-9);
        // Descent is signed.
        let c = pz(0.0, 0.0, 50.0);
        let d = pz(0.0, 100.0, 40.0);
        assert_eq!(delta_elev_m(c, d), Some(-10.0));
        assert!((slope_pct(c, d).unwrap() - -10.0).abs() < 1e-9);
        // Off-coverage on EITHER end → None (no fake 0).
        assert_eq!(delta_elev_m(pz(0.0, 0.0, 1.0), p(1.0, 0.0)), None);
        assert_eq!(slope_pct(pz(0.0, 0.0, 1.0), p(1.0, 0.0)), None);
        // Zero-run leg → no grade even with a rise.
        assert_eq!(slope_pct(pz(5.0, 5.0, 10.0), pz(5.0, 5.0, 20.0)), None);
    }

    // ── formatter goldens ───────────────────────────────────────────────────────────────────────

    #[test]
    fn leg_distance_formatter() {
        assert_eq!(format_leg_distance(412.0), "412 m");
        assert_eq!(format_leg_distance(999.4), "999 m");
        assert_eq!(format_leg_distance(1000.0), "1.00 km");
        assert_eq!(format_leg_distance(1240.0), "1.24 km");
    }

    #[test]
    fn bearing_formatter_zero_padded_one_decimal() {
        assert_eq!(format_bearing(73.2), "073.2°");
        assert_eq!(format_bearing(0.0), "000.0°");
        assert_eq!(format_bearing(90.0), "090.0°");
        assert_eq!(format_bearing(180.04), "180.0°");
        assert_eq!(format_bearing(359.97), "000.0°"); // rounds to 360 → wraps to 000, never "360.0"
    }

    #[test]
    fn delta_elev_and_slope_formatters() {
        assert_eq!(format_delta_elev(8.0), "+8 m");
        assert_eq!(format_delta_elev(-3.0), "-3 m");
        assert_eq!(format_delta_elev(0.0), "+0 m");
        // Slope is an UNSIGNED magnitude in the leg label — direction is on the Δelev clause.
        assert_eq!(format_slope(2.0), "2%");
        assert_eq!(format_slope(-5.0), "5%"); // descent grade printed as magnitude
        assert_eq!(format_slope(0.3), "0%"); // sub-1% reads flat
    }

    #[test]
    fn total_formatter_and_leg_label_shape() {
        assert_eq!(format_total(850.0), "Σ 850 m");
        assert_eq!(format_total(1240.0), "Σ 1.24 km");
        // The full leg label matches the ticket's exact shape.
        let leg = Leg::between(pz(0.0, 0.0, 100.0), pz(0.0, 412.0, 108.0));
        assert_eq!(leg.label(), "412 m · 000.0° · +8 m (2%)");
        // Off-coverage leg drops the elevation clause entirely (no fake rise). 3-4-5 triangle:
        // dx=300 (east), dy=400 (north) → dist 500 m, bearing atan2(300,400)=36.87° → 036.9°.
        let bare = Leg::between(p(0.0, 0.0), p(300.0, 400.0));
        assert_eq!(bare.label(), "500 m · 036.9°");
    }

    // ── chain state machine: add / end / clear / Esc paths (Decision 3) ─────────────────────────

    #[test]
    fn press_appends_and_arms_drawing() {
        let mut c = RulerChain::new();
        assert!(c.is_empty() && !c.drawing);
        c.press(0.0, 0.0, None);
        assert_eq!(c.points.len(), 1);
        assert!(c.drawing, "first press arms drawing");
        c.press(100.0, 0.0, Some(5.0));
        assert_eq!(c.points.len(), 2);
        assert_eq!(c.legs().len(), 1);
        assert!((c.total_m() - 100.0).abs() < 1e-9);
    }

    #[test]
    fn double_click_ends_but_keeps_placed() {
        let mut c = RulerChain::new();
        c.press(0.0, 0.0, None);
        c.press(100.0, 0.0, None);
        c.press(100.0, 100.0, None);
        c.double_click();
        assert!(!c.drawing, "dbl-click ends drawing");
        assert_eq!(
            c.points.len(),
            3,
            "dbl-click KEEPS the placed points (Decision 3)"
        );
        assert_eq!(c.legs().len(), 2);
    }

    #[test]
    fn dedup_tail_removes_coincident_final_vertex() {
        let mut c = RulerChain::new();
        c.press(0.0, 0.0, None);
        c.press(100.0, 0.0, None);
        c.press(100.0, 0.0, None); // dbl-click's coincident second press
        assert!(c.dedup_tail(0.5), "coincident tail removed");
        assert_eq!(c.points.len(), 2);
        // A genuine distinct tail is NOT removed.
        c.press(100.0, 50.0, None);
        assert!(!c.dedup_tail(0.5));
        assert_eq!(c.points.len(), 3);
    }

    /// Esc is the two-step escalating dismissal (Decision 3): first drops the in-progress tail
    /// (keeps a legged measure), a second Esc clears the placed points; a lone un-legged vertex
    /// clears on the first Esc.
    #[test]
    fn escape_two_step_dismissal() {
        let mut c = RulerChain::new();
        // Nothing to dismiss.
        assert!(!c.escape());
        // Multi-vertex, drawing → first Esc keeps points, stops drawing.
        c.press(0.0, 0.0, None);
        c.press(100.0, 0.0, None);
        assert!(c.drawing);
        assert!(c.escape(), "first Esc acts");
        assert!(!c.drawing, "first Esc stops drawing");
        assert_eq!(c.points.len(), 2, "first Esc KEEPS the legged measure");
        // Second Esc (now placed / not drawing) → clears.
        assert!(c.escape(), "second Esc acts");
        assert!(c.is_empty(), "second Esc clears the placed ruler");
        // A lone un-legged vertex clears on the FIRST Esc (nothing worth keeping).
        c.press(5.0, 5.0, None);
        assert_eq!(c.points.len(), 1);
        assert!(c.escape());
        assert!(c.is_empty(), "lone vertex clears on first Esc");
    }

    #[test]
    fn clear_is_idempotent_and_total() {
        let mut c = RulerChain::new();
        c.press(0.0, 0.0, None);
        c.press(1.0, 0.0, None);
        c.clear();
        assert!(c.is_empty() && !c.drawing);
        c.clear(); // idempotent
        assert!(c.is_empty());
    }

    #[test]
    fn status_readout_shows_total_and_last_leg() {
        let mut c = RulerChain::new();
        assert_eq!(c.status_readout(), None, "no legs → no readout");
        c.press(0.0, 0.0, Some(100.0));
        assert_eq!(c.status_readout(), None, "lone vertex → no readout");
        c.press(0.0, 412.0, Some(108.0));
        let s = c.status_readout().unwrap();
        assert!(
            s.starts_with("Σ 412 m · last "),
            "readout leads with the total: {s}"
        );
        assert!(
            s.contains("412 m · 000.0° · +8 m (2%)"),
            "readout carries the last-leg label: {s}"
        );
        // A second leg updates the total and swaps "last" to the newest leg. The 1000 m leg renders
        // in km per the leg formatter (≥1000 m → "1.00 km"), matching the Σ-total's km form.
        c.press(1000.0, 412.0, Some(108.0));
        let s2 = c.status_readout().unwrap();
        assert!(
            s2.starts_with("Σ 1.41 km · last "),
            "total accumulates: {s2}"
        );
        assert!(
            s2.contains("1.00 km · 090.0°"),
            "last-leg swaps to the new leg: {s2}"
        );
    }

    // ── label keying (T-727 world-coord key pin) ────────────────────────────────────────────────

    /// Two DIFFERENT legs that happen to share an identical LABEL STRING must get DIFFERENT `<For>`
    /// keys — the wave-107 T-727 defect (a `<For>` keyed on text retained a stale position). The key
    /// is the world coordinate, so it is distinct even when the label collides.
    #[test]
    fn label_keys_are_world_coords_not_text() {
        // Two legs with the same length + bearing → the SAME label string, at DIFFERENT places.
        let mut c = RulerChain::new();
        c.press(0.0, 0.0, None);
        c.press(0.0, 100.0, None); // leg 1: 100 m due north
        c.press(500.0, 100.0, None); // leg 2 (east), then…
        c.press(500.0, 200.0, None); // leg 3: 100 m due north — identical label to leg 1
        let legs = c.legs();
        let l1 = &legs[0];
        let l3 = &legs[2];
        assert_eq!(
            l1.label(),
            l3.label(),
            "the two north legs share a label string"
        );
        let identity = |x: f64, y: f64| (x, y); // identity projector for the pure test
        let proj = project_legs(&c, identity);
        // Same label, DIFFERENT keys — the whole point of keying by world coordinate.
        assert_eq!(proj[0].label, proj[2].label);
        assert_ne!(
            proj[0].key, proj[2].key,
            "T-727: legs with identical labels must have distinct world-coordinate keys"
        );
        // All keys unique across the chain.
        let mut keys: Vec<String> = proj.iter().map(|p| p.key.clone()).collect();
        keys.sort();
        keys.dedup();
        assert_eq!(keys.len(), proj.len(), "every leg key is unique");
    }

    #[test]
    fn world_key_quantises_and_distinguishes() {
        // Sub-0.1 m jitter maps to the SAME key (no per-frame churn)…
        assert_eq!(
            world_key(10.02, 20.0, 30.0, 40.0),
            world_key(10.03, 20.0, 30.0, 40.0)
        );
        // …but a genuine 1 m move is a different key.
        assert_ne!(
            world_key(10.0, 20.0, 30.0, 40.0),
            world_key(11.0, 20.0, 30.0, 40.0)
        );
    }

    #[test]
    fn project_legs_maps_endpoints_and_midpoint() {
        let mut c = RulerChain::new();
        c.press(0.0, 0.0, None);
        c.press(100.0, 200.0, None);
        // A projector that scales by 2 and offsets — checks endpoints AND midpoint pass through it.
        let proj = project_legs(&c, |x, y| (x * 2.0 + 5.0, y * 2.0 + 7.0));
        assert_eq!(proj.len(), 1);
        let l = &proj[0];
        assert!((l.x1 - 5.0).abs() < 1e-9 && (l.y1 - 7.0).abs() < 1e-9);
        assert!((l.x2 - 205.0).abs() < 1e-9 && (l.y2 - 407.0).abs() < 1e-9);
        // Midpoint world (50,100) → (105, 207).
        assert!((l.mid_x - 105.0).abs() < 1e-9 && (l.mid_y - 207.0).abs() < 1e-9);
    }

    // ── Decision 4 pin: rulers are session-local overlay state, NEVER doc/store writes ───────────

    /// The ruler is a MEASUREMENT, not mission content — Decision 4. This module must therefore
    /// reference NO document mutation: no `store.rs` API, no slot/entity write. Proven on scrubbed
    /// code (comments + strings blanked) so a mention in prose/tests can't satisfy it, and the module
    /// carries no reference to the doc core at all. Combined with the compiler (this file imports no
    /// doc mutator) this is the "no store.rs writes" guarantee: a reload finds a clean map.
    #[test]
    fn no_ruler_doc_writes() {
        let code = crate::arsenal::class_r_scrub::live_code(include_str!("ruler_tool.rs"));
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
                "Decision 4: ruler_tool must be session-local overlay state — found doc-write token \
                 `{banned}`; a ruler must NEVER write the document"
            );
        }
    }

    // ── FIRE THE BEARING RULE (perturb / fail / restore) ────────────────────────────────────────

    /// The bearing convention genuinely discriminates: asserting the WRONG cardinal for a direction
    /// fails, and the RIGHT one passes. A `bearing_deg` that returned a constant, or measured
    /// counter-clockwise, or from east, would pass the perturbed assertion — so this proves the
    /// clockwise-from-north convention is load-bearing, not incidental.
    #[test]
    fn bearing_rule_fires() {
        let o = p(0.0, 0.0);
        let east = p(100.0, 0.0);
        // Baseline: due east is 90° clockwise from north.
        assert!(
            (bearing_deg(o, east) - 90.0).abs() < 1e-9,
            "baseline: east = 90"
        );
        // Perturb: CLAIM east is 0° (which would be true only if bearing measured from east, or
        // returned a constant 0). That is FALSE, so an equality against the perturbed value must NOT
        // hold — the rule fires.
        let perturbed = 0.0;
        assert_ne!(
            (bearing_deg(o, east)).round(),
            perturbed,
            "east must NOT read 0° — if it did, bearing would be measuring from the wrong axis"
        );
        // Perturb the other way: claim east is 270° (counter-clockwise). Also FALSE.
        assert_ne!(
            (bearing_deg(o, east)).round(),
            270.0,
            "east must NOT read 270° — bearing must be CLOCKWISE from north, not counter-clockwise"
        );
        // Restore: the true value holds, and a different direction yields a different bearing (not a
        // constant).
        assert!((bearing_deg(o, east) - 90.0).abs() < 1e-9);
        assert_ne!(
            bearing_deg(o, east).round(),
            bearing_deg(o, p(0.0, 100.0)).round(),
            "bearing must vary with direction (east 90 vs north 0)"
        );
    }
}
