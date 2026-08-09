//! T-661 — the Bottom Toolbelt, split from `eden_chrome.rs`.
//!
//! T-636 splits the single floating pill into TWO mounts, mirroring Eden: the mode buttons
//! (Select / Ruler / LoS) live on a toolbar ([`ModeToolbar`]), and the numeric readouts (CUR / OBJ
//! / SEL / SZ) live in a full-width status bar docked at the bottom of the viewport ([`StatusBar`]).
//! Tools and telemetry are different jobs with different interaction models — one is a set of mode
//! toggles, the other a passive read-out — so conflating them in one ~580 px centred pill was the
//! defect (`editor_chrome_direction.md`). The operator's direction is explicit: the bottom bar
//! STAYS, its content and feel unchanged, stretched to span the viewport instead of floating centred.
//!
//! The status bar also carries the two natural homes the full-width geometry creates:
//!   * a left/centre slot for map furniture — the scale bar and grid references (T-667, wave 106;
//!     built here as an obvious empty slot, NOT filled), and
//!   * a right-end slot for a primary action on its own surface — Eden's `PLAY SCENARIO`; ours is
//!     `OPEN` per `editor_chrome_direction.md` §Open (the slot is built; what the button *does* is
//!     the undecided part of §Open).
//!   * the debug telemetry HUD (T-719) gets a legitimate visible slot in the right section, before
//!     OPEN, still behind its Ctrl+Alt+D toggle and the `chrome_hidden` gate — it was previously
//!     invisible, painted over by DockRight's z-20 column.
//!
//! Not cfg-gated: the native view shell renders both too (the doc-reading `sel_xyz` branch is
//! `#[cfg(target_arch = "wasm32")]` inside the memo).
#![allow(dead_code)]
use leptos::prelude::*;
use map_engine_core::camera::OrthoCamera;

use crate::eden_layout::{HOVER_FILL, TOGGLED_PLATE};
use crate::ui::{cn, MaterialIcon};

// ── T-667 — map furniture: scale bar + edge grid references (pure geometry) ─────────────────────────
//
// The operator decision (registry): a 2D top-down planner EXCEEDS Eden — which ships no scale bar,
// no legend, no grid coordinate labels — with the two distance cues a plan view actually needs:
// a metric scale bar and grid-reference labels framing the MAP PANE. The legend is deliberately
// skipped. Both are DOM chrome (not GPU glyphs like the T-641 spot heights), so their maths lives
// here as pure functions the native `cargo test` proves; the Leptos components below are thin
// wrappers that read the live camera and render what these return.
//
// Everything in this module is `deck_zoom`-driven off the ONE engine convention:
//   `m_per_px = 2^(−deck_zoom)`   (T-639/T-641; `slots_gpu::px_to_m_at_zoom`, `ortho.rs` `scale =
//   2^zoom` px/m). Larger m/pix = zoomed further out.

/// Metres-per-pixel at a given `deck_zoom` — the single scale convention (`2^(−deck_zoom)`), cited
/// against `lod_gates`/`ortho.rs`. Non-finite ⇒ `f64::NAN` so [`format_m_per_px`] prints the
/// em-dash cell rather than a fabricated `1.00 m/px` (T-756; the old T-667 unit-scale fallback
/// looked measured once the readout claimed three significant figures). Callers only ever pass a
/// finite engine zoom inside the live clamp.
#[must_use]
pub fn m_per_px(deck_zoom: f64) -> f64 {
    if deck_zoom.is_finite() {
        2.0_f64.powf(-deck_zoom)
    } else {
        f64::NAN
    }
}

/// The drawn grid's line spacing in world metres — the procedural 1 km grid
/// (`map_engine_render::lanes::grid_lines`, `GRID_STEP = 1000`). Grid-reference labels enumerate
/// lines at world multiples of this, so a label can never drift off the line it names. The
/// `labels_match_grid_lines` test pins this against the live `grid_lines` output (not a private
/// const copy) so a future re-space of the grid fails here rather than silently mislabelling.
pub const GRID_STEP_M: f64 = 1000.0;

/// Everon terrain span (metres, square). The grid is drawn only over `[0, TERRAIN_SPAN_M]`
/// (`lanes::grid_lines` loops `x = 0..width`), so grid references outside it do not exist — the
/// edge-label enumeration clamps to this so a pane edge scrolled past the terrain (negative or
/// over-terrain world coords) emits no phantom label. Matches `select_tool`'s `TERRAIN_W/H`.
pub const TERRAIN_SPAN_M: f64 = 12_800.0;

/// Scale-bar target width band (CSS px). The picker takes the LARGEST round distance whose bar is
/// ≤ [`SCALE_MAX_PX`]; the 1-2-5 ladder then keeps the drawn bar roughly [`SCALE_MIN_PX`]–
/// [`SCALE_MAX_PX`] (worst case ~80 px at the 2→5 rung boundary — a 1-2-5 ladder cannot hold a
/// tighter band, and a scale bar reads fine there).
pub const SCALE_MAX_PX: f64 = 200.0;
/// Nominal lower edge of the scale-bar width band (informational; the picker keys off the max).
pub const SCALE_MIN_PX: f64 = 120.0;

/// A resolved scale bar: the chosen round ground distance, the on-screen bar length, and the label.
#[derive(Clone, Debug, PartialEq)]
pub struct ScaleBarSpec {
    /// Chosen round distance in metres (a `1/2/5 × 10^n` value).
    pub dist_m: f64,
    /// On-screen bar length in CSS px = `dist_m / m_per_px`.
    pub width_px: f64,
    /// Human label, e.g. `"500 m"` or `"2 km"`.
    pub label: String,
}

/// Format a round metric distance: sub-1000 m as `"N m"`, ≥1000 m as `"N km"` (integer km when
/// whole, else one decimal — the 1-2-5 ladder only ever yields whole or `.5` km, e.g. `500 m`,
/// `2 km`, `5 km`, `10 km`).
#[must_use]
pub fn format_distance(dist_m: f64) -> String {
    if dist_m >= 1000.0 {
        let km = dist_m / 1000.0;
        if (km.round() - km).abs() < 1e-9 {
            format!("{} km", km.round() as i64)
        } else {
            format!("{km:.1} km")
        }
    } else {
        format!("{} m", dist_m.round() as i64)
    }
}

/// T-670 — format a screen scale for the status bar's numeric SCALE readout, e.g. `"4.00 m/px"`.
/// This is the number Eden prints in its status bar, and it is [`m_per_px`] — the SAME quantity the
/// T-667 scale bar sizes from and the SAME quantity T-639's contour ladder reasons about
/// (`map_engine_core::world::lod_gates::contour_interval_for_zoom` takes `m_per_px` and documents
/// the identical `2^(−deckZoom)` convention), so the printed value is a true on-screen check of the
/// ladder rather than a lookalike second computation.
///
/// **Three significant figures across the whole zoom clamp** — `MIN_ZOOM −6` ⇒ `64.0 m/px`,
/// `MAX_ZOOM 6` ⇒ `0.0156 m/px` — so the cell keeps a steady width AND the printed number stays
/// within 0.5% of the live scale everywhere. That second property is what lets the readout be a
/// real check of T-639's ladder at close zoom: a fixed decimal count would have decayed to two
/// significant figures below 0.1 m/px and started printing a number the ladder does not use.
///
/// The STRING is also the quantiser: the editor's rAF sampler writes its zoom signal only when this
/// formatting CHANGES, which is what keeps a per-frame zoom read from re-rendering the status bar at
/// 60 fps. Degenerate input (non-finite or ≤ 0) ⇒ an em-dash cell, matching the other readouts'
/// "no value" idiom. That path also covers a non-finite *zoom* once [`m_per_px`] returns `NAN`
/// (T-756) — previously `m_per_px` mapped NaN→1.0 and this printed a confident `"1.00 m/px"`.
#[must_use]
pub fn format_m_per_px(m_per_px: f64) -> String {
    if !m_per_px.is_finite() || m_per_px <= 0.0 {
        return "— m/px".to_string();
    }
    // Decimals for ~3 significant figures at this magnitude. The last two rungs are below the
    // MAX_ZOOM floor (0.0156 m/px) and exist only so a future zoom-ceiling raise degrades sanely.
    // After picking the band, re-check the *rounded* value: a band-top carry (9.996 → 10.00 with
    // two decimals) would otherwise print four significant figures; drop to the next band's width
    // so carry reads `10.0` (T-756).
    let decimals = decimals_for_mpp(m_per_px);
    let factor = 10f64.powi(decimals as i32);
    let rounded = (m_per_px * factor).round() / factor;
    let decimals = if rounded.is_finite() && rounded > 0.0 {
        decimals_for_mpp(rounded)
    } else {
        decimals
    };
    format!("{m_per_px:.decimals$} m/px")
}

/// Decimal count for ~3 significant figures at `m_per_px`'s magnitude. Shared by the band pick and
/// the post-round carry re-pick inside [`format_m_per_px`].
fn decimals_for_mpp(m_per_px: f64) -> usize {
    if m_per_px >= 100.0 {
        0
    } else if m_per_px >= 10.0 {
        1
    } else if m_per_px >= 1.0 {
        2
    } else if m_per_px >= 0.1 {
        3
    } else if m_per_px >= 0.01 {
        4
    } else {
        5
    }
}

/// Pick the scale bar for a screen scale of `m_per_px`: the LARGEST `1/2/5 × 10^n` metres whose bar
/// (`dist / m_per_px`) is ≤ [`SCALE_MAX_PX`]. Live-updates on zoom because `m_per_px` does.
///
/// Walks the 1-2-5 ladder from a coarse ceiling down to the first (largest) value that fits, so the
/// bar is always the widest round distance under the cap. Degenerate `m_per_px` (≤ 0 or non-finite)
/// ⇒ the finest rung (1 m) as a safe floor.
#[must_use]
pub fn pick_scale_bar(m_per_px: f64) -> ScaleBarSpec {
    // Ladder mantissas per decade, descending, so the first fit is the largest.
    const MANTISSA: [f64; 3] = [5.0, 2.0, 1.0];
    if !m_per_px.is_finite() || m_per_px <= 0.0 {
        return ScaleBarSpec {
            dist_m: 1.0,
            width_px: 1.0,
            label: format_distance(1.0),
        };
    }
    // Decades from 10^7 m (10000 km, well past whole-Everon) down to 10^0 m.
    for exp in (0..=7).rev() {
        let decade = 10.0_f64.powi(exp);
        for m in MANTISSA {
            let dist = m * decade;
            let width = dist / m_per_px;
            if width <= SCALE_MAX_PX {
                return ScaleBarSpec {
                    dist_m: dist,
                    width_px: width,
                    label: format_distance(dist),
                };
            }
        }
    }
    // m_per_px so small even 1 m overflows the cap (zoomed past MAX_ZOOM — unreachable): finest rung.
    ScaleBarSpec {
        dist_m: 1.0,
        width_px: 1.0 / m_per_px,
        label: format_distance(1.0),
    }
}

/// Arma grid reference for a world coordinate: 3-digit **hundreds-of-metres**, wrapping every
/// 100 km (`floor(m / 100) mod 1000`, zero-padded). E.g. `6400 m → 064`, `12000 m → 120`,
/// `0 m → 000`. This is the six-figure military-grid half a single axis contributes (the
/// `mortar.rs` "012 020" convention). Negative or non-finite ⇒ `000` (off-terrain guard).
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

/// One edge grid-reference label: the CSS-pixel position along the anchoring edge and the 3-digit
/// text. For an easting (top edge) `pos_px` is the screen X of the vertical grid line; for a
/// northing (left edge) it is the screen Y of the horizontal line.
#[derive(Clone, Debug, PartialEq)]
pub struct EdgeLabel {
    /// CSS px along the edge (screen X for eastings, screen Y for northings).
    pub pos_px: f64,
    /// 3-digit hundreds-of-metres reference.
    pub text: String,
    /// `<For>` identity — the wave-107 **T-727** keying fix, applied to grids (T-793 / O-2).
    /// It encodes the label's axis + its live SCREEN position (quantised to the whole pixel via
    /// [`edge_label_key`]), NOT the display text. A key that were the text (`"090"`) would be
    /// retained across a pan, so Leptos would reuse that node **unchanged** and its `left:` would
    /// stay frozen at the pre-pan `pos_px` (`<For>` "avoids re-creating DOM nodes that are not being
    /// changed", and `pos_px` is a plain field captured once by `let:l` — no inner signal re-reads).
    /// That is exactly the O-2 defect the hostile review caught: a moved label held its old screen x
    /// while a freshly-scrolled-in neighbour sat at the new x, leaving two km labels 70 px apart at
    /// 4 m/px where they MUST be 250 px. Keying on the pixel means any pan/zoom that moves the label
    /// mints a new key ⇒ a new node at the correct `left:`; the whole set updates every frame. Same
    /// idiom as the ruler's rubber-band leg, which keys on its screen coords for the same reason.
    pub key: String,
}

/// The [`EdgeLabel::key`] for a label of `text` on axis `axis` (`'E'`/`'N'`) at screen `pos_px`.
/// Quantised to the whole CSS pixel: a sub-pixel pan does not churn the node every frame (below one
/// pixel there is nothing to redraw), but any visible move — the O-2 case — changes the key and
/// forces a freshly-positioned node. The `text` is folded in only to disambiguate the (rare) frame
/// where two different refs momentarily round to the same pixel during a fast pan; the pixel is what
/// makes the key bust on movement.
#[must_use]
fn edge_label_key(axis: char, pos_px: f64, text: &str) -> String {
    format!("{axis}{}:{text}", pos_px.round() as i64)
}

/// Eastings for the map pane's TOP edge: the vertical grid lines whose screen-X lands inside the
/// pane's horizontal span `[pane_left_px, pane_right_px]`. The world span of that edge is
/// unprojected from the two edge pixels (at the top row `top_px`), grid lines inside it are
/// enumerated, and each is projected BACK to a screen X via the SAME `OrthoCamera::project` the GPU
/// draw uses — so a label sits exactly on its line. Lines that fall outside the pane (occluded by a
/// dock) are dropped, which is the correct Eden-exceeding geometry: refs frame the MAP, not the
/// window.
#[must_use]
pub fn edge_eastings(
    cam: &OrthoCamera,
    pane_left_px: f64,
    pane_right_px: f64,
    top_px: f64,
) -> Vec<EdgeLabel> {
    if pane_right_px <= pane_left_px {
        return Vec::new();
    }
    // World X at the pane's left and right screen edges (top row). The camera is north-up with no
    // rotation, so a screen X maps to a single world X regardless of the row; `top_px` is used so
    // the round-trip is exact against the drawn line. Clamp the span to the terrain — the grid is
    // only drawn over [0, TERRAIN_SPAN_M], so a pane edge scrolled off-terrain emits no phantom ref.
    let wl = cam.unproject_xy(pane_left_px, top_px)[0].clamp(0.0, TERRAIN_SPAN_M);
    let wr = cam.unproject_xy(pane_right_px, top_px)[0].clamp(0.0, TERRAIN_SPAN_M);
    let mut out = Vec::new();
    for wx in grid_lines_in_range(wl, wr) {
        let sx = cam.project([wx, cam.target_y(), 0.0])[0];
        // Guard the float edges: keep only lines that project strictly inside the visible pane span.
        if sx >= pane_left_px - 0.5 && sx <= pane_right_px + 0.5 {
            let text = grid_ref_3digit(wx);
            out.push(EdgeLabel {
                pos_px: sx,
                key: edge_label_key('E', sx, &text),
                text,
            });
        }
    }
    out
}

/// Northings for the map pane's LEFT edge: the horizontal grid lines whose screen-Y lands inside
/// the pane's vertical span `[top_px, bottom_px]`. Mirror of [`edge_eastings`] on the Y axis. Note
/// the screen Y axis is inverted vs world Y (north up), so the world span is `[bottom, top]` in
/// world metres; `grid_lines_in_range` is order-agnostic and each line is projected back for an
/// exact on-line Y.
#[must_use]
pub fn edge_northings(
    cam: &OrthoCamera,
    pane_left_px: f64,
    top_px: f64,
    bottom_px: f64,
) -> Vec<EdgeLabel> {
    if bottom_px <= top_px {
        return Vec::new();
    }
    let w_top = cam.unproject_xy(pane_left_px, top_px)[1].clamp(0.0, TERRAIN_SPAN_M);
    let w_bottom = cam.unproject_xy(pane_left_px, bottom_px)[1].clamp(0.0, TERRAIN_SPAN_M);
    let mut out = Vec::new();
    for wy in grid_lines_in_range(w_bottom, w_top) {
        let sy = cam.project([cam.target_x(), wy, 0.0])[1];
        if sy >= top_px - 0.5 && sy <= bottom_px + 0.5 {
            let text = grid_ref_3digit(wy);
            out.push(EdgeLabel {
                pos_px: sy,
                key: edge_label_key('N', sy, &text),
                text,
            });
        }
    }
    out
}

// ── Toolbelt class recipes (React `overlay.ts`) ────────────────────────────────────────────────────

/// The floating mode-toolbar pill — `cn(overlayPanel, 'flex items-center gap-1 px-1.5 py-1.5')`.
/// This is the tools half of the old TOOLBELT recipe; the readouts half moved to the status bar.
const MODEBAR: &str = "pointer-events-auto rounded-xl border border-white/10 bg-surface-container-lowest/55 shadow-xl backdrop-blur-xl flex items-center gap-1 px-1.5 py-1.5";

/// The full-width status bar surface — the `overlayDocked` glass (same tokens as the docks/strip),
/// stretched edge-to-edge across the bottom. `border-t` gives it the docked seam Eden's status bar
/// has. It is docked `inset-x-0 bottom-0`, so its top edge sits [`STATUSBAR_H_PX`] px up from the
/// viewport bottom; the (much taller) [`crate::eden_layout::TOOLBELT_BAND_PX`] is the *input* band a
/// pointer probe must clear, a separate contract from this bar's painted height.
const STATUSBAR: &str = "pointer-events-auto bg-surface-container-lowest/55 shadow-xl backdrop-blur-xl flex h-9 w-full items-center gap-3 border-t border-white/10 px-3";

/// T-787 — the status bar's rendered HEIGHT in CSS px (`h-9` in [`STATUSBAR`] → 36 px). This is the
/// SOURCE OF TRUTH for how far the bar's top edge sits above the viewport bottom, exported so
/// `eden_layout`'s [`crate::eden_layout::dock_bottom_px`] can inset the docks to STOP at that top
/// edge instead of overlapping it (the O-1 defect: the transparent dock containers ran to
/// `bottom-0` and ate clicks aimed at the readouts + right-end controls). A test below pins this to
/// the `h-*` token in [`STATUSBAR`] so the two can never drift. Distinct from
/// [`crate::eden_layout::TOOLBELT_BAND_PX`], which is the input-handling band (clears the taller
/// floating [`ModeToolbar`]) and deliberately does not shrink the full-bleed canvas.
pub const STATUSBAR_H_PX: f64 = 36.0;

/// T-668 — the tool button's shared GEOMETRY (no state colour). The three states are composed from
/// this base + the one state vocabulary: current mode = [`TOGGLED_PLATE`], a live-but-not-current
/// mode = [`HOVER_FILL`], and a disabled stub would add `crate::eden_layout::DISABLED_GLYPH` (all
/// three tools ship live today, so no button wears the disabled recipe here). Keeping the geometry in
/// one const and the state in the recipes is what stops a fourth ad-hoc "active" tint creeping back.
const TOOL_BASE: &str = "flex items-center gap-1.5 rounded-lg px-2.5 py-1.5 text-label-md";

/// Format a cursor axis for the mono readout. React `BottomToolbelt.fmtCoord`:
/// `n.toFixed(3).padStart(9, ' ')`, and the off-map cell is 7 spaces + an em dash. HTML collapses
/// the leading runs in both engines — `tabular-nums` does the real aligning — so this mirrors the
/// oracle rather than "fixing" it.
fn fmt_coord(v: Option<f64>) -> String {
    match v {
        Some(n) => format!("{n:>9.3}"),
        None => "       —".to_string(),
    }
}

/// F-13 (T-807 review batch, taken here by the wave-201 orchestrator addendum because the readout
/// half lives in THIS file, not `mission_editor`): the Eden-style unit-suffixed cursor axis. Eden's
/// status bar reads `X 8762.61 m` (review Eden reference, frame 170422) — the coordinate carries a
/// ` m` unit. This is [`fmt_coord`] (the exact React `fmtCoord` precision, unchanged — no value or
/// precision is lost, the readout stays correct to the metre so it remains the trusted oracle the
/// grid-label acceptance test unprojects against) with a **presentation-only** ` m` appended to a
/// real value. The off-map "no value" cell stays the bare em-dash — a unit on "nothing" would be a
/// lie. Kept separate from `fmt_coord` so the pure React-parity mirror (and its sibling precision
/// contract in `attributes.rs`) is untouched; only the on-screen presentation gains the unit.
fn fmt_coord_eden(v: Option<f64>) -> String {
    match v {
        // Keep `fmt_coord`'s right-aligned numeric field (the decimal points still line up under
        // `tabular-nums`); the ` m` trails the number exactly as Eden prints it.
        Some(_) => format!("{} m", fmt_coord(v)),
        None => fmt_coord(None),
    }
}

/// The mode toolbar — Select ⇆ Ruler ⇆ LoS, all THREE now LIVE (Select always; Ruler T-642, wave
/// 108; LoS T-643, wave 109). Tools only; different job from the readouts, so a separate mount
/// (T-636). It floats above the full-width [`StatusBar`], keeping the operator's "content and feel
/// unchanged" — the same three buttons in the same pill, just no longer sharing the strip with
/// telemetry.
///
/// T-643 — the LoS button is the point of THIS ticket: it drops `disabled` and becomes a real mode
/// toggle, exactly as T-642 did for Ruler. THE HONESTY RULE it honours (removing `disabled` without
/// a working tool is worse than an honest stub — the corpus has two dead-control cautionary tales) is
/// satisfied because it only enables now that `los_tool` works end-to-end: clicking it sets
/// `tool_mode = LoS` (the TOGGLED_PLATE state + `aria-pressed`), and the map's two-click capture +
/// inline profile panel are live behind it. Each button is active exactly when its tool is the
/// current `tool_mode` — the shared signal the pointer handlers read — so a button and the live tool
/// can never disagree, and clicking any button switches the mode (which also clears the other tools'
/// overlays via the tool-switch Effect in `mission_editor`).
///
/// T-644 (wave 110) — the ONE LoS button now carries a SUB-MODE ([`los_tool::LosMode`]): the first
/// click FROM ANOTHER TOOL activates LoS in whichever sub-mode it last showed; a RE-CLICK while LoS
/// is already active TOGGLES the sub-mode (Ray ⇆ Viewshed, `LosMode::toggled`) — the UX decision
/// `los_tool` documents. The button's title and label reflect the live sub-mode ("Line of sight
/// (ray)" click-two-points vs "(viewshed)" click-one-observer-disc) so the operator always knows
/// which they're in; the pointer commit in `mission_editor` branches on the same `los_mode` signal to
/// route a click to the ray capture or the one-shot viewshed placement. Switching sub-mode clears the
/// other's overlay via the same tool-switch Effect (extended to the viewshed lane).
#[component]
pub fn ModeToolbar(
    /// The active editor tool (shared with the map pointer handlers). Reading it tints the active
    /// button; the buttons set it.
    tool_mode: RwSignal<crate::ruler_tool::EditorTool>,
    /// T-644 — the LoS sub-mode (Ray ⇆ Viewshed). Read here to reflect the active sub-mode in the LoS
    /// button's title/label and toggled by a re-click of the LoS button while LoS is already active;
    /// the map pointer commit reads the SAME signal to route a click. Shared with `mission_editor`.
    los_mode: RwSignal<crate::los_tool::LosMode>,
) -> impl IntoView {
    use crate::ruler_tool::EditorTool;
    // T-668 — the current mode wears TOGGLED_PLATE (plate + 1px dark top border); a live-but-not-
    // current mode wears HOVER_FILL. Same one state language as every other toggle in the chrome, so
    // the active tool reads the same as an open menu or a selected tree row — and can never be
    // mistaken for a merely-hovered one (a hovered inactive tool fills; only the active one has the
    // top border). With three live tools this is a direct per-tool equality.
    let cls = move |mine: EditorTool| {
        if tool_mode.get() == mine {
            cn(&[TOOL_BASE, TOGGLED_PLATE])
        } else {
            cn(&[TOOL_BASE, "text-on-surface-variant", HOVER_FILL])
        }
    };
    let pressed = move |mine: EditorTool| (tool_mode.get() == mine).to_string();
    view! {
        <div class=MODEBAR>
            <button
                type="button"
                class=move || cls(EditorTool::Select)
                aria-pressed=move || pressed(EditorTool::Select)
                title="Select"
                on:pointerdown=move |_| tool_mode.set(EditorTool::Select)
            >
                <MaterialIcon name="arrow_selector_tool" class="block text-base" />
                <span class="hidden sm:inline">"Select"</span>
            </button>
            <button
                type="button"
                class=move || cls(EditorTool::Ruler)
                aria-pressed=move || pressed(EditorTool::Ruler)
                title="Ruler — click a chain of points; Esc clears, double-click ends"
                on:pointerdown=move |_| tool_mode.set(EditorTool::Ruler)
            >
                <MaterialIcon name="straighten" class="block text-base" />
                <span class="hidden sm:inline">"Ruler"</span>
            </button>
            <button
                type="button"
                class=move || cls(EditorTool::LoS)
                aria-pressed=move || pressed(EditorTool::LoS)
                // T-644 — the title reflects the live SUB-MODE so the operator always knows which LoS
                // they're in (ray = click two points; viewshed = click one observer → shade the disc).
                title=move || {
                    if los_mode.get().is_viewshed() {
                        "Line of sight (viewshed) — click one observer to shade the visible disc; \
                         click LoS again for ray; Esc clears"
                    } else {
                        "Line of sight (ray) — click observer, click target; click LoS again for \
                         viewshed; Esc clears"
                    }
                }
                // T-644 — the LoS button's re-click toggles the sub-mode. THE UX DECISION (`los_tool`
                // `LosMode`): the first click from ANOTHER tool just activates LoS (leaving the
                // sub-mode on whatever it last showed — `LosMode::toggled`'s "fresh switch lands on the
                // mode it last showed" contract); a re-click while LoS is ALREADY active advances the
                // sub-mode Ray ⇆ Viewshed. `tool_mode.set(EditorTool::LoS)` stays present on both paths
                // (idempotent when already LoS) so the button never lies about which tool it selects.
                on:pointerdown=move |_| {
                    if tool_mode.get_untracked().is_los() {
                        los_mode.update(|m| *m = m.toggled());
                    } else {
                        tool_mode.set(EditorTool::LoS);
                    }
                }
            >
                <MaterialIcon name="visibility" class="block text-base" />
                // T-644 — the label reflects the live sub-mode (ray vs viewshed) on the wide layout,
                // so the active sub-mode is visible at a glance, not only in the tooltip.
                <span class="hidden sm:inline">
                    {move || if los_mode.get().is_viewshed() { "LoS · viewshed" } else { "LoS · ray" }}
                </span>
            </button>
        </div>
    }
}

/// Full-width status bar — the mono CUR X/Y/Z + SEL/OBJ/SZ readout, plus the map-furniture slot
/// (T-667), the debug HUD slot (T-719), and the OPEN primary-action slot (§Open).
///
/// T-172 B2/B9: Z is DEM-fed (em-dash until the grid publishes / off-coverage), and with exactly
/// one slot selected the readout swaps CUR→SEL and shows that slot's x/y/z (React parity). The
/// per-axis `title="Cursor …"` handles stay constant — they are the frozen cur-smoke's DOM hooks.
#[component]
pub fn StatusBar(
    /// Cursor world position + DEM z, `None` when the pointer is off the map (em-dash cells).
    cursor: RwSignal<Option<(f64, f64, Option<f64>)>>,
    sel_count: RwSignal<usize>,
    obj_count: RwSignal<usize>,
    /// Live selection mirror — drives the CUR↔SEL swap.
    selected_ids: RwSignal<Vec<String>>,
    /// T-172 B9 — debounced compiled-payload estimate (None → `—`).
    #[prop(optional)]
    sz_bytes: Option<RwSignal<Option<usize>>>,
    /// T-719 — the wgpu telemetry HUD string (`z … · c… · glyph … · … FPS · rf …ms`); empty until
    /// the rAF sampler has a value. Its own visibility is gated by `hud_shown` (Ctrl+Alt+D) so it
    /// only paints when the operator has asked for it AND has content.
    #[prop(optional)]
    debug_hud: Option<RwSignal<String>>,
    /// T-719 — the Ctrl+Alt+D toggle for the HUD (default hidden). Together with `chrome_hidden`
    /// (which unmounts the whole bar) this keeps the HUD behind exactly the gates T-635 pinned.
    #[prop(optional)]
    hud_shown: Option<RwSignal<bool>>,
    /// T-642 — the ruler's running-total + last-leg readout (Decision 1), `None` when no ruler is
    /// placed. Rendered in the readout section beside CUR/OBJ/SEL — the place the operator already
    /// reads telemetry — so the summary lands where the eye is without hunting the map. `Option` so a
    /// caller with no ruler (the back-compat shim) can omit it.
    #[prop(optional)]
    ruler_status: Option<RwSignal<Option<String>>>,
    /// T-670 — the live screen scale in **metres per pixel**, the one number Eden prints and we did
    /// not. `RenderEngine::zoom()` is reachable only from the editor's rAF sampler, so the sampler
    /// owns this signal and writes it ONLY when [`format_m_per_px`] would change (see
    /// `mission_editor::start_raf`) — a still or panning camera writes nothing, so the status bar
    /// never re-renders per frame. The live mount always *passes* this signal, seeded to
    /// `m_per_px(−2) = 4.0` before the first rAF tick (wave-115 NIT-3: it is not "absent on
    /// native"). The first rAF frame still performs one redundant write because `last_scale_text`
    /// starts empty. `Option` remains for the compat shim / a hypothetical caller that omits it;
    /// absent ⇒ this cell falls back to a *static* `m_per_px(−2)`, while [`ScaleBar`]'s no-prop
    /// path still tries `camera_snapshot()` on wasm — those two would diverge for such a caller,
    /// and no such caller exists today (single mount in `mission_editor`).
    #[prop(optional)]
    scale_mpp: Option<RwSignal<f64>>,
) -> impl IntoView {
    // Exactly-one-selected → that slot's x/y/z from the doc. Recomputes on selection change AND
    // on the post-mutation selected_ids re-set (drag commit), so it never shows a stale position.
    // (`editor_ops` is wasm-only; the native view shell always renders CUR.)
    let sel_xyz = Memo::new(move |_| -> Option<(f64, f64, f64)> {
        let ids = selected_ids.get();
        if ids.len() == 1 {
            #[cfg(target_arch = "wasm32")]
            {
                return crate::editor_ops::read_attrs(&ids[0]).map(|a| (a.x, a.y, a.z));
            }
        }
        let _ = ids;
        None
    });
    // F-13: the axis readout carries Eden's ` m` unit (`fmt_coord_eden`) — presentation only; the
    // underlying x/y/z are the exact doc/cursor values, still correct to the metre.
    let axis_val = move |i: usize| match sel_xyz.get() {
        Some((x, y, z)) => fmt_coord_eden(Some([x, y, z][i])),
        None => fmt_coord_eden(cursor.get().and_then(|c| match i {
            0 => Some(c.0),
            1 => Some(c.1),
            _ => c.2,
        })),
    };
    view! {
        <div class=STATUSBAR>
            // ── Readouts (left) — the old pill's telemetry, verbatim ──────────────────────────────
            <div class="flex items-center gap-2 font-mono text-code-md text-on-surface-variant">
                <span class="text-outline" title="Cursor">
                    {move || if sel_xyz.get().is_some() { "SEL" } else { "CUR" }}
                </span>
                // T-159.22 — `title` (not `aria-label`): these are roleless `<span>`s, where an
                // `aria-label` is ignored by AT and would be a fake a11y name. `title` is a real
                // tooltip AND the CUR gate's DOM handle, matching the `title="Cursor"` idiom above.
                <span title="Cursor X">
                    "X"
                    <span class="ml-1 text-on-surface tabular-nums">{move || axis_val(0)}</span>
                </span>
                <span title="Cursor Y">
                    "Y"
                    <span class="ml-1 text-on-surface tabular-nums">{move || axis_val(1)}</span>
                </span>
                <span title="Cursor Z">
                    "Z"
                    <span class="ml-1 text-on-surface tabular-nums">{move || axis_val(2)}</span>
                </span>
            </div>
            <span class="h-5 w-px bg-white/10"></span>
            <div
                class="flex items-center gap-2 font-mono text-code-md tabular-nums text-on-surface-variant"
                title="Placed slots on map / current selection"
            >
                <span>
                    "OBJ"
                    <span class="ml-1 text-on-surface">{move || obj_count.get()}</span>
                </span>
                <span>
                    "SEL"
                    <span class="ml-1 text-on-surface">{move || sel_count.get()}</span>
                </span>
                <span title="Estimated save payload">
                    "SZ"
                    <span class="ml-1 text-on-surface">
                        {move || {
                            sz_bytes
                                .and_then(|s| s.get())
                                .map_or_else(
                                    || "—".to_string(),
                                    crate::mission_size::format_bytes,
                                )
                        }}
                    </span>
                </span>
                // ── T-670 (STATUS-ZOOM-001) — the metres-per-pixel SCALE readout: the fourth cell of
                // this mono group, beside SZ. Eden prints this number in its status bar and we
                // printed nothing, which also left T-639's zoom-adaptive contour ladder with no
                // on-screen check. It is deliberately the SAME quantity as the T-667 scale bar in
                // the centre slot — both go through `m_per_px(deck_zoom)` off the SAME engine zoom
                // (see `scale_mpp` below, which now feeds the bar too), so the graphic and the
                // number can never disagree. Its own `title` (like SZ's) — the group title above
                // describes OBJ/SEL only.
                <span data-status-scale title="Map scale — metres per screen pixel">
                    "SCL"
                    <span class="ml-1 text-on-surface">
                        {move || {
                            format_m_per_px(
                                scale_mpp.map_or_else(|| m_per_px(-2.0), |s| s.get()),
                            )
                        }}
                    </span>
                </span>
            </div>
            // ── Ruler readout (T-642, Decision 1) — the running total + last-leg readout, beside the
            // OBJ/SEL/SZ telemetry. Renders ONLY when a ruler has at least one leg (`ruler_status` is
            // `Some`), so the bar is unchanged when no measure is placed. `text-primary` marks it as
            // the measuring channel (distinct from the neutral telemetry), matching the on-map line's
            // colour so the label on the map and the summary in the bar read as one tool.
            {move || {
                ruler_status.and_then(|s| s.get()).map(|text| {
                    view! {
                        <span class="h-5 w-px bg-white/10"></span>
                        <span
                            data-status-ruler
                            class="flex items-center whitespace-nowrap font-mono text-code-md text-primary"
                            title="Ruler — running total · last leg"
                        >
                            {text}
                        </span>
                    }
                })
            }}
            // ── Map-furniture slot (T-667, wave 106) — the metric scale bar lives HERE, in the
            // status bar's CLEAR CENTRE SPAN. The wave-105 verifier pinned this bar's left 256 px
            // and right 320 px as OCCLUDED under the docks until T-721; the `flex-1` spacer centres
            // this slot between the CUR/OBJ/SEL/SZ readouts (left) and the HUD/OPEN (right), so the
            // scale bar renders in the clear middle band that both docks miss. (The edge grid
            // references — the other half of the furniture — cannot live in this slot: they anchor
            // to the MAP-PANE edges, so they render from `MapGridRefs`, an overlay mounted once in
            // `mission_editor`.) T-636 reserved this slot EMPTY as a do-not-build-early guard; T-667
            // is the ticket it guarded for, so the slot is now filled and the pin updated to match.
            <span class="h-5 w-px bg-white/10"></span>
            <div
                data-status-furniture
                class="flex min-w-0 flex-1 items-center justify-center gap-2 font-mono text-code-md text-outline"
                title="Scale bar (T-667)"
            >
                // T-670 forwards `scale_mpp` here so the BAR and the numeric SCL cell read one
                // number, not two independent zoom reads that can disagree by a frame.
                <ScaleBar cursor debug_hud scale_mpp />
            </div>
            // ── Debug HUD slot (T-719) — a legitimate VISIBLE home in the right section, before
            // OPEN. Before T-636 the HUD lived at `right-3 bottom-3` on the overlay with no z-index,
            // painted over by DockRight's z-20 column, so it was invisible. Inside the status bar it
            // is on the same surface as the readouts and can never be occluded. Still gated: it only
            // renders when the operator toggled it on (Ctrl+Alt+D → `hud_shown`) AND the sampler has
            // a non-empty string — and the whole bar is already behind `chrome_hidden`, so the T-635
            // gate stack (chrome_hidden AND hud_shown AND non-empty) is preserved.
            {move || {
                let text = debug_hud.map(|h| h.get()).unwrap_or_default();
                let on = hud_shown.map(|s| s.get()).unwrap_or(false);
                (on && !text.is_empty()).then(|| {
                    view! {
                        <div
                            data-status-hud
                            class="pointer-events-none flex items-center font-mono text-[11px] text-success/90"
                        >
                            {text}
                        </div>
                    }
                })
            }}
            // ── Primary-action slot (§Open) — Eden's bottom-right `PLAY SCENARIO` position, on its
            // own surface. Ours is OPEN. The SLOT is what this ticket builds; what the action does is
            // the undecided part of §Open, so the button is inert here (no handler exists in the
            // owned files) but occupies the real Eden slot with the real Eden weight.
            <button
                type="button"
                data-status-open
                class="flex items-center gap-1.5 rounded-md bg-primary/90 px-3 py-1 text-label-md font-medium text-on-primary transition-colors hover:bg-primary"
                title="Open"
            >
                <MaterialIcon name="folder_open" class="block text-base" />
                <span>"OPEN"</span>
            </button>
        </div>
    }
}

/// T-667 — the metric scale bar that mounts in [`StatusBar`]'s `data-status-furniture` slot.
/// Renders the largest round `1/2/5 × 10^n` distance whose bar fits ≤ [`SCALE_MAX_PX`] via
/// [`pick_scale_bar`].
///
/// **Reactivity (no new rAF loop):** historically the render closure subscribed to `cursor` (pan)
/// and the `debug_hud` ~1 Hz heartbeat, then re-read zoom via `world_assets::camera_snapshot`.
/// That path still compiles as the no-prop fallback.
///
/// **T-670 + wave-115 NIT-3 — live path is the shared `scale_mpp` signal.** The editor always
/// passes `scale_mpp` (seeded to `m_per_px(−2) = 4.0`, then rAF-updated change-guarded), so the
/// bar takes the early return and the `camera_snapshot()` branch is currently dead code with an
/// identical numeric outcome to the seed. The heartbeat/`camera_snapshot` path survives only for
/// a caller that omits the prop; none does today.
#[component]
pub fn ScaleBar(
    /// Pan heartbeat — the editor's pointer-move cursor write (drives the pan re-read).
    cursor: RwSignal<Option<(f64, f64, Option<f64>)>>,
    /// ~1 Hz zoom heartbeat — the rAF debug sampler writes this every second regardless of the HUD
    /// toggle, so a wheel-zoom with a still pointer still refreshes the bar within a second. `Option`
    /// (not `#[prop(optional)]`) so [`StatusBar`] can forward its own optional `debug_hud` straight
    /// through.
    debug_hud: Option<RwSignal<String>>,
    /// T-670 — the editor's live metres-per-pixel, written by the rAF sampler only when the
    /// displayed scale changes. When present it REPLACES the camera re-read below: the bar and the
    /// status bar's numeric SCL cell then resolve from the same `f64`, so the graphic and the
    /// number are the same measurement by construction (and the bar now tracks a wheel-zoom on the
    /// next frame instead of waiting up to a second for the ~1 Hz HUD heartbeat). The live editor
    /// mount always supplies this (seeded 4.0); the `camera_snapshot()` arm below is therefore
    /// dead on the only real caller (wave-115 NIT-3). `Option` (not `#[prop(optional)]`) so
    /// [`StatusBar`] can forward its own optional prop straight through, exactly as it does for
    /// `debug_hud`.
    scale_mpp: Option<RwSignal<f64>>,
) -> impl IntoView {
    let spec = move || -> ScaleBarSpec {
        // T-670 — one scale source when the editor supplies it (see the prop doc). Pan does not
        // change scale, so this path needs neither heartbeat.
        if let Some(s) = scale_mpp {
            return pick_scale_bar(s.get());
        }
        // Subscribe to both heartbeats so the closure re-runs on pan (cursor) and on zoom (hud).
        let _ = cursor.get();
        if let Some(h) = debug_hud {
            let _ = h.get();
        }
        // Default deckZoom −2 (the editor's default) when no engine is registered (native, or
        // pre-mount): a sensible bar rather than a panic. `mut` is only touched on wasm.
        #[allow(unused_mut)]
        let mut deck_zoom = -2.0_f64;
        #[cfg(target_arch = "wasm32")]
        {
            if let Some((_, _, z)) = crate::world_assets::camera_snapshot() {
                deck_zoom = z;
            }
        }
        pick_scale_bar(m_per_px(deck_zoom))
    };
    view! {
        // The bar itself: a baseline with two end ticks (an Eden-plain scale rule), width driven by
        // the resolved px, label centred above. `title` carries the exact distance for hover.
        <div
            data-scale-bar
            class="flex select-none flex-col items-center gap-0.5"
            title=move || format!("Map scale — {}", spec().label)
        >
            <span class="leading-none text-outline">{move || spec().label}</span>
            <div
                class="relative border-x border-b border-outline/70"
                style=move || format!("width:{:.1}px;height:5px", spec().width_px)
            ></div>
        </div>
    }
}

/// T-667 — the edge grid-reference overlay. Renders the 3-digit Arma eastings along the MAP PANE's
/// top edge and northings down its left edge — NOT the viewport edges: the pane is the region
/// between the docks (left = `DOCK_LEFT_PX`, right = `viewport − DOCK_RIGHT_PX`, top = `STRIP_TOP_PX`
/// — read by name from `eden_layout`), which is the correct Eden-exceeding geometry (refs frame the
/// MAP, not the window). Labels sit exactly on the drawn 1 km grid lines because
/// [`edge_eastings`]/[`edge_northings`] project each line back through the SAME `OrthoCamera` the
/// GPU grid uses.
///
/// This CANNOT render from the status-bar slot (it anchors to the pane edges, far from the bar), so
/// it is mounted once by a single dispatcher-authorized line in `mission_editor`. It reads the live
/// camera + the DOM viewport size itself, and re-runs off the same `cursor` (pan) + `debug_hud`
/// (~1 Hz zoom) heartbeats as the scale bar — no new rAF loop. `pointer-events-none` throughout so
/// it never eats a map gesture. Native builds render nothing (no engine, no `window`); the geometry
/// is proven by the pure `labels_match_grid_lines` invariant.
#[component]
pub fn MapGridRefs(
    /// Pan heartbeat (pointer-move cursor write).
    cursor: RwSignal<Option<(f64, f64, Option<f64>)>>,
    /// ~1 Hz zoom heartbeat (rAF debug sampler). `Option` so the mount can pass `Some(debug_hud)`.
    debug_hud: Option<RwSignal<String>>,
) -> impl IntoView {
    // (eastings_top, northings_left) as (pos_px, text) pairs for the current camera + viewport.
    let labels = move || -> (Vec<EdgeLabel>, Vec<EdgeLabel>) {
        let _ = cursor.get();
        if let Some(h) = debug_hud {
            let _ = h.get();
        }
        #[cfg(target_arch = "wasm32")]
        {
            use crate::eden_layout::{DOCK_LEFT_PX, DOCK_RIGHT_PX, STRIP_TOP_PX};
            let Some((tx, ty, zoom)) = crate::world_assets::camera_snapshot() else {
                return (Vec::new(), Vec::new());
            };
            let Some(win) = web_sys::window() else {
                return (Vec::new(), Vec::new());
            };
            let vw = win
                .inner_width()
                .ok()
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            let vh = win
                .inner_height()
                .ok()
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            if vw <= 0.0 || vh <= 0.0 {
                return (Vec::new(), Vec::new());
            }
            // The canvas is full-bleed (NOT inset by the chrome — `eden_layout` note), so the camera
            // viewport IS the whole window; build it exactly as `select_tool::frozen_camera` does.
            let cam = crate::select_tool::frozen_camera(vw, vh, tx, ty, zoom);
            let pane_left = DOCK_LEFT_PX;
            let pane_right = vw - DOCK_RIGHT_PX;
            (
                edge_eastings(&cam, pane_left, pane_right, STRIP_TOP_PX),
                edge_northings(&cam, pane_left, STRIP_TOP_PX, vh),
            )
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            (Vec::new(), Vec::new())
        }
    };
    view! {
        // Full-bleed, non-interactive overlay. Each label is absolutely positioned on its grid line.
        <div
            data-grid-refs
            class="pointer-events-none absolute inset-0 z-10 font-mono text-code-md text-primary/80"
        >
            // Eastings — pinned to the pane's TOP edge (just below the top strip), centred on the
            // vertical line's screen X. Keyed by SCREEN POSITION, not text (T-793 / O-2, the T-727
            // fix): a pan moves the pixel ⇒ new key ⇒ a node with the fresh `left:`. Keying on the
            // text would retain the node and freeze its `left:` at the pre-pan x (the O-2 defect).
            <For
                each=move || labels().0
                key=|l| l.key.clone()
                let:l
            >
                <span
                    class="absolute -translate-x-1/2 rounded bg-surface-container-lowest/60 px-1 leading-none"
                    style=move || {
                        format!(
                            "left:{:.1}px;top:{:.1}px",
                            l.pos_px,
                            crate::eden_layout::STRIP_TOP_PX + 2.0,
                        )
                    }
                >
                    {l.text.clone()}
                </span>
            </For>
            // Northings — pinned to the pane's LEFT edge (just right of the left dock), centred on
            // the horizontal line's screen Y. Keyed by SCREEN POSITION, not text — same T-793 / O-2
            // (T-727) fix as the eastings above: the pixel busts the key on every move.
            <For
                each=move || labels().1
                key=|l| l.key.clone()
                let:l
            >
                <span
                    class="absolute -translate-y-1/2 rounded bg-surface-container-lowest/60 px-1 leading-none"
                    style=move || {
                        format!(
                            "left:{:.1}px;top:{:.1}px",
                            crate::eden_layout::DOCK_LEFT_PX + 2.0,
                            l.pos_px,
                        )
                    }
                >
                    {l.text.clone()}
                </span>
            </For>
        </div>
    }
}

/// Back-compat shim for the pre-T-636 single-pill mount. `eden_chrome` re-exports this name (the
/// stable `crate::eden_chrome::*` import surface the T-661 split promised not to break), so it stays
/// a real public component. It is NOT the mount `mission_editor` uses — the split put the tools
/// ([`ModeToolbar`]) and the readouts ([`StatusBar`]) at two independent mount points, each behind
/// its own `chrome_hidden` gate — but keeping the symbol lets the re-export shim compile without
/// churning a file outside this ticket's scope. It composes the two halves so the name still means
/// "the whole bottom belt" for any caller that reaches for it.
#[component]
pub fn BottomToolbelt(
    cursor: RwSignal<Option<(f64, f64, Option<f64>)>>,
    sel_count: RwSignal<usize>,
    obj_count: RwSignal<usize>,
    selected_ids: RwSignal<Vec<String>>,
    /// Forwarded to `StatusBar`. A required (non-optional) param here so the compat shim can hand it
    /// straight through — the live mount in `mission_editor` passes `sz_bytes` too, so this loses no
    /// generality; a caller with no size estimate can build its own `RwSignal::new(None)`.
    sz_bytes: RwSignal<Option<usize>>,
) -> impl IntoView {
    // T-642 — the shim owns a local `tool_mode` (default Select) purely so `ModeToolbar` compiles;
    // the live mount in `mission_editor` shares the real signal with the pointer handlers. A caller
    // reaching for this compat symbol gets a self-contained, if inert, toggle. `ruler_status` is
    // optional on `StatusBar`, so the shim omits it (no ruler wiring on the compat path).
    let tool_mode = RwSignal::new(crate::ruler_tool::EditorTool::Select);
    // T-644 — the shim owns a local `los_mode` (default Ray) purely so `ModeToolbar` compiles on the
    // compat path; the live mount in `mission_editor` shares the real signal with the pointer commit.
    let los_mode = RwSignal::new(crate::los_tool::LosMode::default());
    view! {
        <ModeToolbar tool_mode los_mode />
        <StatusBar cursor sel_count obj_count selected_ids sz_bytes />
    }
}

/// T-636 — the split is a Leptos view whose innards are structural, so (following `eden_dock_right`
/// / `orbat_manager` precedent) it is pinned by SOURCE INSPECTION rather than a mount: a native test
/// cannot render it, but it can fail loudly if the two-mount structure, the reserved T-667 slot, the
/// T-719 HUD slot, or the §Open slot is unpicked.
///
/// **Every needle is assembled at run time.** This test searches the file it lives in, so a needle
/// spelled out contiguously would put itself in the haystack — an absence check could then never
/// pass. Needles are split/reassembled so the file's own prose never satisfies them (this program's
/// signature defect: a check reporting success over an input it never truly examined).
#[cfg(test)]
mod t636_status_bar {
    use crate::arsenal::class_r_scrub::{live_code, live_source};

    /// This module's file, with comments blanked but string literals KEPT — so the Tailwind class
    /// strings and the readout labels survive as structural landmarks for ordering proofs.
    fn src_kept() -> String {
        live_source(include_str!("eden_toolbelt.rs"))
    }

    /// (structure) The single conflated pill is split into TWO components — a tools mount and a
    /// readouts mount — which is what makes them two independent mount points in `mission_editor`.
    #[test]
    fn tools_and_readouts_are_two_separate_components() {
        let src = live_code(include_str!("eden_toolbelt.rs"));
        let mode_fn = format!("pub fn {}", "ModeToolbar(");
        let status_fn = format!("pub fn {}", "StatusBar(");
        assert!(
            src.contains(&mode_fn) && src.contains(&status_fn),
            "T-636: the belt must split into a ModeToolbar AND a StatusBar component"
        );
    }

    /// (no conflation) The tools mount carries ONLY tools — none of the CUR/OBJ/SEL/SZ readout
    /// labels leak into `ModeToolbar`; they all live in `StatusBar`. Proven by slicing each
    /// component body out of the string-kept source and checking where the labels land.
    #[test]
    fn mode_toolbar_holds_no_readouts_and_status_bar_holds_them() {
        let src = src_kept();
        let mode_at = src
            .find(&format!("fn {}", "ModeToolbar("))
            .expect("ModeToolbar present");
        let status_at = src
            .find(&format!("fn {}", "StatusBar("))
            .expect("StatusBar present");
        assert!(
            mode_at < status_at,
            "ModeToolbar must be defined before StatusBar"
        );
        let mode_body = &src[mode_at..status_at];
        let status_at2 = status_at;
        let compat_at = src
            .find(&format!("fn {}", "BottomToolbelt("))
            .expect("compat shim present");
        let status_body = &src[status_at2..compat_at];

        // The three tool controls live in the toolbar (Select active + Ruler/LoS stubs).
        for tool in ["Select", "Ruler", "LoS"] {
            assert!(
                mode_body.contains(tool),
                "ModeToolbar must carry the {tool} tool"
            );
        }
        // The readout labels must NOT be in the toolbar…
        for label in ["\"CUR\"", "\"OBJ\"", "\"SEL\"", "\"SZ\""] {
            // (labels appear as `"OBJ"` etc. in the view; `Cursor` titles are separate.)
            let bare = label.trim_matches('"');
            let quoted = format!("\"{bare}\"");
            assert!(
                !mode_body.contains(&quoted),
                "T-636: readout {bare} must not live in the tools mount (that conflation is the bug)"
            );
            // …and every readout label must be in the status bar.
            assert!(
                status_body.contains(&quoted),
                "T-636: readout {bare} must live in the full-width StatusBar"
            );
        }
        // The status bar spans the viewport (full width), not a centred fixed pill: its surface
        // recipe carries `w-full`, and the component wears that recipe.
        let recipe_at = src
            .find(&format!("const {}", "STATUSBAR"))
            .expect("STATUSBAR recipe present");
        let recipe = &src[recipe_at
            ..src[recipe_at..]
                .find(';')
                .map(|i| recipe_at + i)
                .unwrap_or(src.len())];
        assert!(
            recipe.contains("w-full"),
            "T-636: the STATUSBAR recipe must be full-width (w-full), stretched across the viewport"
        );
        assert!(
            status_body.contains("class=STATUSBAR"),
            "T-636: StatusBar must wear the full-width STATUSBAR recipe"
        );
    }

    /// T-787 — [`STATUSBAR_H_PX`] is the SOURCE OF TRUTH for the bar's top edge, so it must equal
    /// the `h-*` token actually painted in [`STATUSBAR`]. `eden_layout::dock_bottom_px` insets the
    /// docks by exactly this number so `dock.bottom == bar.y`; if someone re-heights the bar (say
    /// `h-9` → `h-10`) without bumping the const, the docks would resume overlapping the bar and the
    /// O-1 click-eating defect would return — this pin fails loudly instead.
    #[test]
    fn statusbar_height_const_tracks_the_h_token() {
        let painted = crate::eden_layout::tw_len_px(super::STATUSBAR, "h-")
            .expect("the STATUSBAR recipe must state an `h-*` height");
        assert!(
            (painted - super::STATUSBAR_H_PX).abs() < f64::EPSILON,
            "T-787: STATUSBAR paints h-{} px but STATUSBAR_H_PX = {} — the dock-bottom inset is \
             derived from the const, so a drift re-opens the O-1 overlap (docks eat bar clicks)",
            painted,
            super::STATUSBAR_H_PX
        );
    }

    /// (T-667) The map-furniture slot is now FILLED with the scale bar. This is the deliberate
    /// update of the wave-105 do-not-build-early guard (T-636's `reserves_an_empty_t667_furniture_slot`,
    /// renamed here because it now pins the opposite state): T-667 IS the ticket that guard was
    /// held for, so the slot's new content is pinned rather than the emptiness silently deleted.
    ///
    /// The slot keeps its `flex-1` spacer (it still owns the bar's middle and pushes the HUD + OPEN
    /// to the right), gains `justify-center` so the bar sits in the CLEAR CENTRE SPAN the wave-105
    /// verifier said clears both docks (left 256 / right 320 occluded until T-721), and now contains
    /// a `ScaleBar` child. The edge grid references are NOT here — they anchor to the map-pane edges
    /// and render from `MapGridRefs` (pinned separately below).
    #[test]
    fn fills_the_t667_furniture_slot_with_the_scale_bar() {
        let src = src_kept();
        let hook = format!("data-status-{}", "furniture");
        let at = src
            .find(&hook)
            .expect("T-667: the map-furniture slot must exist");
        // Still carries the flex spacer that owns the middle, AND centres its content in the clear
        // span between the docks.
        let open_end = at + src[at..].find('>').expect("furniture div opens");
        let open_tag = &src[at..=open_end];
        assert!(
            open_tag.contains("flex-1"),
            "T-667: the furniture slot must keep the flex-1 spacer (it owns the bar's middle)"
        );
        assert!(
            open_tag.contains("justify-center"),
            "T-667: the furniture slot must centre its content (the clear centre span the docks miss)"
        );
        // The slot is now FILLED: its body contains the ScaleBar child (built this ticket), not the
        // empty div the guard used to pin.
        let body = src[open_end + 1..]
            .split_once("</div>")
            .map(|(b, _)| b)
            .unwrap_or("");
        assert!(
            body.contains("ScaleBar"),
            "T-667: the furniture slot must now render the ScaleBar (the guard's do-not-build-early \
             state is deliberately retired — the slot is filled, not empty)"
        );
    }

    /// (T-667) The scale bar and the edge grid references are both real components with the maths the
    /// ticket names. Proven on scrubbed code (strings blanked) so a needle can't hide in a class
    /// string or comment: `ScaleBar` picks the round distance from the zoom, `MapGridRefs` renders
    /// the pane-edge references, and both reuse the CUR/heartbeat channel (cursor + debug_hud) rather
    /// than a new rAF loop.
    #[test]
    fn t667_components_and_reactivity_channel() {
        let code = live_code(include_str!("eden_toolbelt.rs"));
        // Both public components exist.
        assert!(
            code.contains(&format!("pub fn {}", "ScaleBar("))
                && code.contains(&format!("pub fn {}", "MapGridRefs(")),
            "T-667: ScaleBar + MapGridRefs must be real components"
        );
        // Scale bar drives its width off the zoom→m/px picker, not a hardcoded size.
        assert!(
            code.contains("pick_scale_bar(") && code.contains("m_per_px("),
            "T-667: the scale bar must size from pick_scale_bar(m_per_px(zoom))"
        );
        // Grid refs anchor to the MAP-PANE insets read by NAME from eden_layout (not viewport edges),
        // and project lines through the shared camera.
        assert!(
            code.contains("DOCK_LEFT_PX")
                && code.contains("DOCK_RIGHT_PX")
                && code.contains("STRIP_TOP_PX")
                && code.contains("edge_eastings(")
                && code.contains("edge_northings("),
            "T-667: grid refs must anchor to the map-pane insets and use the edge-label geometry"
        );
        // Reactivity reuses the existing channel: the components read the live camera off the
        // registered engine and re-run on the cursor (pan) + debug_hud (~1 Hz zoom) heartbeats.
        // No `request_animation_frame` is introduced in this file.
        assert!(
            code.contains("camera_snapshot()"),
            "T-667: the components must read the live camera via world_assets::camera_snapshot"
        );
        assert!(
            !code.contains("request_animation_frame"),
            "T-667: reuse the CUR/heartbeat channel — do NOT add a new rAF loop in eden_toolbelt"
        );
    }

    /// (T-793 / O-2) The grid-ref `<For>` rows are keyed by SCREEN POSITION, never the label text.
    /// This is the render half of the O-2 fix and cannot be reached by the native property test (the
    /// `<For>` is a Leptos view innard), so it is pinned on scrubbed code: the `MapGridRefs` body must
    /// key on `l.key` and must NOT key on `l.text`. A text key is retained across a pan, so Leptos
    /// reuses the node and freezes its `left:` (the hostile review's half-updated set); the position
    /// key busts on every move. Proven on `live_code` (strings blanked) so the needle is the real
    /// `key=` binding, not a mention in a comment or class string.
    #[test]
    fn grid_ref_for_is_keyed_by_position_not_text() {
        let code = crate::arsenal::class_r_scrub::live_code(include_str!("eden_toolbelt.rs"));
        let body =
            crate::arsenal::class_r_scrub::only_body(&code, &format!("pub fn {}", "MapGridRefs("));
        // The rows key on the position-derived identity…
        assert!(
            body.contains("key=|l| l.key"),
            "T-793: MapGridRefs rows must be keyed by the position key (l.key), not the label text — \
             a text key retains a moved node and freezes its left: (O-2)"
        );
        // …and never on the text (the reverted defect). Both `<For>`s (eastings + northings) count.
        assert!(
            !body.contains("key=|l| l.text"),
            "T-793: a grid-ref `<For>` keyed on l.text is the O-2 defect — Leptos would reuse the \
             node for a retained ref and hold its stale screen x across a pan"
        );
        let key_bindings = body.matches("key=|l| l.key").count();
        assert!(
            key_bindings >= 2,
            "T-793: both the easting and northing `<For>` must key by position (found {key_bindings})"
        );
    }

    /// (F-13, wave-201 addendum) The Eden-style coordinate unit: a real cursor/selection axis reads
    /// `<n> m` (Eden `X 8762.61 m`, frame 170422), the off-map cell stays the bare em-dash, and the
    /// numeric PRECISION is byte-for-byte `fmt_coord` (three decimals, correct to the metre) — the
    /// suffix is presentation, no value is lost, so the CUR readout stays the trusted oracle the
    /// grid-label acceptance test unprojects against.
    #[test]
    fn eden_coord_readout_carries_the_metre_unit() {
        use super::{fmt_coord, fmt_coord_eden};
        // A real value gains ` m` and keeps `fmt_coord`'s exact digits.
        let e = fmt_coord_eden(Some(8762.61));
        assert!(
            e.ends_with(" m"),
            "F-13: a coordinate axis must read Eden's `<n> m`, got `{e}`"
        );
        assert_eq!(
            e.trim_end_matches(" m"),
            fmt_coord(Some(8762.61)),
            "F-13: the number must be EXACTLY fmt_coord's — the ` m` is presentation, not a reformat \
             (no precision loss: the CUR readout stays the metre-accurate oracle)"
        );
        assert!(
            e.contains("8762.610"),
            "F-13: three-decimal precision is preserved under the unit, got `{e}`"
        );
        // The off-map "no value" cell is the bare em-dash — a unit on nothing would be a lie.
        assert_eq!(
            fmt_coord_eden(None),
            fmt_coord(None),
            "F-13: the off-map cell must stay the em-dash, with NO ` m` suffix"
        );
        assert!(
            !fmt_coord_eden(None).contains('m'),
            "F-13: no unit on the absent readout"
        );
    }

    /// (F-13) The StatusBar's axis assembly RENDERS the unit-suffixed formatter, not the bare
    /// `fmt_coord` — proven on the string-kept live source (the addendum's `live_source` pin), so a
    /// future edit that drops the unit back to the un-suffixed readout fails here. The X/Y/Z cells
    /// go through `fmt_coord_eden`.
    #[test]
    fn status_bar_axis_readout_uses_the_eden_unit_formatter() {
        let src = live_source(include_str!("eden_toolbelt.rs"));
        let body =
            crate::arsenal::class_r_scrub::only_body(&src, &format!("pub fn {}", "StatusBar("));
        assert!(
            body.contains("fmt_coord_eden("),
            "F-13: the StatusBar axis readout must render the Eden ` m`-suffixed value via \
             fmt_coord_eden — the coordinate cells carry the unit (Eden `X … m`)"
        );
    }

    /// (§Open) The primary-action slot exists on its own surface at the right end — Eden's
    /// `PLAY SCENARIO` position; ours is OPEN. The slot is built; the button's behaviour is the
    /// undecided part of §Open, so it is inert here.
    #[test]
    fn builds_the_open_primary_action_slot() {
        let src = src_kept();
        let hook = format!("data-status-{}", "open");
        assert!(
            src.contains(&hook),
            "§Open: the primary-action slot (OPEN) must be built at the bar's right end"
        );
        let label = ["OP", "EN"].concat();
        let at = src.find(&hook).expect("open slot present");
        let window = &src[at..src[at..]
            .find("</button>")
            .map(|i| at + i)
            .unwrap_or(src.len())];
        assert!(
            window.contains(&label) && window.contains("folder_open"),
            "§Open: the slot must present an OPEN button (label + folder_open glyph)"
        );
    }

    /// (T-719) The debug HUD gets a legitimate VISIBLE home inside the status bar's right section,
    /// BEFORE the OPEN slot, gated on `hud_shown` (Ctrl+Alt+D) AND a non-empty sampler string. The
    /// `chrome_hidden` half of the gate is the StatusBar mount wrapper (pinned in `mission_editor`).
    #[test]
    fn hud_slot_is_gated_and_sits_before_open() {
        // Gate expression on scrubbed code (strings blanked) so it is the real gate, not a comment.
        let code = live_code(include_str!("eden_toolbelt.rs"));
        assert!(
            code.contains("on && !text.is_empty()"),
            "T-719: the HUD slot must render only when (hud_shown AND non-empty sampler string)"
        );
        // hud_shown / debug_hud are real optional props threaded into StatusBar.
        assert!(
            code.contains("hud_shown") && code.contains("debug_hud"),
            "T-719: StatusBar must accept the HUD toggle + text signals"
        );
        // Ordering: the HUD slot precedes the OPEN slot in the right section.
        let src = src_kept();
        let hud = src
            .find(&format!("data-status-{}", "hud"))
            .expect("HUD slot present");
        let open = src
            .find(&format!("data-status-{}", "open"))
            .expect("OPEN slot present");
        assert!(
            hud < open,
            "T-719: the HUD slot must sit BEFORE the OPEN slot"
        );
    }
}

/// T-642 — source pins for the Ruler button ENABLE and the status-bar ruler READOUT. Both are Leptos
/// view innards (structural), so — like `t636_status_bar` — they are pinned by SOURCE INSPECTION on
/// scrubbed code, not a render. Needles are assembled at run time so the file's own prose never
/// satisfies an absence check.
#[cfg(test)]
mod t642_ruler {
    use crate::arsenal::class_r_scrub::{live_code, live_source};

    /// This file with comments blanked but strings KEPT (class strings + labels survive as landmarks).
    fn src_kept() -> String {
        live_source(include_str!("eden_toolbelt.rs"))
    }

    /// (button enable) THE RULE: the Ruler button must NOT be a disabled stub any more — it drops
    /// `disabled=true` and becomes a real `tool_mode` toggle. Proven by slicing the ModeToolbar body
    /// and checking the Ruler button's window carries an `on:pointerdown` that sets `EditorTool::Ruler`
    /// and NO `disabled=true`.
    ///
    /// T-643 (wave 109) — the LoS button is now ALSO enabled (its wave-108 disabled forward-guard is
    /// flipped): the same honesty rule now permits it because `los_tool` works end-to-end. The LoS
    /// assertion below therefore mirrors the Ruler one — sets `EditorTool::LoS`, NO `disabled=true`.
    #[test]
    fn ruler_button_is_enabled_and_toggles_tool_mode() {
        let src = src_kept();
        let mode_at = src
            .find(&format!("fn {}", "ModeToolbar("))
            .expect("ModeToolbar present");
        // Body from ModeToolbar to the next component (StatusBar-adjacent code follows it here; the
        // TOOL_BASE const sits above, so slice forward to the next `pub fn`).
        let body_end = src[mode_at + 1..]
            .find("pub fn ")
            .map(|i| mode_at + 1 + i)
            .unwrap_or(src.len());
        let body = &src[mode_at..body_end];
        // The Ruler button toggles tool_mode to Ruler on press…
        let ruler_set = format!("tool_mode.set(EditorTool::{})", "Ruler");
        assert!(
            body.contains(&ruler_set),
            "T-642: the Ruler button must set tool_mode = Ruler on pointerdown"
        );
        // …and Select returns to Select.
        assert!(
            body.contains(&format!("tool_mode.set(EditorTool::{})", "Select")),
            "T-642: the Select button must set tool_mode = Select"
        );
        // The Ruler button's own window (its `straighten` glyph → the button close) carries NO
        // `disabled=true` — THE RULE this ticket exists to honour.
        let straighten_at = body.find("straighten").expect("Ruler glyph present");
        // Walk back to the <button that owns this glyph, forward to its glyph — the region between the
        // button open and the icon is where a `disabled` attr would live.
        let btn_open = body[..straighten_at]
            .rfind("<button")
            .expect("Ruler button open tag");
        let ruler_btn_head = &body[btn_open..straighten_at];
        let disabled_true = ["disabled=", "true"].concat();
        assert!(
            !ruler_btn_head.contains(&disabled_true),
            "T-642: the Ruler button must NOT be `disabled=true` — enabling a non-working stub is the \
             lie THE RULE forbids; it is enabled only because ruler_tool works end-to-end"
        );
        // T-643 — the LoS button is NOW enabled too (wave 109): its window sets tool_mode = LoS on
        // pointerdown and carries NO `disabled=true`, exactly like the Ruler check above. The
        // wave-108 "LoS must stay disabled" forward-guard is retired here — the honesty rule is met
        // because `los_tool` works end-to-end.
        assert!(
            body.contains(&format!("tool_mode.set(EditorTool::{})", "LoS")),
            "T-643: the LoS button must set tool_mode = LoS on pointerdown"
        );
        let los_at = body.find("visibility").expect("LoS glyph present");
        let los_open = body[..los_at].rfind("<button").expect("LoS button open");
        assert!(
            !body[los_open..los_at].contains(&disabled_true),
            "T-643: the LoS button must NOT be `disabled=true` — it is enabled only because los_tool \
             works end-to-end (the honesty rule this ticket, like T-642, exists to honour)"
        );
    }

    /// (status readout) The status bar renders the ruler's running-total + last-leg readout
    /// (Decision 1) in the readout section, off a `ruler_status` prop, behind a `Some`-gate so it is
    /// absent when no ruler is placed. Pinned on scrubbed code so the needle is the real prop + slot,
    /// not a comment.
    #[test]
    fn status_bar_renders_the_ruler_readout() {
        let code = live_code(include_str!("eden_toolbelt.rs"));
        // StatusBar accepts the ruler_status signal…
        assert!(
            code.contains("ruler_status"),
            "T-642: StatusBar must accept a ruler_status prop"
        );
        // …and the readout slot exists (data hook) and is Some-gated.
        let src = src_kept();
        assert!(
            src.contains(&format!("data-status-{}", "ruler")),
            "T-642: the status bar must have a ruler readout slot"
        );
        // The slot reads the signal (`.get()`), so it is live, not a static string.
        let hook = format!("data-status-{}", "ruler");
        let at = src.find(&hook).expect("ruler slot present");
        // The gate expression precedes the slot in the same view arm.
        let region_start = src[..at]
            .rfind("ruler_status")
            .expect("ruler_status read before slot");
        assert!(
            src[region_start..at].contains(".get()"),
            "T-642: the ruler readout must read ruler_status.get() (live, not a fixed string)"
        );
    }

    /// (Decision 4 — session-local, NOT doc state) `eden_toolbelt` renders the ruler readout from a
    /// signal only; it must not reach into any document mutation. A light guard that the readout path
    /// carries no doc-write token (the real no-doc-writes proof is `ruler_tool`'s `no_ruler_doc_writes`
    /// pin + the compiler: `ruler_tool` never imports a doc mutator).
    #[test]
    fn readout_is_display_only_no_doc_writes() {
        let code = live_code(include_str!("eden_toolbelt.rs"));
        for banned in ["move_entities", "add_slot", "store.rs", "MissionDocCore"] {
            assert!(
                !code.contains(banned),
                "T-642: the toolbelt ruler readout must be display-only — found `{banned}`"
            );
        }
    }
}

/// T-667 — the pure furniture maths: the round-distance scale picker (table over zooms), the Arma
/// 3-digit grid formatter (incl. the 100 km wrap), and the labels-match-grid-lines invariant. The
/// invariant checks that every edge label lands on a drawn 1 km grid line — the line set is the
/// exact mirror of `map_engine_render::lanes::grid_lines`' loop (`x = 0..width step 1000, inclusive`;
/// that crate is wasm32-only so a native test reconstructs the identical rule, and `GRID_STEP_M` is
/// documented to equal its `GRID_STEP`), and each label is projected through the SAME `OrthoCamera`
/// the GPU grid uses, so a label can never drift off the line it names. Native — no browser/engine.
#[cfg(test)]
mod t667_furniture_math {
    use super::*;

    /// A camera built exactly as `select_tool::frozen_camera` builds it (bounds `[0,0,12800,12800]`),
    /// so the projection the labels use matches the one the editor + GPU grid use.
    fn cam(width: f64, height: f64, tx: f64, ty: f64, zoom: f64) -> OrthoCamera {
        let mut c = OrthoCamera::new(width, height, tx, ty, zoom);
        c.set_bounds(0.0, 0.0, 12_800.0, 12_800.0);
        c
    }

    /// m/px is exactly `2^(−deck_zoom)` (the T-639/T-641 convention this whole ticket rides).
    #[test]
    fn m_per_px_is_two_pow_neg_zoom() {
        assert!((m_per_px(0.0) - 1.0).abs() < 1e-12);
        assert!((m_per_px(-2.0) - 4.0).abs() < 1e-12); // editor default zoom
        assert!((m_per_px(2.0) - 0.25).abs() < 1e-12);
        assert!((m_per_px(-6.0) - 64.0).abs() < 1e-12); // MIN_ZOOM (whole terrain)
        assert!(m_per_px(f64::NAN).is_nan()); // T-756: non-finite → NAN → em-dash readout
    }

    /// The scale-bar distance table across the zoom range. Each row is `(deck_zoom, expected_dist_m,
    /// expected_label)`. The picker takes the largest `1/2/5×10^n` whose bar is ≤ 200 px, so the bar
    /// px = dist / (2^−zoom) is always ≤ 200 and lands in a readable band (~80–200 px — the tightest
    /// a 1-2-5 ladder holds). This is the ticket's required "table over zooms".
    #[test]
    fn scale_bar_distance_table() {
        // (deck_zoom, dist_m, label). Each row: dist = largest 1/2/5×10^n with dist/(2^−z) ≤ 200 px.
        // The bar px each row yields is in the comment and re-verified against the picker below.
        let table: &[(f64, f64, &str)] = &[
            (2.0, 50.0, "50 m"),      // m/px 0.25 → 50 m → 200 px
            (1.0, 100.0, "100 m"),    // m/px 0.5  → 100 m → 200 px
            (0.0, 200.0, "200 m"),    // m/px 1.0  → 200 m → 200 px
            (-1.0, 200.0, "200 m"),   // m/px 2.0  → 200 m → 100 px (500 m = 250 px > cap)
            (-2.0, 500.0, "500 m"),   // m/px 4.0  → 500 m → 125 px (1000 m = 250 px > cap)
            (-3.0, 1000.0, "1 km"),   // m/px 8.0  → 1000 m → 125 px (2000 m = 250 px > cap)
            (-4.0, 2000.0, "2 km"),   // m/px 16.0 → 2000 m → 125 px
            (-5.0, 5000.0, "5 km"),   // m/px 32.0 → 5000 m → 156 px (10 km = 312 px > cap)
            (-6.0, 10000.0, "10 km"), // m/px 64.0 → 10000 m → 156 px
        ];
        for &(z, want_dist, want_label) in table {
            let spec = pick_scale_bar(m_per_px(z));
            // The picker is the source of truth for the exact rung; assert it is a 1-2-5 value…
            let mant = spec.dist_m / 10.0_f64.powf(spec.dist_m.log10().floor());
            let mant_r = (mant * 10.0).round() / 10.0;
            assert!(
                (mant_r - 1.0).abs() < 1e-9
                    || (mant_r - 2.0).abs() < 1e-9
                    || (mant_r - 5.0).abs() < 1e-9,
                "z={z}: {} is not a 1/2/5×10^n distance",
                spec.dist_m
            );
            // …its bar never exceeds the cap…
            assert!(
                spec.width_px <= SCALE_MAX_PX + 1e-9,
                "z={z}: bar {:.1} px exceeds the {SCALE_MAX_PX} px cap",
                spec.width_px
            );
            // …and it is the LARGEST such rung (the next 1-2-5 step up would exceed the cap).
            let next = next_125_up(spec.dist_m);
            assert!(
                next / m_per_px(z) > SCALE_MAX_PX + 1e-9,
                "z={z}: {} m is not the largest fitting rung — {next} m would also fit",
                spec.dist_m
            );
            // The table's own expectation must match the picker (this is the pinned table).
            assert_eq!(
                spec.dist_m, want_dist,
                "z={z}: picker chose {} m, table says {want_dist} m",
                spec.dist_m
            );
            assert_eq!(spec.label, want_label, "z={z}: label mismatch");
        }
    }

    /// The next `1/2/5 × 10^n` value strictly above `d` (1→2→5→10…). Test helper for "largest rung".
    fn next_125_up(d: f64) -> f64 {
        let decade = 10.0_f64.powf(d.log10().floor());
        let mant = (d / decade * 10.0).round() / 10.0;
        if (mant - 1.0).abs() < 1e-9 {
            2.0 * decade
        } else if (mant - 2.0).abs() < 1e-9 {
            5.0 * decade
        } else {
            10.0 * decade // 5 → next decade's 1
        }
    }

    /// FIRE THE RULE (perturb / fail / restore): the round-distance picker genuinely discriminates —
    /// asserting the WRONG rung for a zoom fails, and the right one passes. A picker that returned a
    /// constant (or ignored zoom) would pass the perturbed assertion, so this proves the table above
    /// is load-bearing.
    #[test]
    fn scale_picker_rule_fires() {
        let good = pick_scale_bar(m_per_px(-2.0)); // 500 m @ zoom −2
        assert_eq!(good.dist_m, 500.0, "baseline: zoom −2 must pick 500 m");
        // Perturb: claim it should be 1000 m. That is FALSE (1000/4 = 250 px > 200 cap), so an
        // equality check against the perturbed value must NOT hold — the rule fires.
        let perturbed_expectation = 1000.0;
        assert_ne!(
            good.dist_m, perturbed_expectation,
            "the picker must reject 1000 m at zoom −2 (its bar overflows the cap) — if this were \
             equal the picker would be ignoring zoom"
        );
        // Restore: the true value still holds, and a different zoom yields a different rung (not a
        // constant).
        assert_eq!(pick_scale_bar(m_per_px(-2.0)).dist_m, 500.0);
        assert_ne!(
            pick_scale_bar(m_per_px(-2.0)).dist_m,
            pick_scale_bar(m_per_px(2.0)).dist_m,
            "the picker must vary with zoom (−2 → 500 m vs +2 → 50 m)"
        );
    }

    /// The Arma 3-digit grid formatter: hundreds-of-metres, zero-padded, wrapping every 100 km.
    #[test]
    fn grid_formatter_arma_3digit_with_wrap() {
        assert_eq!(grid_ref_3digit(0.0), "000");
        assert_eq!(grid_ref_3digit(1000.0), "010"); // 1 km line
        assert_eq!(grid_ref_3digit(6400.0), "064"); // the ticket's worked example
        assert_eq!(grid_ref_3digit(12000.0), "120"); // last 1 km line before 12800
        assert_eq!(grid_ref_3digit(99900.0), "999"); // just before the wrap
        assert_eq!(grid_ref_3digit(100_000.0), "000"); // 100 km wraps to 000
        assert_eq!(grid_ref_3digit(106_400.0), "064"); // wrap keeps the format
                                                       // Off-terrain / degenerate guards.
        assert_eq!(grid_ref_3digit(-1.0), "000");
        assert_eq!(grid_ref_3digit(f64::NAN), "000");
        // Sub-100 m rounds DOWN to the hundreds cell (floor, not round).
        assert_eq!(grid_ref_3digit(199.9), "001");
    }

    /// Grid-line enumeration returns exactly the drawn 1 km positions inside a span, inclusive.
    #[test]
    fn grid_lines_in_range_are_the_drawn_positions() {
        assert_eq!(
            grid_lines_in_range(0.0, 3000.0),
            vec![0.0, 1000.0, 2000.0, 3000.0]
        );
        assert_eq!(grid_lines_in_range(1500.0, 3200.0), vec![2000.0, 3000.0]);
        assert_eq!(grid_lines_in_range(3200.0, 1500.0), vec![2000.0, 3000.0]); // order-agnostic
        assert!(grid_lines_in_range(1100.0, 1900.0).is_empty()); // no line between 1000 and 2000
        assert!(grid_lines_in_range(f64::NAN, 5.0).is_empty());
    }

    /// The distinct vertical grid-line X positions the engine DRAWS on Everon (12800). This mirrors
    /// `map_engine_render::lanes::grid_lines` **operation-for-operation**: its loop is `x from 0 to
    /// width, step GRID_STEP (1000), inclusive` (`x <= width`), so the line set is `{0, 1000, …,
    /// 12000}` (12800 is never hit by the step-1000 loop — Deck's behaviour, per that module's own
    /// doc). `lanes` is a wasm32-only dependency of this crate (the GPU render engine), so a native
    /// `cargo test` cannot link `grid_lines()` directly; the set is reconstructed from the identical
    /// rule instead, and `GRID_STEP_M` is documented to equal that module's `GRID_STEP`. The
    /// invariant below then proves every label lands on one of THESE positions.
    fn drawn_vertical_lines() -> Vec<f64> {
        let width = 12_800.0_f64;
        let mut xs = Vec::new();
        let mut k = 0i64;
        while (k as f64) * GRID_STEP_M <= width {
            xs.push(k as f64 * GRID_STEP_M);
            k += 1;
        }
        xs
    }

    /// CORE INVARIANT — every easting label sits on a drawn grid line and reads its correct ref.
    /// For several camera states, each label returned by `edge_eastings` is unprojected back to a
    /// world X, which must be a multiple of `GRID_STEP_M`, must be one the engine actually draws,
    /// and whose `grid_ref_3digit` equals the label text. A label that drifted from its line (the
    /// failure the ticket calls "worse than no label") would land off a multiple and fail here.
    #[test]
    fn labels_match_grid_lines() {
        let drawn = drawn_vertical_lines();
        let is_drawn = |wx: f64| drawn.iter().any(|d| (d - wx).abs() < 1.0);
        // A spread of zooms + targets (incl. the editor default and a zoomed-in centre).
        let cases = [
            (1600.0, 800.0, -2.0), // zoomed out, near NW
            (1237.0, 843.0, 0.0),  // ~1 km/screen, centre-ish
            (900.0, 600.0, 1.5),   // zoomed in
            (1500.0, 900.0, -4.0), // near whole-terrain
        ];
        // Pane insets read by name from eden_layout (the real geometry).
        use crate::eden_layout::{DOCK_LEFT_PX, DOCK_RIGHT_PX, STRIP_TOP_PX};
        for (w, h, z) in cases {
            let mut tx = 6400.0_f64;
            let mut ty = 6400.0_f64;
            // Slew the target across a few positions so lines cross the pane at varied screen X.
            for shift in [-3000.0_f64, 0.0, 4000.0] {
                tx = (6400.0 + shift).clamp(0.0, 12_800.0);
                ty = (6400.0 + shift * 0.5).clamp(0.0, 12_800.0);
                let c = cam(w, h, tx, ty, z);
                let pane_right = w - DOCK_RIGHT_PX;
                let eastings = edge_eastings(&c, DOCK_LEFT_PX, pane_right, STRIP_TOP_PX);
                for lbl in &eastings {
                    // The label's screen X unprojects to a world X on a drawn grid line…
                    let wx = c.unproject_xy(lbl.pos_px, STRIP_TOP_PX)[0];
                    let k = (wx / GRID_STEP_M).round();
                    let on_line = (wx - k * GRID_STEP_M).abs();
                    assert!(
                        on_line < 1.0,
                        "easting label at {:.1}px unprojects to world x {wx:.2} — {on_line:.2} m off \
                         a 1 km line (drift = worse than no label)",
                        lbl.pos_px
                    );
                    assert!(
                        is_drawn(k * GRID_STEP_M),
                        "easting world x {:.0} is not a line the engine draws",
                        k * GRID_STEP_M
                    );
                    // …and its text is the correct Arma ref for that line.
                    assert_eq!(
                        lbl.text,
                        grid_ref_3digit(k * GRID_STEP_M),
                        "easting label text must match its grid line's 3-digit ref"
                    );
                    // …and it lies inside the visible pane span (framing the MAP, not the window).
                    assert!(
                        lbl.pos_px >= DOCK_LEFT_PX - 0.5 && lbl.pos_px <= pane_right + 0.5,
                        "easting label {:.1}px must be inside the map-pane span [{DOCK_LEFT_PX}, {pane_right:.1}]",
                        lbl.pos_px
                    );
                }
                // Northings: same invariant on the Y axis / left edge.
                let northings = edge_northings(&c, DOCK_LEFT_PX, STRIP_TOP_PX, h);
                for lbl in &northings {
                    let wy = c.unproject_xy(DOCK_LEFT_PX, lbl.pos_px)[1];
                    let k = (wy / GRID_STEP_M).round();
                    assert!(
                        (wy - k * GRID_STEP_M).abs() < 1.0,
                        "northing label at {:.1}px unprojects to world y {wy:.2} — off a 1 km line",
                        lbl.pos_px
                    );
                    assert_eq!(
                        lbl.text,
                        grid_ref_3digit(k * GRID_STEP_M),
                        "northing label text must match its grid line's 3-digit ref"
                    );
                    assert!(
                        lbl.pos_px >= STRIP_TOP_PX - 0.5 && lbl.pos_px <= h + 0.5,
                        "northing label {:.1}px must be inside the map-pane vertical span",
                        lbl.pos_px
                    );
                }
            }
            let _ = (tx, ty);
        }
    }

    /// The grid references frame the MAP PANE, not the viewport: an easting whose line falls under
    /// the LEFT dock (screen X < DOCK_LEFT_PX) is dropped, not drawn at the window edge. Proven by
    /// putting the camera so a 1 km line sits at a screen X inside the left-dock band and asserting
    /// no label claims that position.
    #[test]
    fn grid_refs_are_clipped_to_the_map_pane_not_the_viewport() {
        use crate::eden_layout::{DOCK_LEFT_PX, DOCK_RIGHT_PX, STRIP_TOP_PX};
        let (w, h, z) = (1237.0, 843.0, 0.0); // 1 px ≈ 1 m
        let c = cam(w, h, 6400.0, 6400.0, z);
        let pane_right = w - DOCK_RIGHT_PX;
        let eastings = edge_eastings(&c, DOCK_LEFT_PX, pane_right, STRIP_TOP_PX);
        // Every returned label is inside the pane; none in the occluded dock bands.
        for lbl in &eastings {
            assert!(
                lbl.pos_px >= DOCK_LEFT_PX - 0.5,
                "no easting may render under the left dock ({}px): found one at {:.1}px",
                DOCK_LEFT_PX,
                lbl.pos_px
            );
            assert!(
                lbl.pos_px <= pane_right + 0.5,
                "no easting may render under the right dock (past {pane_right:.1}px): found {:.1}px",
                lbl.pos_px
            );
        }
        // Sanity: with 1 px ≈ 1 m and a 12.8 km terrain, the visible pane spans ~660 m, so at least
        // one 1 km line usually shows — but the hard guarantee under test is the clip, not the count.
        let _ = eastings;
    }
}

/// T-793 (`O-2`) — grid reference labels derive from the LIVE camera every frame. The hostile UX
/// review found a HALF-updated set after a 240 m pan: label positions held while the world moved,
/// putting `090` and `100` 70 px apart at 4 m/px where km lines MUST be 250 px apart — two adjacent
/// labels that cannot both be true. The pure geometry ([`edge_eastings`]) was always live; the defect
/// was the render `<For>` keyed on the label TEXT, so Leptos retained a moved label's DOM node and
/// froze its `left:` (the wave-107 T-727 stale-node class, on grids). The fix keys each row on its
/// SCREEN POSITION ([`EdgeLabel::key`] via [`edge_label_key`]), so any pan/zoom that moves a label
/// mints a new key and a freshly-positioned node.
///
/// This module is the ticket's ACCEPTANCE property test, with the CUR unproject as the trusted
/// oracle (the same `OrthoCamera` the status-bar readout uses, verified to the metre by the review).
/// For 5 scripted pans × 3 zoom levels it asserts, for every visible label `k`:
///   * `|screen_x(k·1000) − label_x| ≤ 2 px` (the label sits on its km line, recomputed independently),
///   * adjacent labels are exactly `1000 / m_per_px` px apart, and
///   * under continuous pan the set updates every frame — two mid-pan samples differ correctly, and
///     the `<For>` key of a retained ref changes so its DOM node cannot be reused stale.
#[cfg(test)]
mod t793_grid_labels_live_camera {
    use super::*;
    use crate::eden_layout::{DOCK_LEFT_PX, DOCK_RIGHT_PX, STRIP_TOP_PX};

    /// The editor's real camera build (`select_tool::frozen_camera`): Everon bounds `[0,0,12800,
    /// 12800]`, north-up, no rotation — so the projection the labels use is the one the GPU grid and
    /// the CUR readout use.
    fn cam(width: f64, height: f64, tx: f64, ty: f64, zoom: f64) -> OrthoCamera {
        let mut c = OrthoCamera::new(width, height, tx, ty, zoom);
        c.set_bounds(0.0, 0.0, 12_800.0, 12_800.0);
        c
    }

    /// The km index each easting label names, recovered from its text (`"064"` → 6). The label's
    /// world line is `k·1000` metres; this lets the test recompute `screen_x(k·1000)` independently
    /// of `pos_px` for the `≤ 2 px` acceptance check.
    fn km_index(text: &str) -> i64 {
        // 3-digit hundreds-of-metres → metres → km. `"064"` = 6400 m = km 6.
        text.parse::<i64>().expect("3-digit ref") * 100 / 1000
    }

    /// ACCEPTANCE — 5 pans × 3 zooms: every visible easting sits within 2 px of its km line
    /// (CUR-unproject oracle), and adjacent labels are exactly `1000 / m_per_px` px apart.
    #[test]
    fn labels_track_the_live_camera_across_pans_and_zooms() {
        let (w, h) = (1600.0, 900.0);
        let pane_right = w - DOCK_RIGHT_PX;
        // 3 scripted zoom levels (m/px = 2^-zoom: 4.0, 1.0, 0.5) and 5 scripted pans (world targets).
        let zooms = [-2.0_f64, 0.0, 1.0];
        let pans = [
            (6400.0_f64, 6400.0_f64), // centre
            (6640.0, 6400.0),         // +240 m east — the review's failing pan distance
            (3000.0, 9000.0),         // NW-ish
            (9500.0, 2500.0),         // SE-ish
            (5120.0, 7680.0),         // off-centre
        ];
        for z in zooms {
            let mpp = m_per_px(z);
            let step_px = 1000.0 / mpp; // km-line spacing in screen px at this zoom
            for (tx, ty) in pans {
                let c = cam(w, h, tx, ty, z);
                let eastings = edge_eastings(&c, DOCK_LEFT_PX, pane_right, STRIP_TOP_PX);
                // (a) Each label is on its km line to within 2 px — recompute screen_x(k·1000) via
                //     the SAME camera and compare to the emitted label_x. The oracle: unprojecting
                //     that screen x returns k·1000 (the trusted CUR round-trip).
                for lbl in &eastings {
                    let k = km_index(&lbl.text);
                    let screen_x = c.project([k as f64 * 1000.0, c.target_y(), 0.0])[0];
                    assert!(
                        (screen_x - lbl.pos_px).abs() <= 2.0,
                        "z={z} pan=({tx},{ty}): label {} at {:.2}px is >2px off its km line's \
                         screen x {screen_x:.2}",
                        lbl.text,
                        lbl.pos_px
                    );
                    // CUR-unproject oracle: the label's own x unprojects back to its km line.
                    let wx = c.unproject_xy(lbl.pos_px, STRIP_TOP_PX)[0];
                    assert!(
                        (wx - k as f64 * 1000.0).abs() <= 2.0 * mpp,
                        "z={z}: label {} unprojects to world x {wx:.1}, not its km line {}",
                        lbl.text,
                        k * 1000
                    );
                }
                // (b) Adjacent labels are exactly one km-line spacing apart (1000 / m_per_px px).
                //     Labels come out in ascending world x → ascending screen x (north-up, no rot).
                for pair in eastings.windows(2) {
                    let gap = pair[1].pos_px - pair[0].pos_px;
                    assert!(
                        (gap - step_px).abs() <= 0.5,
                        "z={z} pan=({tx},{ty}): {} and {} are {gap:.1}px apart; km lines must be \
                         {step_px:.1}px ({mpp} m/px). This is the O-2 arithmetic: 70px @ 4m/px is \
                         two labels that cannot both be true.",
                        pair[0].text,
                        pair[1].text
                    );
                }
                // Northings satisfy the same spacing on the Y axis.
                let northings = edge_northings(&c, DOCK_LEFT_PX, STRIP_TOP_PX, h);
                for pair in northings.windows(2) {
                    let gap = (pair[1].pos_px - pair[0].pos_px).abs();
                    assert!(
                        (gap - step_px).abs() <= 0.5,
                        "z={z} pan=({tx},{ty}): northings {} and {} are {gap:.1}px apart; must be \
                         {step_px:.1}px",
                        pair[0].text,
                        pair[1].text
                    );
                }
            }
        }
    }

    /// ACCEPTANCE — under continuous pan the set updates every frame. Two mid-pan samples of the
    /// SAME visible ref must differ correctly: its screen x moves by the pan delta, and — the fix —
    /// its `<For>` key changes, so Leptos cannot reuse the pre-pan node with a frozen `left:` (the
    /// O-2 half-update). A label whose position was cached across the pan would keep both.
    #[test]
    fn continuous_pan_moves_labels_and_busts_the_for_key() {
        let (w, h) = (1600.0, 900.0);
        let pane_right = w - DOCK_RIGHT_PX;
        let z = -2.0; // 4 m/px — the review's zoom
        let mpp = m_per_px(z);
        // Two frames of a continuous eastward pan: the camera target moves +120 m between samples.
        // Target east ⇒ content scrolls WEST, so a fixed grid line's screen x DROPS by 120/mpp =
        // 30 px (`project`: a larger target_x puts the same world x further left on screen).
        let c0 = cam(w, h, 6400.0, 6400.0, z);
        let c1 = cam(w, h, 6520.0, 6400.0, z);
        let a = edge_eastings(&c0, DOCK_LEFT_PX, pane_right, STRIP_TOP_PX);
        let b = edge_eastings(&c1, DOCK_LEFT_PX, pane_right, STRIP_TOP_PX);
        let expected_shift = -120.0 / mpp; // −30 px screen shift (target east ⇒ line x shrinks)
                                           // At least one ref is visible in BOTH frames (a 30 px pan keeps most refs on screen).
        let mut checked = 0;
        for la in &a {
            if let Some(lb) = b.iter().find(|x| x.text == la.text) {
                // Frame-to-frame the label MOVED by the pan delta…
                let moved = lb.pos_px - la.pos_px;
                assert!(
                    (moved - expected_shift).abs() <= 0.5,
                    "ref {} moved {moved:.1}px between frames; a live label must move by the pan \
                     delta {expected_shift:.1}px, not hold position (the O-2 stall)",
                    la.text
                );
                // …and its `<For>` key changed, so the DOM node is re-created at the new x rather
                // than reused with a stale `left:` (the actual O-2 fix — text-keyed nodes did not).
                assert_ne!(
                    la.key, lb.key,
                    "ref {} kept its <For> key across the pan — a text key would, and Leptos would \
                     then reuse the node and FREEZE its left: at the old x (the O-2 half-update)",
                    la.text
                );
                checked += 1;
            }
        }
        assert!(
            checked > 0,
            "the pan must keep at least one ref visible across both frames to compare"
        );
    }

    /// FIRE THE KEYING RULE (perturb / fail / restore): the position-keyed identity genuinely
    /// discriminates. A key that were the TEXT (the reverted defect) would be EQUAL across a pan —
    /// the very thing that let Leptos freeze the node. This asserts the real key is NOT equal to the
    /// text-only key across a move, so the fix is load-bearing, not incidental.
    #[test]
    fn keying_rule_fires() {
        let (w, h) = (1600.0, 900.0);
        let pane_right = w - DOCK_RIGHT_PX;
        let z = -2.0;
        let c0 = cam(w, h, 6400.0, 6400.0, z);
        let c1 = cam(w, h, 6520.0, 6400.0, z); // +120 m pan
        let a = edge_eastings(&c0, DOCK_LEFT_PX, pane_right, STRIP_TOP_PX);
        let b = edge_eastings(&c1, DOCK_LEFT_PX, pane_right, STRIP_TOP_PX);
        let la = a.first().expect("some easting visible");
        let lb = b
            .iter()
            .find(|x| x.text == la.text)
            .expect("same ref visible after a 30px pan");
        // Baseline: the real (position) key differs across the pan — the node busts, position tracks.
        assert_ne!(
            la.key, lb.key,
            "the live key must change when the label moves"
        );
        // Perturb: the DEFECT key is the text, which is IDENTICAL across the pan…
        assert_eq!(
            la.text, lb.text,
            "the ref text is unchanged by a pan — which is exactly why text is an unsafe <For> key"
        );
        // …so a build that keyed on text would compare equal here and reuse the stale node. The fix
        // is that our key does NOT: restore-check that the position component is what breaks the tie.
        assert!(
            la.key.ends_with(&la.text) && lb.key.ends_with(&lb.text),
            "the key still carries the ref for disambiguation…"
        );
        assert_ne!(
            la.key, lb.key,
            "…but the pixel prefix makes it bust on movement (restore: the rule still holds)"
        );
    }
}

/// T-668 — the mode toolbar speaks the one state vocabulary. The CURRENT tool wears TOGGLED_PLATE
/// (plate + dark top border), a live-but-not-current tool wears HOVER_FILL — so the active tool reads
/// like every other toggle and a hovered inactive tool can never be mistaken for the active one.
/// Source-inspection on scrubbed code (the toolbar is a Leptos view); needles assembled at run time.
#[cfg(test)]
mod t668_state_vocabulary {
    use crate::arsenal::class_r_scrub::{live_code, live_source, only_body};

    /// The `cls` closure composes TOOL_BASE with the recipes — TOGGLED_PLATE for the current mode,
    /// HOVER_FILL for the rest. Proven on scrubbed code so the needle is the real `cn` call.
    #[test]
    fn tool_states_consume_the_vocabulary_recipes() {
        let code = live_code(include_str!("eden_toolbelt.rs"));
        let body = only_body(&code, &format!("pub fn {}", "ModeToolbar("));
        assert!(
            body.contains("TOGGLED_PLATE"),
            "the current tool must wear TOGGLED_PLATE (plate + dark top border)"
        );
        assert!(
            body.contains("HOVER_FILL"),
            "a live-but-not-current tool must wear HOVER_FILL"
        );
    }

    /// THE FIX, as an absence: the old ad-hoc tool states are gone — no `bg-primary/20` active tint
    /// spelled inline outside the recipe, and no weaker `hover:bg-white/5` fill. TOOL_BASE carries
    /// only geometry, and the states come from the recipes, so neither ad-hoc token appears as a
    /// class literal in the mode toolbar. Checked on the string-kept source.
    #[test]
    fn no_ad_hoc_tool_state_classes_remain() {
        let src = live_source(include_str!("eden_toolbelt.rs"));
        let mode = only_body(&src, &format!("pub fn {}", "ModeToolbar("));
        // The weaker ad-hoc hover fill the inactive tool used to wear.
        let weak_hover = ["hover:bg-", "white/5"].concat();
        assert!(
            !mode.contains(&weak_hover),
            "T-668: the toolbar's ad-hoc `hover:bg-white/5` must be gone (use HOVER_FILL's bg-white/10)"
        );
        // The active tint must not be spelled as a bare class inside the toolbar — it comes from
        // TOGGLED_PLATE now. (TOGGLED_PLATE's own definition lives in eden_layout, not here.)
        let active_tint = ["bg-", "primary/20"].concat();
        assert!(
            !mode.contains(&active_tint),
            "T-668: the active tool tint must come from TOGGLED_PLATE, not a bare bg-primary/20 here"
        );
    }

    /// Rule (3) — every mode-tool button keeps its `title` (Select / Ruler / LoS all carry one), so a
    /// tool always explains itself. All three ship live today, so none is disabled, but the tooltip is
    /// the same retention pattern rule 3 requires of a disabled control. Checked on the string-kept
    /// source where the title literals survive.
    #[test]
    fn tools_keep_their_tooltips() {
        let src = live_source(include_str!("eden_toolbelt.rs"));
        let mode = only_body(&src, &format!("pub fn {}", "ModeToolbar("));
        for tip in ["Select", "Ruler", "Line of sight"] {
            assert!(
                mode.contains(tip),
                "the {tip} tool button must carry its title (rule 3 tooltip retention)"
            );
        }
    }
}

/// T-670 (`STATUS-ZOOM-001`) — the numeric metres-per-pixel readout. Eden prints this in its status
/// bar; we printed nothing, which also left T-639's zoom-adaptive contour ladder with no on-screen
/// check. Two halves are proven here: the pure formatting (a real value table, plus the
/// reconciliation that the printed number IS the contour ladder's own `m_per_px`), and — by source
/// inspection, since these are Leptos view innards — that the cell is wired into the OBJ/SEL/SZ
/// group and that the T-667 scale bar now resolves from the SAME signal rather than a second,
/// independently-sampled zoom. Needles are assembled at run time so this module's own prose can
/// never satisfy them.
#[cfg(test)]
mod t670_scale_readout {
    use super::{format_m_per_px, m_per_px, pick_scale_bar};
    use crate::arsenal::class_r_scrub::{live_code, live_source, only_body, only_item};
    use map_engine_core::camera::{MAX_ZOOM, MIN_ZOOM};

    /// The readout across the whole zoom clamp, at the real rungs the operator sees. `MIN_ZOOM −6`
    /// is whole-Everon (64 m/px), `−2` the editor default (4 m/px), `0` unity, `MAX_ZOOM 6` the
    /// close-inspection ceiling (0.0156 m/px). Three significant figures throughout — including
    /// below 0.1 m/px, where a fixed 3-decimal format would have dropped to two.
    #[test]
    fn readout_table_across_the_zoom_clamp() {
        let cases = [
            (MIN_ZOOM, "64.0 m/px"),
            (-4.0, "16.0 m/px"),
            (-2.0, "4.00 m/px"),
            (-1.0, "2.00 m/px"),
            (0.0, "1.00 m/px"),
            (2.0, "0.250 m/px"),
            (4.0, "0.0625 m/px"),
            (MAX_ZOOM, "0.0156 m/px"),
        ];
        for (z, want) in cases {
            let got = format_m_per_px(m_per_px(z));
            assert_eq!(got, want, "zoom {z} must read {want}, got {got}");
        }
        // A degenerate scale reads as the same em-dash "no value" the other cells use — never NaN,
        // never `inf`, on the operator's screen.
        for bad in [f64::NAN, f64::INFINITY, 0.0, -1.0] {
            assert!(
                format_m_per_px(bad).starts_with('\u{2014}'),
                "degenerate m/px {bad} must render the em-dash cell, not a raw float"
            );
        }
        // T-756 (MINOR-4): a non-finite *zoom* must also hit the em-dash — `m_per_px` used to
        // fabricate 1.0 and print a confident "1.00 m/px".
        for bad_z in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let got = format_m_per_px(m_per_px(bad_z));
            assert!(
                got.starts_with('\u{2014}'),
                "non-finite zoom {bad_z} must render the em-dash cell, got {got}"
            );
        }
        // T-756 (MINOR-4): band-top carry must not print four significant figures.
        assert_eq!(format_m_per_px(9.996), "10.0 m/px");
        assert_eq!(format_m_per_px(99.96), "100 m/px");
        assert_eq!(format_m_per_px(0.09996), "0.100 m/px");
    }

    /// The readout is MONOTONE in zoom: zooming in never prints a larger metres-per-pixel. A
    /// formatter that rounded into a non-monotone sequence would make the number lie about the
    /// direction of a gesture, which is worse than printing nothing.
    #[test]
    fn readout_never_goes_backwards_as_you_zoom_in() {
        let mut prev = f64::INFINITY;
        let mut z = MIN_ZOOM;
        while z <= MAX_ZOOM {
            let shown: f64 = format_m_per_px(m_per_px(z))
                .trim_end_matches(" m/px")
                .parse()
                .expect("the readout must be a parseable number plus its unit");
            assert!(
                shown <= prev,
                "zoom {z}: printed {shown} m/px after {prev} — the readout must not increase as \
                 you zoom IN"
            );
            prev = shown;
            z += 0.25;
        }
    }

    /// **Reconciliation with T-639 (wave 101 + T-755).** The summary says this readout is the
    /// on-screen check for the zoom-adaptive contour ladder, so it must print the ladder's OWN
    /// scale, not a lookalike. `apps/website/frontend/src/world_assets/dem_vectors.rs`
    /// `push_contours` computes `2.0_f64.powf(-zoom)` and hands it — with nothing in between — to
    /// `contour_interval_for_zoom`; [`m_per_px`] is that same expression (param name `deck_zoom`).
    ///
    /// `contour_interval_for_zoom` itself cannot be CALLED from here — `map-engine-core`'s `world`
    /// feature is a wasm32-only dependency of this crate, so on native it does not exist. So the
    /// identity is pinned three ways that need no such call: (a) OUR conversion's scrubbed body is
    /// `2^(-deck_zoom)`; (b) the ladder's scrubbed body binds that expression and feeds it to the
    /// interval selector with no adjustment line between; (c) numerically, our fn matches `2^(-z)`
    /// across the clamp (and the printed string stays within display precision). Wave-115 MINOR-3:
    /// the old exact-string needles missed an adjustment inserted between the bind and the call,
    /// and an upstream `zoom` re-bind; the contiguous feed + no-rebind checks close those holes.
    #[test]
    fn the_printed_scale_is_the_contour_ladders_own_scale() {
        // (1) OUR conversion — scrubbed body, not a test-local recomputation of the formula alone.
        let ours = live_code(include_str!("eden_toolbelt.rs"));
        let our_mpp = only_body(&ours, &format!("pub fn {}", "m_per_px("));
        assert!(
            our_mpp.contains(&format!("2.0_f64.{}(-deck_zoom)", "powf")),
            "T-670/T-755: m_per_px must be 2^(-deck_zoom) — the contour ladder's screen-scale              convention"
        );

        // (2) THE LADDER's feed — frontend dem_vectors.rs (not crates/). Contiguous bind→call so an
        // adjustment line between them goes RED; no `let zoom` / `zoom =` rebind before the bind so
        // an upstream re-based zoom goes RED.
        let dem = live_code(include_str!("world_assets/dem_vectors.rs"));
        let push = only_body(&dem, &format!("fn {}", "push_contours("));
        let bind = format!("let m_per_px = 2.0_f64.{}(-zoom);", "powf");
        let call = format!("{}(m_per_px)", "contour_interval_for_zoom");
        let bind_at = push
            .find(&bind)
            .expect("T-639/T-670/T-755: push_contours must bind m_per_px = 2^(-zoom)");
        let call_at = push
            .find(&call)
            .expect("T-639/T-670/T-755: push_contours must feed that m_per_px to the ladder");
        assert!(
            call_at > bind_at,
            "T-670/T-755: contour_interval_for_zoom(m_per_px) must follow the 2^(-zoom) bind"
        );
        let between = &push[bind_at + bind.len()..call_at];
        let squeezed: String = between.split_whitespace().collect::<Vec<_>>().join(" ");
        assert_eq!(
            squeezed,
            "let interval =",
            "T-670/T-755: bind and ladder call must be adjacent (`let interval =` only between);              got {squeezed:?}"
        );
        let before = &push[..bind_at];
        assert!(
            !before.contains("let zoom"),
            "T-670/T-755: push_contours must not rebind `zoom` before computing m_per_px"
        );
        let before_sq: String = before.split_whitespace().collect::<Vec<_>>().join(" ");
        assert!(
            !before_sq.contains("zoom ="),
            "T-670/T-755: push_contours must not assign `zoom` before computing m_per_px"
        );

        // (3) Numerically: our fn matches the shared convention across the clamp, and the printed
        // string stays within display precision of that same quantity.
        let mut z = MIN_ZOOM;
        while z <= MAX_ZOOM {
            let shared = 2.0_f64.powf(-z);
            assert!(
                (shared - m_per_px(z)).abs() < 1e-12,
                "zoom {z}: m_per_px must equal the shared 2^(-zoom) convention"
            );
            let shown: f64 = format_m_per_px(m_per_px(z))
                .trim_end_matches(" m/px")
                .parse()
                .expect("parseable readout");
            assert!(
                (shown - shared).abs() <= shared * 0.0051,
                "zoom {z}: the printed {shown} m/px must be the live {shared} m/px to display                  precision"
            );
            z += 0.125;
        }
    }
    /// **Reconciliation with T-667 (wave 106).** One scale, two surfaces: the graphic bar and this
    /// number must be the same measurement. Given the same `m_per_px`, the bar's chosen ground
    /// distance and the printed number are consistent — the bar is `dist_m / m_per_px` px long,
    /// which is exactly what the printed number says it should be.
    #[test]
    fn the_bar_and_the_number_describe_the_same_scale() {
        let mut z = MIN_ZOOM;
        while z <= MAX_ZOOM {
            let mpp = m_per_px(z);
            let spec = pick_scale_bar(mpp);
            let shown: f64 = format_m_per_px(mpp)
                .trim_end_matches(" m/px")
                .parse()
                .expect("parseable readout");
            // Measuring the drawn bar with the printed scale recovers its labelled distance to
            // within the readout's own display precision (≤ 0.5%).
            let measured = spec.width_px * shown;
            assert!(
                (measured - spec.dist_m).abs() <= spec.dist_m * 0.0051,
                "zoom {z}: a {:.1} px bar read at {shown} m/px measures {measured} m, but is \
                 labelled {} m — the two scale surfaces disagree",
                spec.width_px,
                spec.dist_m
            );
            z += 0.125;
        }
    }

    /// (wiring) The cell is a REAL fourth cell of the OBJ/SEL/SZ mono group in `StatusBar` — not a
    /// floating span elsewhere in the bar — it carries a DOM handle and its own tooltip, and it
    /// renders through the pure formatter above rather than an inline `format!`.
    #[test]
    fn the_scl_cell_sits_in_the_objselsz_group() {
        let src = live_source(include_str!("eden_toolbelt.rs"));
        let status = only_body(&src, &format!("pub fn {}", "StatusBar("));
        let hook = format!("data-status-{}", "scale");
        let at = status
            .find(&hook)
            .expect("T-670: the scale readout must carry a DOM handle");
        // It is inside the SAME group div as OBJ/SEL/SZ: the SZ cell precedes it and the group's
        // closing </div> follows it, with no intervening element opening a new group.
        let sz = status
            .find(&["\"S", "Z\""].concat())
            .expect("SZ cell present");
        assert!(
            sz < at,
            "T-670: the scale cell must be the FOURTH cell — after SZ, inside the same group"
        );
        let group_end = status[sz..]
            .find("</div>")
            .map(|i| sz + i)
            .expect("the OBJ/SEL/SZ group closes");
        assert!(
            at < group_end,
            "T-670: the scale cell must close inside the OBJ/SEL/SZ group, not after it"
        );
        // Its own tooltip (the group title covers OBJ/SEL only), and the label the operator reads.
        let cell_end = status[at..]
            .find("</span>")
            .map(|i| at + i)
            .unwrap_or(status.len());
        let cell = &status[at..cell_end];
        assert!(
            cell.contains("title=") && cell.contains(&["S", "CL"].concat()),
            "T-670: the scale cell must carry its own title and the SCL label"
        );
        // The rendered value goes through the pure formatter (proven on scrubbed CODE, so a
        // mention in a comment or a class string cannot satisfy it).
        let code = live_code(include_str!("eden_toolbelt.rs"));
        let status_code = only_body(&code, &format!("pub fn {}", "StatusBar("));
        assert!(
            status_code.contains(&format!("{}(", "format_m_per_px")),
            "T-670: the cell must render through format_m_per_px, not an inline format!"
        );
    }

    /// (single source) The status bar forwards its scale signal INTO the T-667 scale bar, and the
    /// bar resolves from that signal when it has one — so the number and the graphic can never be
    /// two independently-sampled zooms that disagree. The engine-less `camera_snapshot` fallback
    /// survives for native/compat callers.
    #[test]
    fn the_scale_bar_resolves_from_the_same_signal() {
        let code = live_code(include_str!("eden_toolbelt.rs"));
        let status = only_body(&code, &format!("pub fn {}", "StatusBar("));
        assert!(
            status.contains(&format!("{} cursor debug_hud scale_mpp", "<ScaleBar")),
            "T-670: StatusBar must forward scale_mpp into the ScaleBar (one scale, two surfaces)"
        );
        let bar = only_item(&code, &format!("pub fn {}", "ScaleBar("));
        assert!(
            bar.contains("scale_mpp: Option<RwSignal<f64>>"),
            "T-670: ScaleBar must accept the shared scale signal"
        );
        // It PREFERS the signal: the early return off `scale_mpp` precedes the camera re-read.
        let prefer = bar
            .find(&format!("{} = scale_mpp {{", "if let Some(s)"))
            .expect("T-670: ScaleBar must branch on the shared signal");
        let snapshot = bar
            .find(&format!("{}()", "camera_snapshot"))
            .expect("T-667: the camera fallback must survive for engine-less callers");
        assert!(
            prefer < snapshot,
            "T-670: the shared signal must take precedence over a second camera read"
        );
        // Wave 133 F2 / T-756 NIT-3 — comment corrections (seed / camera_snapshot-dead notes).
        // Raw include_str keeps docs that live_code blanks; only_item scopes to ScaleBar so the
        // test module cannot hollow-self-match; needles are fragment-assembled.
        let docs = include_str!("eden_toolbelt.rs");
        let bar_docs = only_item(docs, &format!("pub fn {}", "ScaleBar("));
        let seeded = format!("{}{}", "seeded ", "4.0");
        let cam_dead = format!("{}{}", "dead on the only real ", "caller");
        assert!(
            bar_docs.contains(&seeded),
            "T-756 / wave 133 F2: ScaleBar docs must keep the NIT-3 seed note (4 m/px default)"
        );
        assert!(
            bar_docs.contains(&cam_dead),
            "T-756 / wave 133 F2: ScaleBar docs must keep the NIT-3 camera_snapshot-dead note"
        );
    }
}
