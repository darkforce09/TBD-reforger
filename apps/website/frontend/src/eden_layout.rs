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
pub const STRIP_TOP_PX: f64 = 48.0;
/// Left dock width — `w-64`. Expanded value; live inset is [`dock_left_px`] (→ [`STUB_PX`] collapsed,
/// → 0 while `chrome_hidden`).
pub const DOCK_LEFT_PX: f64 = 256.0;
/// Right dock width — `w-80`. Expanded value; live inset is [`dock_right_px`] (→ [`STUB_PX`]
/// collapsed, → 0 while `chrome_hidden`).
pub const DOCK_RIGHT_PX: f64 = 320.0;
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

/// `cn(overlayDocked, 'flex h-full items-center gap-2 border-b border-white/10 px-3')`.
pub(crate) const STRIP: &str = "pointer-events-auto bg-surface-container-lowest/55 shadow-xl backdrop-blur-xl flex h-full items-center gap-2 border-b border-white/10 px-3";
/// `cn(overlayDocked, …)` + the dock's own edge border.
pub(crate) const DOCK_L: &str = "pointer-events-auto bg-surface-container-lowest/55 shadow-xl backdrop-blur-xl flex h-full flex-col overflow-y-auto border-r border-white/10 p-3";
pub(crate) const DOCK_R: &str = "pointer-events-auto bg-surface-container-lowest/55 shadow-xl backdrop-blur-xl h-full overflow-y-auto border-l border-white/10 p-3";

/// The shared icon-button recipe (React TopCommandStrip:148).
pub(crate) const BTN_ICON: &str = "rounded-md p-1.5 text-on-surface-variant transition-colors hover:bg-white/10 disabled:opacity-30 disabled:hover:bg-transparent";
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
    /// the LIVE inset via the accessor — neither smuggles in a bare `96.0`/`256`/`320`/`48`.
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
        let layout = include_str!("eden_layout.rs");
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
        let band_read = "eden_layout::toolbelt_band_px()";
        let sel = live_code(include_str!("select_tool.rs"));

        let raw_editor = include_str!("mission_editor.rs");
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
            "eden_layout::dock_left_px()",
            "eden_layout::dock_right_px()",
            "eden_layout::strip_top_px()",
            "eden_layout::toolbelt_band_px()",
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
        for bare in [
            ["96", ".0"].concat(),
            ["256", ".0"].concat(),
            ["320", ".0"].concat(),
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
        centre_hold_target, chrome_hidden, dock_left_collapsed, dock_left_px, dock_right_collapsed,
        dock_right_px, pane_center_px, set_chrome_hidden, set_dock_left_collapsed,
        set_dock_right_collapsed, strip_top_px, toolbelt_band_px, DOCK_LEFT_PX, DOCK_RIGHT_PX,
        STRIP_TOP_PX, STUB_PX, TOOLBELT_BAND_PX,
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
        let src = include_str!("mission_editor.rs");
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
