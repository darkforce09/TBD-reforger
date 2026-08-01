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
/// Bottom band reserved for the toolbelt. It floats (`bottom-5` ≈ 20 px + ~44 px tall) rather than
/// docking full-width, so this is a generous band, not an exact height.
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
