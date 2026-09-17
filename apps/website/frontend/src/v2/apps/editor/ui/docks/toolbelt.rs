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
use website_map_engine::camera::ortho::state::OrthoCamera;
use website_map_engine::editing::tools::line_of_sight::capture::LosMode;
#[cfg(target_arch = "wasm32")]
use website_map_engine::editing::tools::selection;

use crate::v2::apps::editor::shell::layout::{HOVER_FILL, TOGGLED_PLATE};
use crate::v2::core::ui::{cn, MaterialIcon};

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

pub use website_map_engine::camera::grid_reference::GRID_STEP_M;

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

/// The grid reference and the line enumeration are the map furniture's, and they live with the
/// camera that draws it. Named here so the pane's edge labels and every other reader of a grid
/// square resolve to one implementation.
pub use website_map_engine::camera::grid_reference::{grid_lines_in_range, grid_ref_3digit};

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
/// viewport bottom; the (much taller) [`crate::v2::apps::editor::shell::layout::TOOLBELT_BAND_PX`] is the *input* band a
/// pointer probe must clear, a separate contract from this bar's painted height.
const STATUSBAR: &str = "pointer-events-auto bg-surface-container-lowest/55 shadow-xl backdrop-blur-xl flex h-9 w-full items-center gap-3 border-t border-white/10 px-3";

/// T-787 — the status bar's rendered HEIGHT in CSS px (`h-9` in [`STATUSBAR`] → 36 px). This is the
/// SOURCE OF TRUTH for how far the bar's top edge sits above the viewport bottom, exported so
/// `eden_layout`'s [`crate::v2::apps::editor::shell::layout::dock_bottom_px`] can inset the docks to STOP at that top
/// edge instead of overlapping it (the O-1 defect: the transparent dock containers ran to
/// `bottom-0` and ate clicks aimed at the readouts + right-end controls). A test below pins this to
/// the `h-*` token in [`STATUSBAR`] so the two can never drift. Distinct from
/// [`crate::v2::apps::editor::shell::layout::TOOLBELT_BAND_PX`], which is the input-handling band (clears the taller
/// floating [`ModeToolbar`]) and deliberately does not shrink the full-bleed canvas.
pub const STATUSBAR_H_PX: f64 = 36.0;

/// T-668 — the tool button's shared GEOMETRY (no state colour). The three states are composed from
/// this base + the one state vocabulary: current mode = [`TOGGLED_PLATE`], a live-but-not-current
/// mode = [`HOVER_FILL`], and a disabled stub would add `crate::v2::apps::editor::shell::layout::DISABLED_GLYPH` (all
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
    tool_mode: RwSignal<website_map_engine::editing::tools::ruler::EditorTool>,
    /// T-644 — the LoS sub-mode (Ray ⇆ Viewshed). Read here to reflect the active sub-mode in the LoS
    /// button's title/label and toggled by a re-click of the LoS button while LoS is already active;
    /// the map pointer commit reads the SAME signal to route a click. Shared with `mission_editor`.
    los_mode: RwSignal<LosMode>,
) -> impl IntoView {
    use website_map_engine::editing::tools::ruler::EditorTool;
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
    // (The native view shell hosts no document, so it always renders CUR.)
    let sel_xyz = Memo::new(move |_| -> Option<(f64, f64, f64)> {
        let ids = selected_ids.get();
        if ids.len() == 1 {
            #[cfg(target_arch = "wasm32")]
            {
                return website_map_engine::editing::hosted_commands::read_attrs(&ids[0])
                    .map(|a| (a.x, a.y, a.z));
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
                                    crate::v2::apps::editor::shell::mission_size::format_bytes,
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
            if let Some((_, _, z)) = website_map_engine::streaming::host::camera_snapshot() {
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
            use crate::v2::apps::editor::shell::layout::{
                DOCK_LEFT_PX, DOCK_RIGHT_PX, STRIP_TOP_PX,
            };
            let Some((tx, ty, zoom)) = website_map_engine::streaming::host::camera_snapshot()
            else {
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
            let cam = selection::frozen_camera(vw, vh, tx, ty, zoom);
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
                            crate::v2::apps::editor::shell::layout::STRIP_TOP_PX + 2.0,
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
                            crate::v2::apps::editor::shell::layout::DOCK_LEFT_PX + 2.0,
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
/// stable `crate::v2::apps::editor::shell::eden_chrome::*` import surface the T-661 split promised not to break), so it stays
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
    let tool_mode = RwSignal::new(website_map_engine::editing::tools::ruler::EditorTool::Select);
    // T-644 — the shim owns a local `los_mode` (default Ray) purely so `ModeToolbar` compiles on the
    // compat path; the live mount in `mission_editor` shares the real signal with the pointer commit.
    let los_mode = RwSignal::new(LosMode::default());
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
#[path = "tests/toolbelt/status_bar.rs"]
mod t636_status_bar;

/// T-642 — source pins for the Ruler button ENABLE and the status-bar ruler READOUT. Both are Leptos
/// view innards (structural), so — like `t636_status_bar` — they are pinned by SOURCE INSPECTION on
/// scrubbed code, not a render. Needles are assembled at run time so the file's own prose never
/// satisfies an absence check.
#[cfg(test)]
#[path = "tests/toolbelt/ruler_controls.rs"]
mod t642_ruler;

/// T-667 — the pure furniture maths: the round-distance scale picker (table over zooms), the Arma
/// 3-digit grid formatter (incl. the 100 km wrap), and the labels-match-grid-lines invariant. The
/// invariant checks that every edge label lands on a drawn 1 km grid line — the line set is the
/// exact mirror of `map_engine_render::lanes::grid_lines`' loop (`x = 0..width step 1000, inclusive`;
/// that crate is wasm32-only so a native test reconstructs the identical rule, and `GRID_STEP_M` is
/// documented to equal its `GRID_STEP`), and each label is projected through the SAME `OrthoCamera`
/// the GPU grid uses, so a label can never drift off the line it names. Native — no browser/engine.
#[cfg(test)]
#[path = "tests/toolbelt/furniture_geometry.rs"]
mod t667_furniture_math;

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
#[path = "tests/toolbelt/live_grid_labels.rs"]
mod t793_grid_labels_live_camera;

/// T-668 — the mode toolbar speaks the one state vocabulary. The CURRENT tool wears TOGGLED_PLATE
/// (plate + dark top border), a live-but-not-current tool wears HOVER_FILL — so the active tool reads
/// like every other toggle and a hovered inactive tool can never be mistaken for the active one.
/// Source-inspection on scrubbed code (the toolbar is a Leptos view); needles assembled at run time.
#[cfg(test)]
#[path = "tests/toolbelt/toolbar_state.rs"]
mod t668_state_vocabulary;

/// T-670 (`STATUS-ZOOM-001`) — the numeric metres-per-pixel readout. Eden prints this in its status
/// bar; we printed nothing, which also left T-639's zoom-adaptive contour ladder with no on-screen
/// check. Two halves are proven here: the pure formatting (a real value table, plus the
/// reconciliation that the printed number IS the contour ladder's own `m_per_px`), and — by source
/// inspection, since these are Leptos view innards — that the cell is wired into the OBJ/SEL/SZ
/// group and that the T-667 scale bar now resolves from the SAME signal rather than a second,
/// independently-sampled zoom. Needles are assembled at run time so this module's own prose can
/// never satisfy them.
#[cfg(test)]
#[path = "tests/toolbelt/scale_readout.rs"]
mod t670_scale_readout;
