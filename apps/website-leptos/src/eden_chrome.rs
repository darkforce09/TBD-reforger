//! T-159.21 — Eden chrome scaffold for the Mission Creator (/missions/:id/edit).
//!
//! The docked shell React renders around the map: a Top Command Strip (title, Undo/Redo, the
//! T-159.20 Save/Export controls, a disabled Settings stub), a Bottom Toolbelt (Select + CUR/SEL/OBJ
//! readout), and left/right dock placeholders. This slice is the **scaffold**: the docks hold
//! placeholder text only — the outliner tree and asset palette land in T-159.22 (spec C4/C7).
//!
//! **Layering (React MissionCreatorPage:272):** the chrome overlays a full-bleed canvas; it never
//! shrinks it. Every `select_tool` probe builds its camera from the container's bounding rect, so a
//! resized container would silently invalidate the pan/select/marquee/move gates. The panels are
//! absolutely positioned inside the gesture container instead, and the host div stops `pointerdown`
//! from bubbling into the map handlers (see `mission_editor`'s view).
//!
//! **Not cfg-gated:** the components compile on the native target too (the `cargo check -p
//! website-leptos` shell). Nothing here touches a wasm-only type — the doc-driving `on:click` bodies
//! are `#[cfg(target_arch = "wasm32")]` inside the closure, the T-159.20 Save-button precedent.
#![allow(dead_code)]
use leptos::prelude::*;

use crate::ui::MaterialIcon;

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
const STRIP: &str = "pointer-events-auto bg-surface-container-lowest/55 shadow-xl backdrop-blur-xl flex h-full items-center gap-2 border-b border-white/10 px-3";
/// `cn(overlayDocked, …)` + the dock's own edge border.
const DOCK_L: &str = "pointer-events-auto bg-surface-container-lowest/55 shadow-xl backdrop-blur-xl h-full overflow-y-auto border-r border-white/10 p-3";
const DOCK_R: &str = "pointer-events-auto bg-surface-container-lowest/55 shadow-xl backdrop-blur-xl h-full overflow-y-auto border-l border-white/10 p-3";
/// `cn(overlayPanel, 'flex items-center gap-1 px-1.5 py-1.5')`.
const TOOLBELT: &str = "pointer-events-auto rounded-xl border border-white/10 bg-surface-container-lowest/55 shadow-xl backdrop-blur-xl flex items-center gap-1 px-1.5 py-1.5";

/// The shared icon-button recipe (React TopCommandStrip:148).
const BTN_ICON: &str = "rounded-md p-1.5 text-on-surface-variant transition-colors hover:bg-white/10 disabled:opacity-30 disabled:hover:bg-transparent";
/// A vertical hairline divider (React `<span className="h-5 w-px bg-white/10" />`).
const DIVIDER: &str = "h-5 w-px bg-white/10";

/// A toolbelt tool button — active (Select) vs disabled stub (Ruler / LoS).
const TOOL_ACTIVE: &str =
    "flex items-center gap-1.5 rounded-lg px-2.5 py-1.5 text-label-md transition-colors bg-primary/20 text-primary";
const TOOL_DISABLED: &str = "flex items-center gap-1.5 rounded-lg px-2.5 py-1.5 text-label-md transition-colors text-on-surface-variant opacity-30 hover:bg-transparent";

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

/// Top Command Strip — title · Undo/Redo · Save/Export (T-159.20, moved here) · Settings stub.
///
/// Scope (spec C1/C7): no File/Edit/View menu stubs, no time scrubber / weather select, no history
/// dropdown, no Mission Settings dialog, and no Save dialog (semver stays an inline input — the
/// dialog's size estimate + progress + debug panel are a later slice).
#[component]
pub fn TopCommandStrip(
    /// Mission title. The `:id` route param today; the doc's `meta.title` once the settings/hydrate
    /// lane lands (React binds an editable input to `setTitle`).
    title: String,
    can_undo: RwSignal<bool>,
    can_redo: RwSignal<bool>,
    save_semver: RwSignal<String>,
    save_status: RwSignal<String>,
) -> impl IntoView {
    view! {
        <div class=STRIP>
            <span class="min-w-0 flex-1 truncate text-label-md font-semibold text-on-surface">
                {title}
            </span>
            <span class=DIVIDER></span>
            // `aria-label` is the gate's DOM handle for the button path (smoke_undo_editor A3/A6) —
            // a real a11y name, not a test-only attribute.
            <button
                type="button"
                aria-label="Undo"
                title="Undo (Ctrl+Z)"
                class=BTN_ICON
                disabled=move || !can_undo.get()
                on:click=move |_| {
                    #[cfg(target_arch = "wasm32")]
                    {
                        crate::mission_history::undo();
                    }
                }
            >
                <MaterialIcon name="undo" class="block text-base" />
            </button>
            <button
                type="button"
                aria-label="Redo"
                title="Redo (Ctrl+Shift+Z)"
                class=BTN_ICON
                disabled=move || !can_redo.get()
                on:click=move |_| {
                    #[cfg(target_arch = "wasm32")]
                    {
                        crate::mission_history::redo();
                    }
                }
            >
                <MaterialIcon name="redo" class="block text-base" />
            </button>
            <span class=DIVIDER></span>
            <input
                type="text"
                aria-label="Version"
                class="w-20 rounded border border-outline-variant/40 bg-surface-container px-2 py-1 font-mono text-xs text-on-surface"
                prop:value=move || save_semver.get()
                on:input=move |ev| save_semver.set(event_target_value(&ev))
            />
            <button
                type="button"
                class="rounded bg-primary px-3 py-1 text-xs font-medium text-on-primary"
                on:click=move |_| {
                    #[cfg(target_arch = "wasm32")]
                    crate::mission_commands::save_now(save_semver.get_untracked(), save_status);
                }
            >
                "Save Version"
            </button>
            <button
                type="button"
                class="rounded border border-outline-variant/40 px-3 py-1 text-xs font-medium text-on-surface"
                on:click=move |_| {
                    #[cfg(target_arch = "wasm32")]
                    crate::mission_commands::export_now(&save_semver.get_untracked());
                }
            >
                "Export JSON"
            </button>
            <span class="min-w-24 font-mono text-xs text-on-surface-variant">
                {move || save_status.get()}
            </span>
            // Settings stub (spec C1: "optional disabled Settings stub"). The Mission Settings
            // dialog (hillshade / grid / environment) is out of scope for the scaffold.
            <button type="button" aria-label="Mission settings" class=BTN_ICON disabled=true>
                <MaterialIcon name="settings" class="block text-base" />
            </button>
        </div>
    }
}

/// Left dock placeholder — ORBAT + Editor Layers trees land in T-159.22 (spec C4).
#[component]
pub fn DockLeft() -> impl IntoView {
    view! {
        <aside class=DOCK_L>
            <h2 class="text-label-sm font-semibold uppercase tracking-wide text-on-surface-variant">
                "ORBAT / Layers"
            </h2>
            <p class="mt-2 text-label-sm text-outline">"Outliner lands in T-159.22."</p>
        </aside>
    }
}

/// Right dock placeholder — the Asset Palette lands in T-159.22 (spec C4).
#[component]
pub fn DockRight() -> impl IntoView {
    view! {
        <aside class=DOCK_R>
            <h2 class="text-label-sm font-semibold uppercase tracking-wide text-on-surface-variant">
                "Assets"
            </h2>
            <p class="mt-2 text-label-sm text-outline">"Asset palette lands in T-159.22."</p>
        </aside>
    }
}

/// Bottom Toolbelt — Select (active) + Ruler/LoS disabled stubs, then the mono CUR X/Y + SEL/OBJ
/// readout.
///
/// Scope (spec C3): CUR is **X/Y only** — no Z column (React's Z is DEM-fed), no SEL-mode coordinate
/// swap, and no `SZ` payload estimate.
#[component]
pub fn BottomToolbelt(
    /// Cursor world position, `None` when the pointer is off the map (renders the em-dash cells).
    cursor: RwSignal<Option<(f64, f64)>>,
    sel_count: RwSignal<usize>,
    obj_count: RwSignal<usize>,
) -> impl IntoView {
    view! {
        <div class=TOOLBELT>
            <button type="button" class=TOOL_ACTIVE aria-pressed="true" title="Select">
                <MaterialIcon name="arrow_selector_tool" class="block text-base" />
                <span class="hidden sm:inline">"Select"</span>
            </button>
            <button type="button" class=TOOL_DISABLED disabled=true title="Ruler (soon)">
                <MaterialIcon name="straighten" class="block text-base" />
                <span class="hidden sm:inline">"Ruler"</span>
            </button>
            <button type="button" class=TOOL_DISABLED disabled=true title="Line of sight (soon)">
                <MaterialIcon name="visibility" class="block text-base" />
                <span class="hidden sm:inline">"LoS"</span>
            </button>
            <span class="mx-1 h-5 w-px bg-white/10"></span>
            <div class="flex items-center gap-2 px-1 font-mono text-code-md text-on-surface-variant">
                <span class="text-outline" title="Cursor">"CUR"</span>
                <span>
                    "X"
                    <span class="ml-1 text-on-surface tabular-nums">
                        {move || fmt_coord(cursor.get().map(|c| c.0))}
                    </span>
                </span>
                <span>
                    "Y"
                    <span class="ml-1 text-on-surface tabular-nums">
                        {move || fmt_coord(cursor.get().map(|c| c.1))}
                    </span>
                </span>
            </div>
            <span class="mx-1 h-5 w-px bg-white/10"></span>
            <div
                class="flex items-center gap-2 px-1 font-mono text-code-md tabular-nums text-on-surface-variant"
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
            </div>
        </div>
    }
}
