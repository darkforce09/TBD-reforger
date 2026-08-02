//! T-661 — Eden chrome layout constants, split from `eden_chrome.rs`.
//!
//! The chrome insets (`STRIP_TOP_PX`, `DOCK_LEFT_PX`, `DOCK_RIGHT_PX`, `TOOLBELT_BAND_PX`) are the
//! source the Tailwind utilities in `mission_editor`'s view are written from, and `select_tool` /
//! `mission_editor` read them back to keep pan/select/marquee gates aligned with the panels — so
//! they stay `pub`. The class recipes below are the shared `overlay.ts` ports used by the strip and
//! docks. Pure `const`s, no wasm; the native view shell compiles them too.
#![allow(dead_code)]

// ── Chrome insets (CSS px) ───────────────────────────────────────────────────────────────────────
// These ARE the source the Tailwind utilities in `mission_editor`'s view are written from, and
// `select_tool::farthest_empty_px` insets its probe grid by them so a "guaranteed-empty" click px
// can never land under a panel that would swallow the pointerdown. Change a class → change the
// const (and vice versa) — they are one contract, verified by the select + marquee gates.

/// Top Command Strip height — `h-12` / the docks' `top-12`.
pub const STRIP_TOP_PX: f64 = 48.0;
/// Left dock width — `w-64`.
pub const DOCK_LEFT_PX: f64 = 256.0;
/// Right dock width — `w-80`.
pub const DOCK_RIGHT_PX: f64 = 320.0;
/// Bottom band reserved for the toolbelt chrome — the region a pointer probe must stay ABOVE to be
/// on the real map, read identically by `select_tool::farthest_empty_px` and `mission_editor`'s
/// palette-drop `on_canvas` gate (the two live readers; a test pins that they agree on this const).
///
/// T-636 split the single floating pill into a full-width status bar docked at `inset-x-0 bottom-0`
/// (`h-9` = 36 px) with the mode toolbar floating just above it (`bottom-11` ≈ 44 px + a ~44 px
/// pill → its top edge sits ~88 px up). The band must clear the TALLER of the two — the floating
/// toolbar — so 96 px still holds with a small margin (it was already generous for the old pill).
/// It is a reserved band, not an exact surface height: the wgpu canvas is full-bleed and is NOT
/// inset by it (shrinking the canvas would invalidate every `select_tool` camera probe — see
/// `mission_editor`'s view note), so the canvas is deliberately not a third reader of this const.
pub const TOOLBELT_BAND_PX: f64 = 96.0;

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

/// T-636 — `TOOLBELT_BAND_PX` is an INPUT-HANDLING const: it is the bottom-chrome band the
/// pointer→world readers inset by, so if the readers disagree a click under the status bar would be
/// mapped to a world coordinate as if the bar were not there. This file OWNS the number; the two
/// live readers must consume it by NAME (via the `eden_chrome` re-export) rather than a magic
/// literal, so the T-636 status-bar re-layout — or any future height change — stays consistent by
/// construction across all three files. That is exactly what this pins.
///
/// It lives here (the const's owner, natively compiled) rather than in `select_tool`, which is
/// `#[cfg(target_arch = "wasm32")]` and so invisible to a native `cargo test`.
#[cfg(test)]
mod t636_band_readers_agree {
    use crate::arsenal::class_r_scrub::live_code;

    /// The band has ONE definition here, and both readers reference it by name — neither smuggles in
    /// a bare `96.0`. `live_code` blanks comments + string literals, so a `96` mentioned in prose or
    /// a class string can never satisfy (or false-fail) a needle; the definition itself is checked on
    /// raw source, where the `= 96.0` value is real code.
    #[test]
    fn both_readers_reference_the_single_band_const() {
        // Exactly one definition, in this file (raw — the literal is real code, not prose).
        let layout = include_str!("eden_layout.rs");
        let name = "TOOLBELT_BAND_PX";
        let def = format!("pub const {name}: f64 = 96.0;");
        assert!(
            layout.contains(&def),
            "eden_layout must DEFINE {name} exactly once as the band's single value (96.0)"
        );
        assert_eq!(
            layout.matches(&format!("pub const {name}")).count(),
            1,
            "{name} must have exactly one definition — one source of truth"
        );

        // Reader 1: select_tool::farthest_empty_px (the pointer→world probe-grid inset).
        // Reader 2: mission_editor's palette-drop `on_canvas` gate.
        // Both must consume the const through the `eden_chrome` re-export, by NAME.
        //
        // `live_code` (via `scrub`) cuts from a file's FIRST `#[cfg(test)]` to EOF. select_tool has
        // none, so its whole body scrubs. mission_editor's first `#[cfg(test)]` is a `clear_for_test`
        // helper near the TOP (above the band reader), so scrubbing the whole file would drop the
        // reader — slice from the page fn anchor first (the t662/t635 idiom), then scrub that.
        let read = format!("eden_chrome::{name}");
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
            sel.contains(&read),
            "select_tool must inset by the named const {name}, not a literal"
        );
        assert!(
            editor.contains(&read),
            "mission_editor's palette-drop on_canvas gate must inset by the named const {name}"
        );
        // Count the reads so a reader that quietly stops insetting by the band is caught: two live
        // sites today (one per file). Pinned as a floor so adding a legitimate third reader is fine.
        assert!(
            sel.matches(&read).count() >= 1 && editor.matches(&read).count() >= 1,
            "each reader file must reference the band const at least once"
        );

        // No reader may hardcode the height (that would silently diverge if the const changed). The
        // needle is split so this test's own source cannot satisfy it. eden_layout is excluded — it
        // legitimately holds the literal in the definition above.
        let bare = ["96", ".0"].concat();
        assert!(
            !sel.contains(&bare),
            "T-636: select_tool must not hardcode the band height — it comes from the const"
        );
        assert!(
            !editor.contains(&bare),
            "T-636: mission_editor must not hardcode the band height — it comes from the const"
        );
    }
}
