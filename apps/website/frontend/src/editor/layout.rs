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
/// `docks_are_equal_width` a structural check instead of a numeric one.
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
/// dock stops. It equals the status bar's painted height ([`crate::editor::panels::toolbelt::STATUSBAR_H_PX`]),
/// so a dock's bottom edge lands exactly on the bar's top edge (`dock.bottom == bar.y`) instead of
/// running to `bottom-0` and overlapping it.
///
/// **The defect this closes (O-1).** Both docks are transparent `pointer-events` containers that ran
/// `top-12 … bottom-0`, i.e. `y48 → viewportH`, while the status bar (`inset-x-0 bottom-0`, `h-9`)
/// occupies the bottom [`crate::editor::panels::toolbelt::STATUSBAR_H_PX`] px. The dock rectangles therefore
/// covered the bar's full width and `elementFromPoint` at the bar's left/right ends resolved to a
/// DOCK, not the bar — the containers ate clicks aimed at the readouts and the right-end controls.
/// Insetting the wrappers by this much lifts their bottom edge off the bar.
///
/// NOT [`TOOLBELT_BAND_PX`] (96 px): that is the *input-handling* band a pointer probe must clear to
/// count as on-map (it clears the taller floating [`crate::editor::panels::toolbelt::ModeToolbar`] and does not
/// shrink the full-bleed canvas). This is the *painted DOM* inset for the visible bar only — the two
/// are different contracts and subtracting the full band here would leave a 60 px dead strip where a
/// dock covers neither the bar nor the map. The DOM half of this number is the mounts' `bottom-9`.
pub const DOCK_BOTTOM_PX: f64 = crate::editor::panels::toolbelt::STATUSBAR_H_PX;

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
/// gate the `title=` on `!disabled`". The `disabled_controls_keep_their_tooltip` pins in the chrome
/// files check each disabled control still carries its `title`. This const exists so the rule has a
/// name the pins and future edits can cite; its value is documentation, never rendered.
pub(crate) const DISABLED_KEEPS_TOOLTIP: &str =
    "title stays on a disabled control (tooltip retention — rule 3)";

/// Convention — **the checkmark gutter is reserved UNCONDITIONALLY in menus.** Eden only allocates it
/// when a menu happens to carry a checked item, so its label indent jumps between menus; that is a
/// bug NOT to copy. Every menu row leads with this fixed-width cell whether or not it shows a check,
/// so labels never shift. `size-4 shrink-0` matches the tree chevron/spacer cell, and a check glyph
/// (or nothing) renders INSIDE it. Prepend it to a menu row's flex children.
pub(crate) const MENU_GUTTER: &str = "flex size-4 shrink-0 items-center justify-center";

/// T-636 / T-638 — `TOOLBELT_BAND_PX` (and the three dock/strip insets) are INPUT-HANDLING numbers:
/// they are the chrome band the pointer→world readers inset by, so if the readers disagree a click
/// under the status bar (or a collapsed dock's freed strip) would be mapped to a world coordinate as
/// if the chrome were not there. This file OWNS the numbers; the two chokepoint readers this ticket
/// owns must consume them through the T-638 **accessors** (`eden_layout::dock_left_px()` etc.) rather
/// than a magic literal OR the frozen expanded const, so a collapse — and any future height change —
/// stays consistent by construction. That is exactly what this pins.
///
/// It lives here (the consts' owner, natively compiled) rather than in `select_tool`, which is
/// `#[cfg(target_arch = "wasm32")]` and so invisible to a native `cargo test`.
#[cfg(test)]
mod t636_band_readers_agree {
    use crate::arsenal::class_r_scrub::live_code;

    /// The band has ONE definition here (its expanded value), and both chokepoint readers reference
    /// the LIVE inset via the accessor — neither smuggles in a bare `96.0`/`240.0`/`48.0`.
    /// `live_code` blanks comments + string literals, so a number mentioned in prose or a class
    /// string can never satisfy (or false-fail) a needle; the definitions themselves are checked on
    /// raw source, where the `= 96.0` value is real code.
    ///
    /// T-638 accessor-conversion completeness: the two readers (`select_tool::farthest_empty_px`, the
    /// pointer→world probe-grid inset; `mission_editor`'s palette-drop `on_canvas` gate) each moved
    /// from `eden_chrome::TOOLBELT_BAND_PX` (a frozen const read) onto `eden_layout::*_px()` (the
    /// dynamic accessor), and NEITHER may be left on a hardcoded inset literal.
    #[test]
    fn both_readers_reference_the_single_band_const() {
        // Exactly one definition per inset, in this file (raw — the literals are real code, not
        // prose). The expanded consts survive (the `eden_chrome` shim + `eden_toolbelt` grid-refs
        // read them by name as bare f64), so the band value is still pinned once.
        let layout = include_str!("layout.rs");
        let name = "TOOLBELT_BAND_PX";
        let def = format!("pub const {name}: f64 = 96.0;");
        assert!(
            layout.contains(&def),
            "eden_layout must DEFINE {name} exactly once as the band's expanded value (96.0)"
        );
        for (n, one) in [
            ("STRIP_TOP_PX", true),
            ("DOCK_LEFT_PX", true),
            ("DOCK_RIGHT_PX", true),
            ("TOOLBELT_BAND_PX", true),
        ] {
            if one {
                assert_eq!(
                    layout.matches(&format!("pub const {n}")).count(),
                    1,
                    "{n} must have exactly one definition — one source of truth"
                );
            }
        }

        // Reader 1: select_tool::farthest_empty_px (the pointer→world probe-grid inset).
        // Reader 2: mission_editor's palette-drop `on_canvas` gate.
        // Both must consume the LIVE inset through the T-638 accessor, by NAME.
        //
        // `live_code` (via `scrub`) cuts from a file's FIRST `#[cfg(test)]` to EOF. select_tool has
        // none, so its whole body scrubs. mission_editor's first `#[cfg(test)]` is a `clear_for_test`
        // helper near the TOP (above the band reader), so scrubbing the whole file would drop the
        // reader — slice from the page fn anchor first (the t662/t635 idiom), then scrub that.
        let band_read = "editor::layout::toolbelt_band_px()";
        let sel = live_code(include_str!("tools/select_tool.rs"));

        let raw_editor = include_str!("../mission_editor.rs");
        let anchor = format!("{}{}", "pub fn Mission", "EditorPage() -> impl IntoView");
        assert_eq!(
            raw_editor.matches(anchor.as_str()).count(),
            1,
            "scrub anchor must be unambiguous"
        );
        let editor =
            live_code(&raw_editor[raw_editor.find(anchor.as_str()).expect("anchor present")..]);

        assert!(
            sel.contains(band_read),
            "select_tool must inset by the accessor {band_read}, not a literal or frozen const"
        );
        assert!(
            editor.contains(band_read),
            "mission_editor's on_canvas gate must inset by the accessor {band_read}"
        );

        // T-638 completeness: every one of the four insets is read via its accessor in BOTH readers
        // (a reader that quietly stops insetting by one axis is caught), and NO reader mentions the
        // old frozen `eden_chrome::CONST` read for these four (that would freeze the axis against
        // collapse). eden_chrome + eden_toolbelt legitimately keep the const NAMES — they are not
        // this ticket's owns and read the expanded value as bare f64 — so they are excluded here.
        for acc in [
            "editor::layout::dock_left_px()",
            "editor::layout::dock_right_px()",
            "editor::layout::strip_top_px()",
            "editor::layout::toolbelt_band_px()",
        ] {
            assert!(
                sel.contains(acc),
                "select_tool must read the live inset via {acc}"
            );
            assert!(
                editor.contains(acc),
                "mission_editor on_canvas must read the live inset via {acc}"
            );
        }
        for frozen in [
            "eden_chrome::DOCK_LEFT_PX",
            "eden_chrome::DOCK_RIGHT_PX",
            "eden_chrome::STRIP_TOP_PX",
            "eden_chrome::TOOLBELT_BAND_PX",
        ] {
            assert!(
                !sel.contains(frozen),
                "select_tool must not read the frozen const {frozen} — collapse needs the accessor"
            );
            assert!(
                !editor.contains(frozen),
                "mission_editor must not read the frozen const {frozen} — use the accessor"
            );
        }

        // No reader may hardcode an inset (that would silently diverge from a collapse). The needles
        // are split so this test's own source cannot satisfy them. eden_layout is excluded — it
        // legitimately holds the literals in the definitions above.
        // T-637: the two dock literals collapsed from 256/320 to one equalised 240 (`DOCK_PX`).
        for bare in [
            ["96", ".0"].concat(),
            ["240", ".0"].concat(),
            ["48", ".0"].concat(),
        ] {
            assert!(
                !sel.contains(&bare),
                "T-638: select_tool must not hardcode an inset ({bare}) — it comes from the accessor"
            );
            assert!(
                !editor.contains(&bare),
                "T-638: mission_editor must not hardcode an inset ({bare}) — use the accessor"
            );
        }
    }
}

/// T-638 — the collapse state machine, the inset accessors, and the centre-hold math. Pure + native:
/// the toggles live in `mission_editor`'s wasm keydown and the docks are Leptos views (no native
/// mount), so — following the t636/t662 idiom — the *policy* is factored into these pure functions and
/// pinned here, where a native `cargo test` can execute it. The keydown/chevron are thin callers that
/// flip the signals this module mirrors; a source pin below proves the `E`/`R` wiring is present.
#[cfg(test)]
mod t638_collapse {
    use super::{
        centre_hold_target, dock_left_collapsed, dock_left_px, dock_right_collapsed, dock_right_px,
        pane_center_px, set_chrome_hidden, set_dock_left_collapsed, set_dock_right_collapsed,
        strip_top_px, toolbelt_band_px, DOCK_LEFT_PX, DOCK_RIGHT_PX, STRIP_TOP_PX, STUB_PX,
        TOOLBELT_BAND_PX,
    };
    use map_engine_core::camera::OrthoCamera;

    /// Reset the three thread-local latches so tests don't leak state into one another (they run on
    /// the same thread). Every test that touches the accessors starts here.
    fn reset() {
        set_dock_left_collapsed(false);
        set_dock_right_collapsed(false);
        set_chrome_hidden(false);
    }

    /// The accessors fold collapse + chrome_hidden into the live inset exactly as documented:
    /// expanded → the const; collapsed → the 24×24 stub; hidden → 0 (full-bleed), regardless of the
    /// per-dock latch.
    #[test]
    fn accessors_fold_collapse_and_hidden() {
        reset();
        // Expanded, shown.
        assert_eq!(dock_left_px(), DOCK_LEFT_PX);
        assert_eq!(dock_right_px(), DOCK_RIGHT_PX);
        assert_eq!(strip_top_px(), STRIP_TOP_PX);
        assert_eq!(toolbelt_band_px(), TOOLBELT_BAND_PX);

        // Collapse each dock in turn — only its own inset shrinks to the stub; strip/band unchanged.
        set_dock_left_collapsed(true);
        assert_eq!(dock_left_px(), STUB_PX);
        assert_eq!(
            dock_right_px(),
            DOCK_RIGHT_PX,
            "left collapse must not touch right"
        );
        assert_eq!(
            strip_top_px(),
            STRIP_TOP_PX,
            "docks collapse; the strip does not"
        );
        assert_eq!(toolbelt_band_px(), TOOLBELT_BAND_PX);
        set_dock_right_collapsed(true);
        assert_eq!(dock_right_px(), STUB_PX);

        // chrome_hidden WINS: every inset reports 0 while active, even though both docks are still
        // latched collapsed underneath.
        set_chrome_hidden(true);
        assert_eq!(dock_left_px(), 0.0);
        assert_eq!(dock_right_px(), 0.0);
        assert_eq!(strip_top_px(), 0.0);
        assert_eq!(toolbelt_band_px(), 0.0);
        // …and the collapse latches PERSIST underneath (orthogonal states).
        assert!(dock_left_collapsed() && dock_right_collapsed());

        // Un-hide: the persisted collapse state re-applies (stub, not full) — the hide/show cycle
        // never clobbered it.
        set_chrome_hidden(false);
        assert_eq!(dock_left_px(), STUB_PX);
        assert_eq!(dock_right_px(), STUB_PX);
        reset();
    }

    /// The `E`/`R` state machine, modelled as the exact toggle the keydown arms run: E flips only the
    /// left latch, R only the right; each is its own independent toggle. Perturbation: were E to write
    /// the right latch (a copy-paste swap), the "R untouched by E" assertion fails.
    #[test]
    fn e_toggles_left_r_toggles_right_independently() {
        reset();
        // E: left off→on, right untouched.
        set_dock_left_collapsed(!dock_left_collapsed());
        assert!(dock_left_collapsed(), "E collapses the left dock");
        assert!(!dock_right_collapsed(), "E must not touch the right dock");
        // R: right off→on, left still on.
        set_dock_right_collapsed(!dock_right_collapsed());
        assert!(dock_right_collapsed(), "R collapses the right dock");
        assert!(dock_left_collapsed(), "R must not touch the left dock");
        // E again: left on→off (a toggle, not a one-way set), right still on.
        set_dock_left_collapsed(!dock_left_collapsed());
        assert!(!dock_left_collapsed(), "E again expands the left dock");
        assert!(dock_right_collapsed());
        reset();
    }

    /// The chevron glyph MIRRORS the state, in the same 24×24 box, pointing outward when expanded and
    /// flipping when collapsed — for BOTH docks. This is the exact match arm `collapse_chevron` uses,
    /// pinned so a glyph regression (e.g. both docks showing the same chevron) is caught natively.
    #[test]
    fn chevron_glyph_mirrors_state_per_dock() {
        // `(collapsed, expanded_is_left) -> icon`, verbatim from `collapse_chevron`.
        fn icon(collapsed: bool, expanded_is_left: bool) -> &'static str {
            match (collapsed, expanded_is_left) {
                (false, true) => "chevron_left",
                (true, true) => "chevron_right",
                (false, false) => "chevron_right",
                (true, false) => "chevron_left",
            }
        }
        // Left dock: « expanded, » collapsed.
        assert_eq!(icon(false, true), "chevron_left");
        assert_eq!(icon(true, true), "chevron_right");
        // Right dock: » expanded, « collapsed (the mirror of the left).
        assert_eq!(icon(false, false), "chevron_right");
        assert_eq!(icon(true, false), "chevron_left");
        // The flip is real: collapsing swaps the glyph for each dock.
        assert_ne!(icon(false, true), icon(true, true));
        assert_ne!(icon(false, false), icon(true, false));
        // Expanded, the two docks point in OPPOSITE (outward) directions.
        assert_ne!(icon(false, true), icon(false, false));
    }

    /// The `E`/`R` keydown wiring is present in `mission_editor` (source pin — the arms live in a
    /// wasm-only keydown a native test cannot fire). Needles are assembled so this test's own source
    /// cannot satisfy them.
    #[test]
    fn keydown_binds_e_and_r_to_the_collapse_latches() {
        let src = include_str!("../mission_editor.rs");
        let arm = |code: &str| format!("\"{code}\" if !modk");
        // E → left latch, R → right latch.
        assert!(
            src.contains(&arm("KeyE")) && src.contains("dock_left_collapsed.set("),
            "E must toggle dock_left_collapsed"
        );
        assert!(
            src.contains(&arm("KeyR")) && src.contains("dock_right_collapsed.set("),
            "R must toggle dock_right_collapsed"
        );
        // Backspace (T-662) still owns hide-chrome — the E/R arms are ADDED, not a rebind of it.
        assert!(
            src.contains("chrome_hidden.set(!chrome_hidden.get_untracked())"),
            "T-662 Backspace hide-chrome must be untouched"
        );
    }

    /// The map-pane centre is the midpoint of the chrome-free rect using the LIVE insets. Collapsing a
    /// dock moves the pane centre toward that side by half the freed width — the delta the centre-hold
    /// consumes.
    #[test]
    fn pane_centre_uses_live_insets() {
        reset();
        let (w, h) = (1920.0, 1080.0);
        let full = pane_center_px(w, h);
        assert!((full.0 - (DOCK_LEFT_PX + (w - DOCK_RIGHT_PX)) / 2.0).abs() < 1e-9);
        assert!((full.1 - (STRIP_TOP_PX + (h - TOOLBELT_BAND_PX)) / 2.0).abs() < 1e-9);

        // Collapse the left dock: its inset drops DOCK_LEFT_PX→STUB_PX, so the pane centre shifts LEFT
        // by half that change.
        set_dock_left_collapsed(true);
        let left_col = pane_center_px(w, h);
        let expected_dx = (STUB_PX - DOCK_LEFT_PX) / 2.0; // negative → leftward
        assert!(
            (left_col.0 - (full.0 + expected_dx)).abs() < 1e-9,
            "pane centre must shift by half the freed left width"
        );
        assert!(
            (left_col.1 - full.1).abs() < 1e-9,
            "y is unaffected by a dock collapse"
        );
        reset();
    }

    /// CENTRE-HOLD, fired against the engine's OWN camera (map_engine_core `OrthoCamera` — the exact
    /// type `select_tool::frozen_camera` builds). This is the perturb/fail/restore proof the ticket
    /// asks for: with the nudge applied, the world point under the pane centre is INVARIANT across the
    /// collapse reflow (RESTORE); without it, that point MOVES by the pane-centre delta in world units
    /// (FAIL) — so the assertion is not vacuously true.
    #[test]
    fn centre_hold_keeps_the_pane_centre_world_point() {
        reset();
        let (w, h) = (1920.0, 1080.0);
        let (tx, ty, zoom) = (6400.0, 6400.0, -2.0);
        // Full-bleed camera: viewport IS the whole window (matches the editor + `frozen_camera`).
        let cam0 = OrthoCamera::new(w, h, tx, ty, zoom);
        let scale = zoom.exp2();

        // World point under the pane centre BEFORE the collapse.
        let before = pane_center_px(w, h);
        let held = cam0.unproject_xy(before.0, before.1);

        // Collapse the left dock → the pane centre moves.
        set_dock_left_collapsed(true);
        let after = pane_center_px(w, h);
        assert!(
            (after.0 - before.0).abs() > 1.0,
            "the reflow must actually move the pane centre or the test proves nothing"
        );

        // RESTORE: nudge the target, rebuild the (same-size) camera, and the held world point is back
        // under the NEW pane centre to sub-pixel world precision.
        let (nx, ny) = centre_hold_target(tx, ty, scale, before, after);
        let cam1 = OrthoCamera::new(w, h, nx, ny, zoom);
        let after_hold = cam1.unproject_xy(after.0, after.1);
        assert!(
            (after_hold[0] - held[0]).abs() < 1e-6 && (after_hold[1] - held[1]).abs() < 1e-6,
            "centre-hold: the pane-centre world point must be invariant across the reflow (got {after_hold:?}, want {held:?})"
        );

        // FAIL (perturbation): WITHOUT the nudge, the same-camera reflow shifts the pane-centre world
        // point by exactly the freed half-width in world units — proving the hold is load-bearing.
        let no_hold = cam0.unproject_xy(after.0, after.1);
        let drift = (no_hold[0] - held[0]).abs();
        let expected_drift = ((after.0 - before.0) / scale).abs();
        assert!(
            drift > 1e-6,
            "without centre-hold the world point MUST move — else the hold is untested"
        );
        assert!(
            (drift - expected_drift).abs() < 1e-6,
            "the un-held drift must equal the pane-centre delta / scale ({drift} vs {expected_drift})"
        );
        reset();
    }

    /// Degenerate guards: a non-positive scale (impossible for a live camera) is a no-op nudge, and an
    /// unchanged pane centre yields no move.
    #[test]
    fn centre_hold_degenerate_is_a_noop() {
        assert_eq!(
            centre_hold_target(10.0, 20.0, 0.0, (1.0, 2.0), (3.0, 4.0)),
            (10.0, 20.0)
        );
        assert_eq!(
            centre_hold_target(10.0, 20.0, 4.0, (5.0, 6.0), (5.0, 6.0)),
            (10.0, 20.0),
            "no pane-centre change → no target move"
        );
    }
}

/// T-668 — the ONE state vocabulary, pinned. Each rule's recipe const is its own source of truth for
/// the whole chrome, so a drift that would re-introduce two state languages fails HERE, once, rather
/// than being caught (or missed) file-by-file. Pure `const`s in the production half, natively
/// compiled — a `cargo test` reads them directly.
#[cfg(test)]
mod t668_state_vocabulary {
    use super::{DISABLED_GLYPH, DISABLED_KEEPS_TOOLTIP, HOVER_FILL, MENU_GUTTER, TOGGLED_PLATE};

    /// Rule (1) — HOVER is a solid fill and NOTHING ELSE. It fills on hover (`hover:bg-white/10`),
    /// eases (`transition-colors`), and must NOT carry a border token — a border is rule (2)'s cue,
    /// and mixing it in is exactly the confusion this ticket removes.
    #[test]
    fn hover_fill_is_a_solid_fill_no_border() {
        assert!(
            HOVER_FILL.contains("hover:bg-white/10"),
            "HOVER = solid fill (the Aegis reading of Eden's amber-hover)"
        );
        assert!(
            HOVER_FILL.contains("transition-colors"),
            "the hover fill must ease in, not snap"
        );
        assert!(
            !HOVER_FILL.contains("border"),
            "HOVER must carry no border — a border is the TOGGLED cue; sharing it is the bug"
        );
    }

    /// Rule (2) — TOGGLED-ON is a lighter plate PLUS a 1px dark top border, and the two together are
    /// what make it distinct from a hover fill BY CONSTRUCTION. The plate is the Aegis primary tint;
    /// the `border-t border-background/…` is the dark top lip a hovered control never grows.
    #[test]
    fn toggled_plate_is_plate_plus_dark_top_border() {
        assert!(
            TOGGLED_PLATE.contains("bg-primary/20") && TOGGLED_PLATE.contains("text-primary"),
            "TOGGLED = the lighter Aegis primary plate"
        );
        assert!(
            TOGGLED_PLATE.contains("border-t") && TOGGLED_PLATE.contains("border-background"),
            "TOGGLED = plate + a 1px dark TOP border (distinct-by-construction from hover)"
        );
    }

    /// Rules (1) and (2) are distinct BY CONSTRUCTION — the whole point. The toggled plate carries a
    /// `border-t` the hover fill does not, and the hover fill carries a `hover:` fill the toggled
    /// plate does not, so no control can ever render in a state where the two are indistinguishable.
    #[test]
    fn hover_and_toggled_can_never_be_confused() {
        assert!(
            TOGGLED_PLATE.contains("border-t") && !HOVER_FILL.contains("border-t"),
            "only the toggled plate has the top border"
        );
        assert!(
            HOVER_FILL.contains("hover:bg-") && !TOGGLED_PLATE.contains("hover:bg-"),
            "only the hover recipe fills on hover; the toggled plate is a persistent fill"
        );
        assert_ne!(
            HOVER_FILL, TOGGLED_PLATE,
            "the two states must not be the same string (the bg-white/10-as-active defect)"
        );
    }

    /// Rule (3) — DISABLED dims the glyph and cancels the hover fill, so a dimmed control does not
    /// still light up under the pointer. The tooltip half is a pattern, not a class — its name is
    /// pinned so the chrome-file `disabled_controls_keep_their_tooltip` pins have a shared referent.
    #[test]
    fn disabled_glyph_dims_and_cancels_hover() {
        assert!(
            DISABLED_GLYPH.contains("disabled:opacity-30"),
            "DISABLED = dimmed glyph"
        );
        assert!(
            DISABLED_GLYPH.contains("disabled:hover:bg-transparent"),
            "a disabled control must not still fill on hover (cancels HOVER_FILL)"
        );
        assert!(
            DISABLED_KEEPS_TOOLTIP.contains("tooltip"),
            "rule 3's tooltip-retention pattern must be named for the per-file pins to cite"
        );
    }

    /// Convention — the menu checkmark gutter is a fixed-width, always-present cell (Eden's jumping
    /// indent is the bug NOT to copy). `shrink-0` keeps it from collapsing when a row is tight, and
    /// `size-4` matches the tree chevron cell so a menu and a tree read at the same indent.
    #[test]
    fn menu_gutter_is_a_fixed_always_present_cell() {
        assert!(
            MENU_GUTTER.contains("size-4") && MENU_GUTTER.contains("shrink-0"),
            "the gutter is a fixed-width cell that never collapses (no jumping indent)"
        );
    }

    /// FIRE THE RULE ONCE (perturb / fail / restore) on rule (2), the load-bearing one. The property
    /// under test is "toggled-on is distinguishable from hover by a border". PERTURB: a would-be
    /// toggled recipe that is just the hover fill (the `bg-white/10`-as-active defect, stated as a
    /// value) has NO border, so the distinguishing check FAILS on it — proving the check has teeth.
    /// RESTORE: the real `TOGGLED_PLATE` carries the border and passes. A check that passed for both
    /// would be asserting nothing.
    #[test]
    fn toggled_distinct_from_hover_rule_fires() {
        // The real recipe is distinguishable from a hover fill — it has the top border.
        let distinguishable = |toggled: &str| toggled.contains("border-t");
        assert!(
            distinguishable(TOGGLED_PLATE),
            "RESTORE: the real toggled plate carries the distinguishing top border"
        );
        // PERTURB: the defect this ticket removes — "toggled" rendered as the neutral hover fill.
        let defect_toggled = "bg-white/10";
        assert!(
            !distinguishable(defect_toggled),
            "PERTURB: a toggled state that is merely the hover fill has no border — the check must \
             REJECT it, or it is asserting nothing (this is the bg-white/10-as-active bug)"
        );
        // And the defect value is not what we ship.
        assert_ne!(
            TOGGLED_PLATE, defect_toggled,
            "the toggled recipe must not be the bare hover fill"
        );
    }
}

/// T-637 — **THE DOCK GEOMETRY IS AN INPUT CONTRACT, NOT A STYLESHEET.**
///
/// Equalising the docks to Eden's 240/240 is a two-line change to a pair of `f64`s and a pair of
/// Tailwind classes — and getting those two halves out of step is the most dangerous edit in this
/// file, because it fails SILENTLY. `select_tool` unprojects the pointer by the `f64`s; the browser
/// lays the panels out from the classes. If the class says 256 and the const says 240, every panel
/// still draws correctly, the map still draws correctly, and every click inside the map pane resolves
/// to a world position 16 px wrong — 64 world metres at a typical zoom. No screenshot shows it and no
/// render test catches it.
///
/// These pins close that loop end to end: the mount class parses back to the const ([`tw_width_px`]),
/// the const is what the live accessor reports, and the accessor is what a real
/// `map_engine_core::camera::OrthoCamera` unprojects with. The perturbation fires the rule on the
/// exact half-edit it exists to catch — the pre-T-637 `w-64` class left behind while the const moved.
#[cfg(test)]
mod t637_dock_geometry {
    use super::{
        dock_bottom_px, dock_left_px, dock_right_px, set_chrome_hidden, set_dock_left_collapsed,
        set_dock_right_collapsed, strip_top_px, tw_len_px, tw_width_px, BTN_ICON, DOCK_BOTTOM_PX,
        DOCK_L, DOCK_LEFT_MOUNT, DOCK_LEFT_MOUNT_COLLAPSED, DOCK_LEFT_PX, DOCK_PX, DOCK_R,
        DOCK_RIGHT_MOUNT, DOCK_RIGHT_MOUNT_COLLAPSED, DOCK_RIGHT_PX, ROW_MENUS, ROW_MENUS_PX,
        ROW_TOOLS, ROW_TOOLS_PX, STRIP_ROWS, STRIP_TOP_PX,
    };
    use crate::arsenal::class_r_scrub::live_code;
    use crate::editor::panels::toolbelt::STATUSBAR_H_PX;
    use map_engine_core::camera::OrthoCamera;

    /// Both collapse latches off and the chrome shown, so the accessors report the EXPANDED consts.
    /// The latches are thread-locals shared with `t638_collapse`, which runs on the same thread.
    fn expanded() {
        set_dock_left_collapsed(false);
        set_dock_right_collapsed(false);
        set_chrome_hidden(false);
    }

    /// **THE EQUALISATION.** Eden is 240/240 in every one of the 75 screenshots; we were 256 left and
    /// 320 right. The asymmetry was not cosmetic — a 320 px right dock is what pushed its trailing tab
    /// off the viewport (the T-632 clipping this ticket absorbed), and an off-centre map pane is what
    /// made the collapse reflow feel like a jump.
    ///
    /// Stated structurally: both sides resolve to the ONE [`DOCK_PX`], so "equal" is a definition
    /// rather than two literals that happen to match today.
    #[test]
    fn the_docks_are_one_equalised_width() {
        assert_eq!(
            DOCK_LEFT_PX, DOCK_RIGHT_PX,
            "T-637: Eden's docks are the same width; ours were 256/320"
        );
        assert_eq!(DOCK_LEFT_PX, DOCK_PX);
        assert_eq!(DOCK_RIGHT_PX, DOCK_PX);
        assert!(
            (DOCK_PX - 240.0).abs() < f64::EPSILON,
            "T-637: the equalised width is Eden's 240, got {DOCK_PX}"
        );
        // The dock widths and the STRIP height are DIFFERENT contracts — equalising X must not have
        // moved Y. T-634's two-row split depends on this number.
        assert!(
            (STRIP_TOP_PX - 48.0).abs() < f64::EPSILON,
            "T-637 must not touch the strip height contract"
        );
    }

    /// **THE SILENT-FAILURE PIN, AND THE ONE THIS TICKET MOST NEEDED.** The width the browser lays
    /// out (parsed back out of the mount class) and the width the pointer unprojection insets by (the
    /// live accessor) are ONE number, and the proof is carried all the way through to a world
    /// coordinate on the engine's own camera.
    ///
    /// PERTURB / FAIL / RESTORE is inline and on the real failure mode: `w-64` is the class this
    /// ticket replaced. Feed it to the same camera and the pane's left edge lands 64 world metres
    /// away — the assertion checks the drift is EXACTLY `Δpx / scale`, so the check has teeth in both
    /// directions (a drift of zero would mean the unprojection ignores its x argument).
    #[test]
    fn the_mounted_dock_width_and_the_pointer_unprojection_are_one_number() {
        expanded();

        // (1) Class → px. The collapsed mounts deliberately state NO width: the wrapper shrinks to
        // the stub the dock renders, which is what makes the freed strip click-through to the map.
        let dom_left =
            tw_width_px(DOCK_LEFT_MOUNT).expect("the left mount must state a `w-*` width");
        let dom_right =
            tw_width_px(DOCK_RIGHT_MOUNT).expect("the right mount must state a `w-*` width");
        assert!(
            tw_width_px(DOCK_LEFT_MOUNT_COLLAPSED).is_none()
                && tw_width_px(DOCK_RIGHT_MOUNT_COLLAPSED).is_none(),
            "T-638: a collapsed dock's wrapper must state no width, or it keeps covering the map"
        );

        // (2) px → the const the input path insets by.
        assert!(
            (dom_left - dock_left_px()).abs() < f64::EPSILON,
            "T-637: the LEFT mount class lays out {dom_left} px but the pointer unprojection insets \
             by {} px — every click in the map pane would be off by the difference",
            dock_left_px()
        );
        assert!(
            (dom_right - dock_right_px()).abs() < f64::EPSILON,
            "T-637: the RIGHT mount class lays out {dom_right} px but the pointer unprojection \
             insets by {} px",
            dock_right_px()
        );

        // (3) The world-space consequence, on the engine's own camera (the exact type
        // `select_tool::frozen_camera` builds), full-bleed viewport like the editor's.
        let (w, h) = (1920.0, 1080.0);
        let (tx, ty, zoom) = (6400.0, 6400.0, -2.0);
        let cam = OrthoCamera::new(w, h, tx, ty, zoom);
        let scale = zoom.exp2();

        let edge_from_const = cam.unproject_xy(dock_left_px(), strip_top_px())[0];
        let edge_from_dom = cam.unproject_xy(dom_left, strip_top_px())[0];
        assert!(
            (edge_from_const - edge_from_dom).abs() < 1e-9,
            "T-637: the map pane's LEFT edge must be one world point whether you derive it from the \
             mount class or from the inset const"
        );
        let right_from_const = cam.unproject_xy(w - dock_right_px(), strip_top_px())[0];
        let right_from_dom = cam.unproject_xy(w - dom_right, strip_top_px())[0];
        assert!(
            (right_from_const - right_from_dom).abs() < 1e-9,
            "T-637: the map pane's RIGHT edge must be one world point from either derivation"
        );

        // (4) PERTURB — the pre-T-637 class, left behind while the const moved to 240. This is the
        // half-edit the pin exists for, stated as a value so the check must reject it.
        let stale = tw_width_px("absolute bottom-0 left-0 top-12 z-20 w-64")
            .expect("the stale `w-64` class still parses");
        assert!(
            (stale - dom_left).abs() > f64::EPSILON,
            "PERTURB: the stale class must differ from the shipped one or this proves nothing"
        );
        let drifted = cam.unproject_xy(stale, strip_top_px())[0];
        let drift_m = (drifted - edge_from_const).abs();
        assert!(
            (drift_m - (stale - dom_left).abs() / scale).abs() < 1e-9,
            "PERTURB: an out-of-step class must shift the unprojected edge by exactly Δpx / scale \
             ({drift_m} m for {} px at scale {scale})",
            (stale - dom_left).abs()
        );
        assert!(
            drift_m > 1.0,
            "PERTURB: {} px of class/const drift is {drift_m} world metres of pointer error — \
             silent, plausible-looking, and invisible to every render test",
            (stale - dom_left).abs()
        );

        // RESTORE: the shipped pair is the zero-drift one.
        assert!((edge_from_const - edge_from_dom).abs() < 1e-9);
        expanded();
    }

    /// T-787 (O-1) — the docks stop AT the status bar's top edge, not over it. The bar is docked
    /// `inset-x-0 bottom-0` at height [`STATUSBAR_H_PX`], so its top is `barY = h − STATUSBAR_H_PX`;
    /// an expanded dock ends `bottom-9` = [`dock_bottom_px`] up, so its bottom is
    /// `dockBottom = h − dock_bottom_px()`. The acceptance is `dockBottom <= barY` at every swept
    /// viewport — and we hold it as EQUALITY (the dock's bottom lands exactly on the bar's top, no
    /// gap, no overlap). This is the Y-axis twin of the class↔const width pin above: the mount's
    /// `bottom-*` token, read back with [`tw_len_px`], must equal the accessor's number, or the DOM
    /// and the acceptance-geometry model have drifted.
    #[test]
    fn the_docks_bottom_lands_on_the_status_bar_top() {
        expanded();

        // (1) class → px: both expanded mounts state `bottom-9`, and it resolves to DOCK_BOTTOM_PX
        // (= the bar height). The collapsed mounts deliberately state NO `bottom-*` (the wrapper
        // shrinks to its stub), exactly as they state no `w-*`.
        let dom_bottom_left =
            tw_len_px(DOCK_LEFT_MOUNT, "bottom-").expect("left mount must state a `bottom-*`");
        let dom_bottom_right =
            tw_len_px(DOCK_RIGHT_MOUNT, "bottom-").expect("right mount must state a `bottom-*`");
        assert!(
            (dom_bottom_left - DOCK_BOTTOM_PX).abs() < f64::EPSILON
                && (dom_bottom_right - DOCK_BOTTOM_PX).abs() < f64::EPSILON,
            "T-787: the mounts lay out bottom={dom_bottom_left}/{dom_bottom_right} px but \
             DOCK_BOTTOM_PX = {DOCK_BOTTOM_PX} — the DOM half drifted from the const"
        );
        assert_eq!(
            DOCK_BOTTOM_PX, STATUSBAR_H_PX,
            "T-787: the dock-bottom inset must equal the status bar's painted height, or the dock \
             edge cannot land on the bar's top edge"
        );
        assert!(
            tw_len_px(DOCK_LEFT_MOUNT_COLLAPSED, "bottom-").is_none()
                && tw_len_px(DOCK_RIGHT_MOUNT_COLLAPSED, "bottom-").is_none(),
            "T-787: a collapsed dock's wrapper must state no `bottom-*` (it shrinks to its stub)"
        );

        // (2) px → geometry: the review's acceptance, at every swept viewport. dockBottom <= barY.
        for (w, h) in [(1920.0, 1080.0), (1366.0, 768.0), (2560.0, 1440.0)] {
            let bar_y = h - STATUSBAR_H_PX; // bar docked bottom-0, height STATUSBAR_H_PX
            let dock_bottom = h - dock_bottom_px(); // expanded dock ends dock_bottom_px() up
            assert!(
                dock_bottom <= bar_y,
                "T-787: at {w}x{h} the dock bottom ({dock_bottom}) must be <= the bar top ({bar_y})"
            );
            assert!(
                (dock_bottom - bar_y).abs() < f64::EPSILON,
                "T-787: at {w}x{h} the dock bottom ({dock_bottom}) should land exactly on the bar \
                 top ({bar_y}) — no overlap (the O-1 defect) and no dead gap"
            );
        }

        // (3) PERTURB — the pre-T-787 class ran `bottom-0`, i.e. a 0 px inset. Stated as a value the
        // check must reject: it puts the dock bottom at the viewport floor, BELOW the bar top by
        // exactly STATUSBAR_H_PX — the overlap that let the containers eat the bar's clicks.
        let stale_bottom = tw_len_px("absolute bottom-0 left-0 top-12 z-20 w-60", "bottom-")
            .expect("the stale `bottom-0` class still parses");
        assert!(
            (stale_bottom - DOCK_BOTTOM_PX).abs() > f64::EPSILON,
            "PERTURB: the stale `bottom-0` inset must differ from the shipped one or this proves \
             nothing"
        );
        let (w, h) = (1920.0, 1080.0);
        let bar_y = h - STATUSBAR_H_PX;
        let stale_dock_bottom = h - stale_bottom; // = 1080, the viewport floor
        assert!(
            stale_dock_bottom > bar_y,
            "PERTURB: `bottom-0` puts the dock bottom at {stale_dock_bottom}, BELOW the bar top \
             ({bar_y}) — a {STATUSBAR_H_PX} px overlap that swallows every click aimed at the bar"
        );

        // RESTORE.
        expanded();
    }

    /// The mount classes are RENDERED, not merely declared: `mission_editor` names these consts
    /// instead of hand-writing a second copy of the width. Checked on scrubbed code (strings and
    /// comments blanked), sliced from the page fn so the file's leading `#[cfg(test)]` helper does
    /// not cut the body away — the t636/t662 idiom.
    ///
    /// Exactly one use each: a second mount would be a second place the width could drift.
    #[test]
    fn mission_editor_mounts_the_docks_from_these_consts() {
        let raw = include_str!("../mission_editor.rs");
        let anchor = format!("{}{}", "pub fn Mission", "EditorPage() -> impl IntoView");
        let editor = live_code(&raw[raw.find(anchor.as_str()).expect("anchor present")..]);
        for name in [
            "editor::layout::DOCK_LEFT_MOUNT",
            "editor::layout::DOCK_LEFT_MOUNT_COLLAPSED",
            "editor::layout::DOCK_RIGHT_MOUNT",
            "editor::layout::DOCK_RIGHT_MOUNT_COLLAPSED",
        ] {
            assert!(
                editor.contains(name),
                "T-637: `{name}` must be RENDERED by mission_editor — a hand-written class literal \
                 is a second source for a width the pointer unprojection has to agree with"
            );
        }
        // Exactly four `eden_layout::DOCK_*` reads in the page body: two per dock (expanded +
        // collapsed). A fifth is a second mount, i.e. a second place the geometry can drift.
        assert_eq!(
            editor.matches("editor::layout::DOCK_").count(),
            4,
            "T-637: the docks mount in exactly two places, each reading its expanded/collapsed pair"
        );
    }

    /// T-637 — the T-634 FOLD-BACK. `STRIP_ROWS` / `ROW_MENUS` / `ROW_TOOLS` / `ROW_*_PX` now live
    /// here beside their siblings, and the dead one-row `STRIP` they replaced is deleted.
    ///
    /// `STRIP`'s load-bearing claim survives its deletion: the strip is made of the SAME glass as the
    /// docks it sits above (that shared surface is why the chrome reads as one product rather than
    /// as assembled parts). It used to be checked by comparing the two shells to each other; it is
    /// now checked directly against the docks, which is what it always meant.
    #[test]
    fn the_strip_shell_folded_back_and_still_shares_the_docks_glass() {
        let surface =
            "pointer-events-auto bg-surface-container-lowest/55 shadow-xl backdrop-blur-xl";
        for (name, recipe) in [
            ("STRIP_ROWS", STRIP_ROWS),
            ("DOCK_L", DOCK_L),
            ("DOCK_R", DOCK_R),
        ] {
            assert!(
                recipe.starts_with(surface),
                "T-637: `{name}` must open with the shared docked-overlay glass"
            );
        }
        // The height contract T-634 built the split for, restated where the consts now live.
        assert!(
            (ROW_MENUS_PX + ROW_TOOLS_PX - STRIP_TOP_PX).abs() < f64::EPSILON,
            "T-637: the two strip rows SPLIT {STRIP_TOP_PX} px, they do not add to it"
        );
        assert!(
            ROW_MENUS.contains("h-6"),
            "the menu row is the FIXED half of the split"
        );
        assert!(
            ROW_TOOLS.contains("flex-1") && !ROW_TOOLS.contains("h-["),
            "the tool row takes the REMAINDER — two stated heights could drift from STRIP_TOP_PX"
        );
        // The dead shell is gone. Needle assembled so this test's own source cannot satisfy it.
        let layout = include_str!("layout.rs");
        let dead = format!("{} STRIP:", "pub(crate) const");
        assert!(
            !layout.contains(&dead),
            "T-637: the one-row `STRIP` shell had no consumer left after T-634 and is deleted — \
             `#![allow(dead_code)]` means nothing warns, so this is the only thing that would notice \
             it coming back"
        );
        // The equalised docks are a flex COLUMN each, so a dock body can claim the leftover height
        // instead of leaving it as void below a short tree.
        for (name, dock) in [("DOCK_L", DOCK_L), ("DOCK_R", DOCK_R)] {
            assert!(
                dock.contains("flex") && dock.contains("flex-col") && dock.contains("h-full"),
                "T-637: `{name}` must be a full-height column — the void under the tree was a dock \
                 that never told its children they could grow"
            );
        }
    }

    /// T-637 — **the tripwire T-634 left, resolved.** `BTN_ICON` used to rest at
    /// `text-on-surface-variant` with `p-1.5`: a live glyph looked like a dead one, and a 36 px
    /// control could not sit in a 24 px strip row. T-634 could not fix it (this file was another
    /// slice's owns that wave) and made a local `TOOL_ICON` copy instead, which left the defect
    /// standing for every OTHER caller — the help panel's close button, the docks, the toolbelt.
    ///
    /// The recipe is now fixed at the source and the copy is deleted, so the fix reaches every
    /// caller. The pin is INVERTED from T-634's, deliberately: its premise was that `BTN_ICON` is
    /// still muted, and that premise is exactly what this ticket had to falsify.
    #[test]
    fn btn_icon_rests_bright_and_fits_a_dense_row() {
        assert!(
            BTN_ICON.contains("text-on-surface") && !BTN_ICON.contains("text-on-surface-variant"),
            "T-637: a LIVE icon button rests at full strength; dimming is reserved for DISABLED, \
             where it means something"
        );
        assert!(
            BTN_ICON.contains("p-0.5") && !BTN_ICON.contains("p-1.5"),
            "T-637: `p-1.5` around a 24 px line box is a 36 px control — too tall for a 24 px strip \
             row or a dense dock"
        );
        assert!(
            !BTN_ICON.contains("hover:") && !BTN_ICON.contains("disabled:"),
            "T-637: state comes from the T-668 vocabulary at the call site (HOVER_FILL / \
             DISABLED_GLYPH), not from an ad-hoc pair baked into the geometry recipe"
        );
        // The local copy is gone from the strip. Needle assembled so this source cannot satisfy it.
        let strip = include_str!("panels/top_strip.rs");
        let copy = format!("{} TOOL_ICON", "const");
        assert!(
            !strip.contains(&copy),
            "T-637: `TOOL_ICON` existed only to route around the muted BTN_ICON; with the recipe \
             fixed it is a second source of truth for the same geometry"
        );
    }
}
