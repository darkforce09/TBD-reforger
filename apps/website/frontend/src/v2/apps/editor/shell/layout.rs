//! T-661 — Eden chrome layout constants, split from `eden_chrome.rs`.
//!
//! The chrome insets (`STRIP_TOP_PX`, `DOCK_LEFT_PX`, `DOCK_RIGHT_PX`, `TOOLBELT_BAND_PX`) are the
//! source the Tailwind utilities in `mission_editor`'s view are written from, and `select_tool` /
//! `mission_editor` read them back to keep pan/select/marquee gates aligned with the panels — so
//! they stay `pub`. The class recipes below are the shared `overlay.ts` ports used by the strip and
//! docks. Pure `const`s, no wasm; the native view shell compiles them too.
//!
//! ## T-638 — the insets became DYNAMIC (dock collapse)
//!
//! Eden collapses each dock to a 24×24 stub in its outer top corner (`E` = left / Entity List,
//! `R` = right / Asset Browser), and the map pane REFLOWS to fill the freed width. The four
//! consts below are still the **expanded** geometry, but the live inset is now read through the
//! four accessors ([`dock_left_px`] / [`dock_right_px`] / [`strip_top_px`] / [`toolbelt_band_px`]),
//! which fold in the session-local collapse + `chrome_hidden` state. The consts stay `pub` and
//! keep their exact values because two readers outside this ticket's owns consume them **by name as
//! bare `f64` values** — the `eden_chrome` re-export shim and `eden_toolbelt`'s T-667 grid-ref
//! overlay — so renaming them to functions would not compile there. The chokepoint readers this
//! ticket DOES own (`select_tool::farthest_empty_px`, `mission_editor`'s palette-drop `on_canvas`
//! gate) moved onto the accessors, so a collapsed dock changes both what counts as on-canvas and the
//! marquee self-check's probe grid — the two things a stale inset would silently break.
//!
//! **Only the docks collapse.** The top strip and the bottom toolbelt band are unchanged by a dock
//! toggle — but `strip_top_px()`/`toolbelt_band_px()` exist as accessors too so the four readers move
//! as one seam and the `chrome_hidden` "full-bleed" rule (below) applies to all four at once.
//!
//! **`chrome_hidden` × collapse are ORTHOGONAL (T-662 × T-638).** `chrome_hidden` (Backspace) hides
//! the whole chrome subtree; while it is active the map is full-bleed, so **all four accessors report
//! 0** ("hidden wins"). Per-dock collapse is a *separate* latch that PERSISTS through a hide/show
//! cycle: unhide and a dock that was collapsed comes back collapsed. The two states never fight
//! because hidden zeroes the inset outright while collapse only chooses stub-vs-full for a *shown*
//! dock.
#![allow(dead_code)]

use std::cell::Cell;

// ── Chrome insets (CSS px) ───────────────────────────────────────────────────────────────────────
// These ARE the source the Tailwind utilities in `mission_editor`'s view are written from, and
// `select_tool::farthest_empty_px` insets its probe grid by them so a "guaranteed-empty" click px
// can never land under a panel that would swallow the pointerdown. Change a class → change the
// EXPANDED const (and vice versa) — they are one contract, verified by the select + marquee gates.
// T-638: the LIVE inset is `dock_left_px()` etc.; these consts are the expanded value the accessors
// fall back to when the dock is shown and open.

/// Top Command Strip height — `h-12` / the docks' `top-12`. Expanded value; live inset is
/// [`strip_top_px`] (unchanged by dock collapse, zeroed only while `chrome_hidden`).
///
/// **T-637 does NOT touch this.** The strip height is a separate contract from the dock widths:
/// T-634 split the strip into two rows that SUM to it ([`ROW_MENUS_PX`] + [`ROW_TOOLS_PX`]), and the
/// `top-12`/`h-12` utilities are written from it. Equalising the docks moves the X insets only.
pub const STRIP_TOP_PX: f64 = 48.0;

/// T-637 — the EQUALISED dock width, in CSS px. Eden is 240/240 in every one of the 75 screenshots;
/// we were 256 left and 320 right, which is what pushed the right dock's trailing tab off the
/// viewport (the T-632 clipping this ticket absorbed — the clipping was a symptom of the width).
///
/// One number, two names: [`DOCK_LEFT_PX`] and [`DOCK_RIGHT_PX`] both resolve to it, because the two
/// readers outside this file's owns (`eden_chrome`'s re-export shim, `eden_toolbelt`'s grid-ref
/// overlay) consume the per-side names as bare `f64`s and a rename would not compile there. Stating
/// the equality as a definition rather than as two coincidentally-equal literals is what makes
/// `the_docks_are_one_equalised_width` a structural check instead of a numeric one.
pub const DOCK_PX: f64 = 240.0;

/// Left dock width — [`DOCK_LEFT_CLASS`] (`w-60`). Expanded value; live inset is [`dock_left_px`]
/// (→ [`STUB_PX`] collapsed, → 0 while `chrome_hidden`).
pub const DOCK_LEFT_PX: f64 = DOCK_PX;
/// Right dock width — [`DOCK_RIGHT_CLASS`] (`w-60`). Expanded value; live inset is [`dock_right_px`]
/// (→ [`STUB_PX`] collapsed, → 0 while `chrome_hidden`).
pub const DOCK_RIGHT_PX: f64 = DOCK_PX;
/// Bottom band reserved for the toolbelt chrome — the region a pointer probe must stay ABOVE to be
/// on the real map, read identically by `select_tool::farthest_empty_px` and `mission_editor`'s
/// palette-drop `on_canvas` gate (the two live readers; a test pins that they agree — now via the
/// [`toolbelt_band_px`] accessor).
///
/// T-636 split the single floating pill into a full-width status bar docked at `inset-x-0 bottom-0`
/// (`h-9` = 36 px) with the mode toolbar floating just above it (`bottom-11` ≈ 44 px + a ~44 px
/// pill → its top edge sits ~88 px up). The band must clear the TALLER of the two — the floating
/// toolbar — so 96 px still holds with a small margin (it was already generous for the old pill).
/// It is a reserved band, not an exact surface height: the wgpu canvas is full-bleed and is NOT
/// inset by it (shrinking the canvas would invalidate every `select_tool` camera probe — see
/// `mission_editor`'s view note), so the canvas is deliberately not a third reader of this const.
pub const TOOLBELT_BAND_PX: f64 = 96.0;

/// T-638 — the collapsed dock stub: a 24×24 square in the panel's outer top corner (Eden, measured
/// across all 75 screenshots: left `x 0..23 y 47..70`, right `x 1896..1919 y 47..70`). Collapsed is
/// neither a rail nor a vanish — the dock becomes exactly this stub, docked at the screen corner,
/// overlaying the map; the freed width reflows into the map pane. Same value drives the inset
/// accessors, the Tailwind stub size (`w-6 h-6`), and the 24×24 chevron hit-box.
pub const STUB_PX: f64 = 24.0;

/// T-787 — the dock wrappers' BOTTOM inset in CSS px: how far above the viewport bottom an expanded
/// dock stops. It equals the status bar's painted height ([`crate::v2::apps::editor::ui::docks::toolbelt::STATUSBAR_H_PX`]),
/// so a dock's bottom edge lands exactly on the bar's top edge (`dock.bottom == bar.y`) instead of
/// running to `bottom-0` and overlapping it.
///
/// **The defect this closes (O-1).** Both docks are transparent `pointer-events` containers that ran
/// `top-12 … bottom-0`, i.e. `y48 → viewportH`, while the status bar (`inset-x-0 bottom-0`, `h-9`)
/// occupies the bottom [`crate::v2::apps::editor::ui::docks::toolbelt::STATUSBAR_H_PX`] px. The dock rectangles therefore
/// covered the bar's full width and `elementFromPoint` at the bar's left/right ends resolved to a
/// DOCK, not the bar — the containers ate clicks aimed at the readouts and the right-end controls.
/// Insetting the wrappers by this much lifts their bottom edge off the bar.
///
/// NOT [`TOOLBELT_BAND_PX`] (96 px): that is the *input-handling* band a pointer probe must clear to
/// count as on-map (it clears the taller floating [`crate::v2::apps::editor::ui::docks::toolbelt::ModeToolbar`] and does not
/// shrink the full-bleed canvas). This is the *painted DOM* inset for the visible bar only — the two
/// are different contracts and subtracting the full band here would leave a 60 px dead strip where a
/// dock covers neither the bar nor the map. The DOM half of this number is the mounts' `bottom-9`.
pub const DOCK_BOTTOM_PX: f64 = crate::v2::apps::editor::ui::docks::toolbelt::STATUSBAR_H_PX;

// ── T-637 — the DOM half of the inset contract ───────────────────────────────────────────────────
//
// **THE SILENT FAILURE THIS CLOSES.** The insets above are input-handling numbers: `select_tool`
// unprojects the pointer by them and `mission_editor` mounts the docks with a Tailwind width class.
// Nothing connected the two. `DOCK_LEFT_PX = 256.0` and `class="… w-64"` agreed only because a human
// remembered that `w-64` is 256 px, and a change to one without the other maps every click inside
// the map pane to the WRONG world position by exactly the difference — a plausible-looking wrongness
// no rendering test catches, because both the panel and the map still draw correctly.
//
// ── MEASURED, not assumed ────────────────────────────────────────────────────────────────────────
// The slice gate does not run Trunk, so it cannot tell you whether a re-layout renders. These
// numbers came from `tailwindcss` run against `style/aegis.css` with the whole `src/**/*.rs` as its
// content, then the real dock markup laid out in a headless Chrome and read back through
// `getBoundingClientRect` and `scrollWidth − clientWidth`, at a 1920×1080 viewport:
//
//   left dock       240.00 wide · 1032 high · horizontal overflow 0   ← T-637: `bottom-0`, top y48
//     header row      223.00 available, 198.63 used (chevron 24 · Layers 57.75 · Locations 84.88 · verb 20)
//     filter row       22.00 high
//     tree region     958.00 high  ← the "~900 px of void", now the tree's
//   right dock      240.00 wide · horizontal overflow 0
//     tab strip       223.00 available, 202.00 used (7×20 tabs + 20 verb + 24 chevron + gaps)
//   tree row idle    16.00 high      tree row SELECTED (with its `border-t`)  16.00 high
//
// Every overflow figure is 0. The pre-ticket header wanted 228 px of a 215 px row and SQUEEZED
// rather than reporting anything (see `eden_dock_left`'s header-budget note).
//
// T-787 changed ONLY the vertical span: the wrappers now end `bottom-9` (= DOCK_BOTTOM_PX = 36 px)
// instead of `bottom-0`, so at 1920×1080 each dock is 996 high (y48 → y1044, the status bar's top)
// and the tree region is 922 high. The WIDTHS above are unchanged — this ticket touched no `w-*`.
//
// So the mount classes live HERE, beside the numbers they must agree with, `mission_editor` renders
// these consts rather than a hand-written literal, and [`tw_width_px`] reads the width back OUT of
// the class string. `t637_dock_geometry` closes the loop: class → px → unprojected world point.

/// T-637 — the LEFT dock wrapper's classes while EXPANDED. The `w-*` token is the DOM half of
/// [`DOCK_LEFT_PX`]; `top-12` is the DOM half of [`STRIP_TOP_PX`]; T-787's `bottom-9` is the DOM half
/// of [`DOCK_BOTTOM_PX`] — it stops the dock at the status bar's top edge instead of `bottom-0`.
pub(crate) const DOCK_LEFT_MOUNT: &str = "absolute bottom-9 left-0 top-12 z-20 w-60";
/// T-637 — the LEFT dock wrapper while COLLAPSED (T-638): no `w-*`, no `bottom-*`, so the wrapper
/// shrinks to the [`STUB_PX`] box the dock renders and the freed strip is click-through to the map.
pub(crate) const DOCK_LEFT_MOUNT_COLLAPSED: &str = "absolute left-0 top-12 z-20";
/// T-637 — the RIGHT dock wrapper while expanded. Same `w-60` as the left: that IS the equalisation.
/// T-787 `bottom-9` (= [`DOCK_BOTTOM_PX`]) matches the left: both docks stop at the status bar's top.
pub(crate) const DOCK_RIGHT_MOUNT: &str = "absolute bottom-9 right-0 top-12 z-20 w-60";
/// T-637 — the RIGHT dock wrapper while collapsed. See [`DOCK_LEFT_MOUNT_COLLAPSED`].
pub(crate) const DOCK_RIGHT_MOUNT_COLLAPSED: &str = "absolute right-0 top-12 z-20";

/// T-637 — the Tailwind v4 spacing scale, in CSS px: `w-60` / `h-4` / `size-6` → `N × 4.0`
/// (`--spacing` is `0.25rem` = 4 px and the theme does not override it). Returns `None` when the
/// class list carries no token with `prefix`, so a caller can tell "absent" from "zero".
///
/// This is the READ-BACK direction of the inset contract: it lets a native test recover the width a
/// mount class will actually produce in the browser and compare it against the `f64` the pointer
/// unprojection insets by. Only the plain numeric scale is supported — an arbitrary-value token
/// (`w-[13px]`) is deliberately NOT parsed, because the whole point is that the chrome geometry
/// stays on the scale the rest of the UI is written in; such a token reads as absent and the pins
/// fail loudly rather than silently accepting an off-scale width.
#[must_use]
pub fn tw_len_px(classes: &str, prefix: &str) -> Option<f64> {
    classes
        .split_whitespace()
        .filter_map(|tok| tok.strip_prefix(prefix))
        .find_map(|n| n.parse::<f64>().ok())
        .map(|n| n * 4.0)
}

/// T-637 — the width a `w-*` token in `classes` resolves to in CSS px. See [`tw_len_px`].
#[must_use]
pub fn tw_width_px(classes: &str) -> Option<f64> {
    tw_len_px(classes, "w-")
}

thread_local! {
    /// T-638 — the left (Entity List) dock's collapse latch. Session-local (no prefs store: the
    /// `world_layer_prefs` seam is out of this ticket's owns — persisting collapse as an editor
    /// preference is left as residue for T-688, per the ticket). Mirrored here from the reactive
    /// `RwSignal` `mission_editor` owns so the wasm hot-path readers + the accessors see one truth
    /// without threading a signal through `select_tool`.
    static DOCK_LEFT_COLLAPSED: Cell<bool> = const { Cell::new(false) };
    /// T-638 — the right (Asset Browser) dock's collapse latch. See [`DOCK_LEFT_COLLAPSED`].
    static DOCK_RIGHT_COLLAPSED: Cell<bool> = const { Cell::new(false) };
    /// T-638 — mirror of `mission_editor`'s T-662 `chrome_hidden`. Hidden ⇒ every inset accessor
    /// reports 0 (the map is full-bleed while the chrome is hidden). Kept separate from the two
    /// collapse latches so hide/show does not clobber the persisted per-dock collapse state.
    static CHROME_HIDDEN: Cell<bool> = const { Cell::new(false) };
}

/// T-638 — set the left dock collapse latch (mirrored from the view's reactive signal).
pub fn set_dock_left_collapsed(v: bool) {
    DOCK_LEFT_COLLAPSED.with(|c| c.set(v));
}
/// T-638 — set the right dock collapse latch.
pub fn set_dock_right_collapsed(v: bool) {
    DOCK_RIGHT_COLLAPSED.with(|c| c.set(v));
}
/// T-638 — mirror `chrome_hidden` (Backspace hide-interface) into the layout seam.
pub fn set_chrome_hidden(v: bool) {
    CHROME_HIDDEN.with(|c| c.set(v));
}

/// T-638 — is the left dock collapsed right now?
#[must_use]
pub fn dock_left_collapsed() -> bool {
    DOCK_LEFT_COLLAPSED.with(Cell::get)
}
/// T-638 — is the right dock collapsed right now?
#[must_use]
pub fn dock_right_collapsed() -> bool {
    DOCK_RIGHT_COLLAPSED.with(Cell::get)
}
/// T-638 — is the whole chrome hidden right now (Backspace)?
#[must_use]
pub fn chrome_hidden() -> bool {
    CHROME_HIDDEN.with(Cell::get)
}

/// T-638 — live LEFT inset (CSS px): 0 while the chrome is hidden (full-bleed), else [`STUB_PX`]
/// collapsed / [`DOCK_LEFT_PX`] expanded. This is the value the owned on-canvas + marquee readers
/// consume, so a collapse both grows the map pane and lets a click over the freed strip reach the map.
#[must_use]
pub fn dock_left_px() -> f64 {
    if chrome_hidden() {
        0.0
    } else if dock_left_collapsed() {
        STUB_PX
    } else {
        DOCK_LEFT_PX
    }
}
/// T-638 — live RIGHT inset (CSS px). See [`dock_left_px`].
#[must_use]
pub fn dock_right_px() -> f64 {
    if chrome_hidden() {
        0.0
    } else if dock_right_collapsed() {
        STUB_PX
    } else {
        DOCK_RIGHT_PX
    }
}
/// T-638 — live TOP inset (CSS px): the strip does not collapse, so this is [`STRIP_TOP_PX`] unless
/// the chrome is hidden (then 0). It exists as an accessor so the four inset readers move as one seam.
#[must_use]
pub fn strip_top_px() -> f64 {
    if chrome_hidden() {
        0.0
    } else {
        STRIP_TOP_PX
    }
}
/// T-638 — live BOTTOM band (CSS px): the toolbelt band does not collapse, so this is
/// [`TOOLBELT_BAND_PX`] unless the chrome is hidden (then 0).
#[must_use]
pub fn toolbelt_band_px() -> f64 {
    if chrome_hidden() {
        0.0
    } else {
        TOOLBELT_BAND_PX
    }
}
/// T-787 — the live BOTTOM inset of a dock WRAPPER (CSS px): the DOM half of the mount classes, not
/// the input band. An expanded, shown dock ends `bottom-9` = [`DOCK_BOTTOM_PX`] above the viewport
/// floor (landing on the status bar's top edge); a collapsed wrapper drops `bottom-*` to shrink to
/// its stub, and a hidden chrome unmounts the wrapper — both report 0, exactly mirroring
/// [`DOCK_LEFT_MOUNT_COLLAPSED`] / the `chrome_hidden` gate. `mission_editor` reads the MOUNT
/// strings, not this accessor; it exists so the geometry test can assert `dock.bottom == bar.y`
/// against the same const the DOM half is pinned to (the T-637 class↔const discipline, on the Y axis).
#[must_use]
pub fn dock_bottom_px() -> f64 {
    if chrome_hidden() {
        0.0
    } else {
        DOCK_BOTTOM_PX
    }
}

// ── T-638 — map-pane centre + camera centre-hold (pure; native-tested) ─────────────────────────────

/// T-638 — the centre of the MAP PANE in screen (CSS) px for a `width × height` viewport, using the
/// LIVE insets. The pane is the chrome-free region `[dock_left_px(), width − dock_right_px()] ×
/// [strip_top_px(), height − toolbelt_band_px()]`; its centre is where the operator's eye sits, so it
/// is the world point we hold across a collapse reflow. Full-bleed canvas: the camera's own viewport
/// is the whole window, so the pane centre is generally NOT the window centre — that offset is the
/// whole reason a collapse makes the map appear to slide.
#[must_use]
pub fn pane_center_px(width: f64, height: f64) -> (f64, f64) {
    let (l, r, t, b) = (
        dock_left_px(),
        dock_right_px(),
        strip_top_px(),
        toolbelt_band_px(),
    );
    ((l + (width - r)) * 0.5, (t + (height - b)) * 0.5)
}

/// T-638 — CENTRE-HOLD decision, as pure camera math.
///
/// **Decision (documented per the ticket's STILL-OPEN item):** on a collapse reflow the camera holds
/// the world point that was under the **map-pane centre** — Eden's behaviour, where the map appears
/// to *slide* into the freed space rather than *jump*. We implement it through the resize path: the
/// engine's `resize` only re-sizes the camera viewport (`width_px`/`height_px`); it never moves the
/// world `target`, which stays projected at the WINDOW centre. So after the insets change we nudge the
/// target by exactly the pane-centre delta.
///
/// Derivation (ortho, top-left px, `flipY:false`, uniform `scale` = 2^zoom, viewport unchanged):
/// `unproject(px,py) = ( target.x + (px − w/2)/scale , target.y − (py − h/2)/scale )`. Requiring the
/// world point at the OLD pane centre `c0` to land at the NEW pane centre `c1` gives
/// `target' = ( target.x + (c0.x − c1.x)/scale , target.y + (c1.y − c0.y)/scale )` — independent of
/// `w`/`h` and of the target itself. Returns the new `(target_x, target_y)`; the caller clamps to
/// bounds via `set_view` (a nudge that would leave the terrain is absorbed by the clamp, matching a
/// pan into a corner). `scale ≤ 0` (impossible for a live camera) is treated as "no move".
#[must_use]
pub fn centre_hold_target(
    target_x: f64,
    target_y: f64,
    scale: f64,
    pane_center_before: (f64, f64),
    pane_center_after: (f64, f64),
) -> (f64, f64) {
    // A non-finite or non-positive scale (impossible for a live camera) → no move; the explicit NaN
    // arm keeps a NaN scale from poisoning the target (and dodges `neg_cmp_op_on_partial_ord`).
    if scale.is_nan() || scale <= 0.0 {
        return (target_x, target_y);
    }
    let (c0x, c0y) = pane_center_before;
    let (c1x, c1y) = pane_center_after;
    (
        target_x + (c0x - c1x) / scale,
        target_y + (c1y - c0y) / scale,
    )
}

// ── Class recipes ────────────────────────────────────────────────────────────────────────────────
// Ported from React `features/mission-creator/layout/overlay.ts`. The `cn(recipe, '…')` call sites
// are pre-merged into literals here (the `mortar.rs` idiom — `ui::cn` is a naive joiner and can't be
// `const`); each merge below is conflict-free, so the concatenation IS what tailwind-merge yields.

/// React `overlayPanel`, verbatim.
const OVERLAY_PANEL: &str = "pointer-events-auto rounded-xl border border-white/10 bg-surface-container-lowest/55 shadow-xl backdrop-blur-xl";
/// React `overlayDocked`, verbatim.
const OVERLAY_DOCKED: &str =
    "pointer-events-auto bg-surface-container-lowest/55 shadow-xl backdrop-blur-xl";

// ── T-637 — the top strip's shell, folded back from `eden_top_strip.rs` ──────────────────────────
//
// T-634 (wave 115) needed a two-row strip shell and a tighter icon recipe. `eden_layout` was another
// slice's `owns` that wave, so it defined `STRIP_ROWS`/`ROW_MENUS`/`ROW_TOOLS`/`TOOL_ICON` LOCALLY in
// `eden_top_strip.rs` and reported the fold-back as residue. This is that fold-back. Two things died
// with it:
//
//   * **`STRIP` (the one-row shell) is GONE.** `eden_top_strip` was its only consumer and it moved to
//     [`STRIP_ROWS`] at T-634, which left `STRIP` referenced by nothing but the test that compared
//     the two. The file carries `#![allow(dead_code)]`, so nothing warned. Its load-bearing claim —
//     "the strip is made of the same glass as the docks it sits above" — did not die with it: it is
//     now checked directly between [`STRIP_ROWS`] and [`DOCK_L`]/[`DOCK_R`], which is what it always
//     meant.
//   * **`TOOL_ICON` is GONE** — see [`BTN_ICON`] below.

/// T-637 (was T-634's `STRIP_ROWS`) — the top strip's shell: [`OVERLAY_DOCKED`]'s glass, the
/// `border-b` edge, and a COLUMN so the menu row and the tool row stack. It states no height of its
/// own beyond `h-full`; the 48 px comes from `mission_editor`'s `h-12`, written from
/// [`STRIP_TOP_PX`].
pub(crate) const STRIP_ROWS: &str = "pointer-events-auto bg-surface-container-lowest/55 shadow-xl backdrop-blur-xl flex h-full flex-col border-b border-white/10";

/// T-637 (was T-634's `ROW_MENUS`) — strip row 1, Eden's `y 0–22`: the menu bar, the editable title
/// and the live slot census. Identity and commands. `h-6` is [`ROW_MENUS_PX`] — this is the FIXED
/// row of the split.
pub(crate) const ROW_MENUS: &str = "flex h-6 shrink-0 items-center gap-2 px-3";

/// T-637 (was T-634's `ROW_TOOLS`) — strip row 2, Eden's `y 22–40`: history/undo/redo, the ORBAT
/// Manager, the environment cluster and the one primary action. `flex-1` is [`ROW_TOOLS_PX`] — the
/// REMAINDER, so the two rows can never drift from [`STRIP_TOP_PX`].
pub(crate) const ROW_TOOLS: &str =
    "flex min-h-0 flex-1 items-center gap-1.5 border-t border-white/10 px-3";

/// T-637 (was T-634's) — the menu row's fixed height (`h-6`). Documentation-with-teeth: the pins
/// check `ROW_MENUS_PX + ROW_TOOLS_PX == STRIP_TOP_PX`, so the split can never grow the strip.
pub(crate) const ROW_MENUS_PX: f64 = 24.0;
/// T-637 (was T-634's) — the tool row's height: the remainder, 48 − 24. Not a class value.
pub(crate) const ROW_TOOLS_PX: f64 = 24.0;

/// `cn(overlayDocked, …)` + the dock's own edge border.
///
/// T-637 — the padding drops `p-3` → `p-2`. At 320 px the right dock could afford 12 px of gutter on
/// each side; at the equalised [`DOCK_PX`] those 24 px are the difference between the tab strip
/// fitting and the trailing tab clipping, and Eden's own gutter is 8.
pub(crate) const DOCK_L: &str = "pointer-events-auto bg-surface-container-lowest/55 shadow-xl backdrop-blur-xl flex h-full flex-col overflow-y-auto border-r border-white/10 p-2";
/// The right dock's shell classes: [`DOCK_L`]'s recipe mirrored, so the edge border sits on the
/// leading side and the two docks read as one pair of panels around the canvas.
pub(crate) const DOCK_R: &str = "pointer-events-auto bg-surface-container-lowest/55 shadow-xl backdrop-blur-xl flex h-full flex-col overflow-y-auto border-l border-white/10 p-2";

/// The shared icon-button recipe (React TopCommandStrip:148), as T-637 rebuilt it.
///
/// **It used to rest at `text-on-surface-variant` with `p-1.5`, and that was a defect, not a style.**
/// A muted rest colour meant a LIVE glyph and a DEAD one looked the same, so dimming carried no
/// information — exactly the "the undo/redo/history glyphs are too dim to find" complaint. And
/// `p-1.5` around a 24 px `text-base` line box is a 36 px control, which cannot sit in a 24 px strip
/// row or a dense dock. T-634 could not fix it here (this file was another slice's owns that wave),
/// so it routed around the defect with a local `TOOL_ICON` copy and left the defect standing for
/// every OTHER caller. T-637 fixes the recipe and deletes the copy.
///
/// A live control now rests at full `text-on-surface`; dimming is reserved for DISABLED, where it
/// means exactly one thing. State comes from the T-668 vocabulary ([`HOVER_FILL`] +
/// [`DISABLED_GLYPH`]) composed at the call site rather than this recipe's old ad-hoc
/// `hover:`/`disabled:` pair, so the chrome speaks one state language.
pub(crate) const BTN_ICON: &str = "shrink-0 rounded p-0.5 text-on-surface";
/// A vertical hairline divider (React `<span className="h-5 w-px bg-white/10" />`).
pub(crate) const DIVIDER: &str = "h-5 w-px bg-white/10";

// ── T-668 — the one state vocabulary (editor_chrome_direction.md §"The state vocabulary") ──────────
//
// Eden reads as ONE product because it uses one state language everywhere; ours read as assembled
// because it used several — the top-strip open menu wore the SAME `bg-white/10` its every neighbour
// wears on hover, so "hovered" and "toggled on" were indistinguishable. These four named recipes are
// that one language, in Aegis's clothes (the desaturated `#adc6ff` primary, our glass surfaces — no
// literal amber; "amber" in the design doc is Eden's, and it maps onto our solid neutral fill). Every
// interactive chrome control consumes one of them instead of an ad-hoc `hover:`/`bg-*`/`opacity-*`
// combo, and the ad-hoc variants are deleted where they contradicted a rule.
//
// The load-bearing property is rule (1) vs rule (2): HOVER is a solid fill, TOGGLED-ON is a lighter
// PLATE + a 1px dark TOP BORDER. They are distinct BY CONSTRUCTION — a hovered control never grows a
// top border and a toggled one never merely fills — so the two can never be confused no matter how
// the palette shifts. That is exactly the confusion `bg-white/10`-as-active created.

/// Rule (1) — **HOVER = solid fill.** The transient pointer-over state for a neutral interactive
/// control (menu-bar buttons, icon buttons, tree rows). A solid fill, never a border — so it can
/// never be mistaken for [`TOGGLED_PLATE`]. This is the Aegis reading of Eden's "orange is hover, not
/// toggled-on": our solid fill is `bg-white/10` (the glass-surface neutral), not amber.
///
/// Carries `transition-colors` so the fill eases in, and lifts the label to `text-on-surface` on
/// hover (the muted→bright idiom the tree rows already used). Compose after a control's base +
/// geometry classes: `cn(&["… base …", HOVER_FILL])`.
pub(crate) const HOVER_FILL: &str = "transition-colors hover:bg-white/10 hover:text-on-surface";

/// Rule (2) — **TOGGLED ON = lighter plate + 1px dark top border.** The persistent "this is the
/// active/selected/open one" state: an open menu, the current dock tab's panel, a selected tree row.
/// The lighter plate is the Aegis primary tint (`bg-primary/20 text-primary`, the established "on"
/// colour across the chrome); the `border-t border-background/60` is the 1px dark top border that
/// makes it distinct from ANY hover fill BY CONSTRUCTION — a hovered control never grows this border.
/// `#0d1322` (`--color-background`) is the dark base, so the border reads as a recessed lip, Eden's
/// toggled-plate cue.
pub(crate) const TOGGLED_PLATE: &str = "bg-primary/20 text-primary border-t border-background/60";

/// Rule (3) — **DISABLED = dimmed glyph, and the tooltip STILL SHOWS.** The dim half: the control
/// keeps its slot, greys out, and does not react to hover. The tooltip half is not a class — it is
/// the **pattern** [`DISABLED_KEEPS_TOOLTIP`] documents: the `title=` stays on the control (or its
/// wrapper) even while `disabled`, so a control that cannot act still explains why. A disabled
/// control that goes silent is strictly worse than one that speaks (verified on Eden's Redo).
///
/// `disabled:hover:bg-transparent` cancels [`HOVER_FILL`]'s fill so a dimmed control does not still
/// light up under the pointer. Compose it AFTER `HOVER_FILL` so the `disabled:` variant wins.
pub(crate) const DISABLED_GLYPH: &str = "disabled:opacity-30 disabled:hover:bg-transparent";

/// Rule (3), the tooltip half, as a documented invariant rather than a class: a control that carries
/// a `title=` (or `aria-label` used as its tooltip) MUST keep it when `disabled`. In Leptos a static
/// `title=` attribute is emitted regardless of the `disabled` prop, so the pattern is simply "do not
/// gate the `title=` on `!disabled`". The chrome files hold that end: `context_menu`'s
/// `every_disabled_row_in_both_takes_has_a_nonempty_title` and `toolbelt`'s
/// `tools_keep_their_tooltips` check each disabled control still carries its `title`. This const
/// exists so the rule has a name the pins and future edits can cite; its value is documentation,
/// never rendered.
pub(crate) const DISABLED_KEEPS_TOOLTIP: &str =
    "title stays on a disabled control (tooltip retention — rule 3)";

/// Convention — **the checkmark gutter is reserved UNCONDITIONALLY in menus.** Eden only allocates it
/// when a menu happens to carry a checked item, so its label indent jumps between menus; that is a
/// bug NOT to copy. Every menu row leads with this fixed-width cell whether or not it shows a check,
/// so labels never shift. `size-4 shrink-0` matches the tree chevron/spacer cell, and a check glyph
/// (or nothing) renders INSIDE it. Prepend it to a menu row's flex children.
pub(crate) const MENU_GUTTER: &str = "flex size-4 shrink-0 items-center justify-center";

#[cfg(test)]
#[path = "tests/layout/band_readers.rs"]
mod t636_band_readers_agree;

#[cfg(test)]
#[path = "tests/layout/dock_collapse.rs"]
mod t638_collapse;

#[cfg(test)]
#[path = "tests/layout/state_vocabulary.rs"]
mod t668_state_vocabulary;

#[cfg(test)]
#[path = "tests/layout/dock_geometry.rs"]
mod t637_dock_geometry;
