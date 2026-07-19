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
//! website-frontend` shell). Nothing here touches a wasm-only type — the doc-driving `on:click` bodies
//! are `#[cfg(target_arch = "wasm32")]` inside the closure, the T-159.20 Save-button precedent.
#![allow(dead_code)]
use leptos::prelude::*;

use crate::asset_catalog::{CatalogNode, CatalogState};
use crate::outliner::{flatten_visible, FlatRow, NodeKind, OutlinerNode, VIRTUAL_SLOT_THRESHOLD};
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
const DOCK_L: &str = "pointer-events-auto bg-surface-container-lowest/55 shadow-xl backdrop-blur-xl flex h-full flex-col overflow-y-auto border-r border-white/10 p-3";
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

// Top Command Strip (T-172 B9) — menu bar · editable title · time scrubber + weather ·
// History (disabled) · Undo/Redo · Save dialog · Export · Settings.

/// One top-strip menu (T-172 B9). React rendered File/Edit/View/Mission/Environment as dead
/// "(soon)" stubs; these open real dropdowns with the commands that exist. No DOM while closed.
struct MenuItem {
    label: &'static str,
    /// None = disabled row (rendered, not clickable — parity with genuinely-future features).
    action: Option<MenuAction>,
}

#[derive(Clone, Copy)]
enum MenuAction {
    Save,
    Export,
    Undo,
    Redo,
    Settings,
}

const MENUS: [(&str, &[MenuItem]); 5] = [
    (
        "File",
        &[
            MenuItem {
                label: "Save Version…",
                action: Some(MenuAction::Save),
            },
            MenuItem {
                label: "Export JSON",
                action: Some(MenuAction::Export),
            },
        ],
    ),
    (
        "Edit",
        &[
            MenuItem {
                label: "Undo",
                action: Some(MenuAction::Undo),
            },
            MenuItem {
                label: "Redo",
                action: Some(MenuAction::Redo),
            },
        ],
    ),
    (
        "View",
        &[MenuItem {
            label: "Map layers — render host (T-159.28)",
            action: None,
        }],
    ),
    (
        "Mission",
        &[MenuItem {
            label: "Mission Settings…",
            action: Some(MenuAction::Settings),
        }],
    ),
    (
        "Environment",
        &[MenuItem {
            label: "Time & Weather (Mission Settings)…",
            action: Some(MenuAction::Settings),
        }],
    ),
];

/// Minutes-since-midnight ↔ `HH:MM` for the time scrubber (T-172 B9). Pure + tested.
pub fn minutes_to_hhmm(min: u32) -> String {
    format!("{:02}:{:02}", (min / 60) % 24, min % 60)
}

pub fn hhmm_to_minutes(s: &str) -> Option<u32> {
    let (h, m) = s.split_once(':')?;
    let h: u32 = h.parse().ok()?;
    let m: u32 = m.parse().ok()?;
    if h > 23 || m > 59 {
        return None;
    }
    Some(h * 60 + m)
}

#[component]
pub fn TopCommandStrip(
    /// Mission title fallback — the `:id` route param; the doc's `meta.title` wins once read.
    title: String,
    can_undo: RwSignal<bool>,
    can_redo: RwSignal<bool>,
    save_semver: RwSignal<String>,
    save_status: RwSignal<String>,
    /// T-159.26 — unsaved-changes flag; a `•` after the title marks dirty (React's `isDirty` dot).
    #[prop(optional)]
    dirty: Option<RwSignal<bool>>,
    /// T-159.26 — the Mission Settings dialog's open flag (gear button toggles it).
    #[prop(optional)]
    settings_open: Option<RwSignal<bool>>,
    /// T-172 B9 — doc revision; re-reads the env (scrubber/weather) + title after undo/redo.
    #[prop(optional)]
    doc_tick: Option<RwSignal<u64>>,
    /// T-172 B9 — obj count for the Save dialog's size line.
    #[prop(optional)]
    obj_count: Option<RwSignal<usize>>,
    /// T-177 B2 / T-071.0 — the ORBAT Manager modal's open flag (the top-strip button toggles it).
    /// Disabled in the scaffold-only case, like `settings_open`.
    #[prop(optional)]
    orbat_open: Option<RwSignal<bool>>,
) -> impl IntoView {
    let open_menu = RwSignal::new(None::<usize>);
    let save_open = RwSignal::new(false);
    let save_notes = RwSignal::new(String::new());
    #[cfg(target_arch = "wasm32")]
    {
        let esc = window_event_listener(leptos::ev::keydown, move |ev| {
            if ev.key() == "Escape" {
                if open_menu.get_untracked().is_some() {
                    open_menu.set(None);
                }
                if save_open.get_untracked() {
                    save_open.set(false);
                }
            }
        });
        on_cleanup(move || esc.remove());
    }
    // Env mirror for the inline scrubber/weather — re-read on every doc change.
    let env = Memo::new(move |_| {
        if let Some(t) = doc_tick {
            t.track();
        }
        #[cfg(target_arch = "wasm32")]
        {
            crate::editor_ops::read_env()
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            crate::dto::MissionEnv::default()
        }
    });
    let run_action = move |a: MenuAction| {
        open_menu.set(None);
        match a {
            MenuAction::Save => save_open.set(true),
            MenuAction::Export => {
                #[cfg(target_arch = "wasm32")]
                crate::mission_commands::export_now(&save_semver.get_untracked());
            }
            MenuAction::Undo => {
                #[cfg(target_arch = "wasm32")]
                crate::mission_history::undo();
            }
            MenuAction::Redo => {
                #[cfg(target_arch = "wasm32")]
                crate::mission_history::redo();
            }
            MenuAction::Settings => {
                if let Some(s) = settings_open {
                    s.set(true);
                }
            }
        }
    };
    let title_fallback = StoredValue::new(title);
    view! {
        <div class=STRIP>
            // Menu bar (screen 05: File / Edit / View / Mission / Environment).
            <div class="flex items-center">
                {MENUS
                    .iter()
                    .enumerate()
                    .map(|(i, (name, items))| {
                        view! {
                            <div class="relative">
                                <button
                                    type="button"
                                    class=move || {
                                        if open_menu.get() == Some(i) {
                                            "rounded bg-white/10 px-2 py-1 text-label-sm text-on-surface"
                                        } else {
                                            "rounded px-2 py-1 text-label-sm text-on-surface-variant transition-colors hover:bg-white/10 hover:text-on-surface"
                                        }
                                    }
                                    on:click=move |_| {
                                        open_menu
                                            .update(|m| {
                                                *m = if *m == Some(i) { None } else { Some(i) };
                                            });
                                    }
                                >
                                    {*name}
                                </button>
                                {move || {
                                    (open_menu.get() == Some(i))
                                        .then(|| {
                                            view! {
                                                <div class="glass animate-menu-in absolute top-full left-0 z-50 mt-1 w-64 rounded-lg py-1 shadow-lg">
                                                    {items
                                                        .iter()
                                                        .map(|it| {
                                                            let label = it.label;
                                                            match it.action {
                                                                Some(a) => {
                                                                    let disabled = move || match a {
                                                                        MenuAction::Undo => !can_undo.get(),
                                                                        MenuAction::Redo => !can_redo.get(),
                                                                        _ => false,
                                                                    };
                                                                    view! {
                                                                        <button
                                                                            type="button"
                                                                            class="flex w-full items-center px-3 py-1.5 text-left text-label-sm text-on-surface transition-colors hover:bg-white/10 disabled:cursor-default disabled:text-outline disabled:hover:bg-transparent"
                                                                            disabled=disabled
                                                                            on:click=move |_| run_action(a)
                                                                        >
                                                                            {label}
                                                                        </button>
                                                                    }
                                                                        .into_any()
                                                                }
                                                                None => {
                                                                    view! {
                                                                        <span class="flex w-full items-center px-3 py-1.5 text-label-sm text-outline">
                                                                            {label}
                                                                        </span>
                                                                    }
                                                                        .into_any()
                                                                }
                                                            }
                                                        })
                                                        .collect_view()}
                                                </div>
                                            }
                                        })
                                }}
                            </div>
                        }
                    })
                    .collect_view()}
            </div>
            // T-177 B2 / T-071.0 — ORBAT Manager: opens the modal shell (browse/select the live
            // faction → squad → slot tree). Sits right of the Environment menu. Disabled in the
            // scaffold-only case (no `orbat_open` signal), mirroring the settings gear.
            <button
                type="button"
                aria-label="ORBAT Manager"
                class="rounded px-2 py-1 text-label-sm font-semibold text-primary transition-colors hover:bg-primary/15 disabled:opacity-30 disabled:hover:bg-transparent"
                disabled=orbat_open.is_none()
                on:click=move |_| {
                    if let Some(o) = orbat_open {
                        o.set(true);
                    }
                }
            >
                "ORBAT Manager"
            </button>
            // Click-away scrim for an open menu (below the dropdowns' z-50).
            {move || {
                open_menu
                    .get()
                    .is_some()
                    .then(|| {
                        view! {
                            <div
                                class="fixed inset-0 z-40"
                                on:click=move |_| open_menu.set(None)
                            ></div>
                        }
                    })
            }}
            <span class=DIVIDER></span>
            // Editable mission title (React setTitle) + the dirty dot.
            <div class="flex min-w-0 flex-1 items-center">
                {move || {
                    if let Some(t) = doc_tick {
                        t.track();
                    }
                    #[cfg(target_arch = "wasm32")]
                    let doc_title = {
                        let t = crate::editor_ops::read_title();
                        if t.is_empty() { title_fallback.get_value() } else { t }
                    };
                    #[cfg(not(target_arch = "wasm32"))]
                    let doc_title = title_fallback.get_value();
                    view! {
                        <input
                            type="text"
                            aria-label="Mission title"
                            class="w-full min-w-0 truncate rounded border border-transparent bg-transparent px-1.5 py-0.5 text-label-md font-semibold text-on-surface outline-none transition-colors focus:border-outline-variant/40 focus:bg-surface-container"
                            prop:value=doc_title
                            on:change=move |ev| {
                                #[cfg(target_arch = "wasm32")]
                                {
                                    let v = event_target_value(&ev);
                                    if !v.trim().is_empty() {
                                        crate::editor_ops::set_title(v.trim());
                                    }
                                }
                                #[cfg(not(target_arch = "wasm32"))]
                                let _ = &ev;
                            }
                        />
                    }
                }}
                {dirty
                    .map(|d| {
                        view! {
                            <span
                                class=move || if d.get() { "ml-1.5 text-primary" } else { "hidden" }
                                title="Unsaved changes"
                                aria-label="Unsaved changes"
                            >
                                "•"
                            </span>
                        }
                    })}
            </div>
            // Inline time scrubber + weather (screen 05 center) — same doc fields as the
            // Mission Settings dialog (`update_environment`, one undo step per commit).
            <div class="flex shrink-0 items-center gap-2">
                <input
                    type="range"
                    min="0"
                    max="1439"
                    step="1"
                    aria-label="Time of day"
                    class="w-28 accent-[--color-primary]"
                    prop:value=move || {
                        hhmm_to_minutes(&env.get().time).unwrap_or(360).to_string()
                    }
                    on:change=move |ev| {
                        #[cfg(target_arch = "wasm32")]
                        {
                            let v: u32 = event_target_value(&ev).parse().unwrap_or(0);
                            crate::editor_ops::update_environment(
                                serde_json::json!({ "time": minutes_to_hhmm(v) }).to_string(),
                            );
                        }
                        #[cfg(not(target_arch = "wasm32"))]
                        let _ = &ev;
                    }
                />
                <span class="font-mono text-xs tabular-nums text-on-surface-variant">
                    {move || env.get().time}
                </span>
                <select
                    aria-label="Weather"
                    class="rounded border border-outline-variant/40 bg-surface-container px-1.5 py-0.5 text-xs text-on-surface"
                    prop:value=move || env.get().weather
                    on:change=move |ev| {
                        #[cfg(target_arch = "wasm32")]
                        crate::editor_ops::update_environment(
                            serde_json::json!({ "weather": event_target_value(&ev) }).to_string(),
                        );
                        #[cfg(not(target_arch = "wasm32"))]
                        let _ = &ev;
                    }
                >
                    <option value="clear">"Clear"</option>
                    <option value="overcast">"Overcast"</option>
                    <option value="heavy_rain">"Heavy Rain"</option>
                    <option value="dense_fog">"Dense Fog"</option>
                </select>
            </div>
            <span class=DIVIDER></span>
            // History — present-but-disabled (React parity; version list lands with the history
            // lane).
            <button
                type="button"
                aria-label="History"
                title="Version history (soon)"
                class=BTN_ICON
                disabled=true
            >
                <MaterialIcon name="history" class="block text-base" />
            </button>
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
            <button
                type="button"
                class="rounded bg-primary px-3 py-1 text-xs font-medium text-on-primary"
                on:click=move |_| save_open.set(true)
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
            // T-159.26 — Mission Settings (environment). Opens the dialog when a `settings_open`
            // signal is threaded (the editor); disabled in the scaffold-only case.
            <button
                type="button"
                aria-label="Mission settings"
                class=BTN_ICON
                disabled=settings_open.is_none()
                on:click=move |_| {
                    if let Some(s) = settings_open {
                        s.set(true);
                    }
                }
            >
                <MaterialIcon name="settings" class="block text-base" />
            </button>
            // Save Version dialog (React SaveVersionDialog: semver + notes + size estimate +
            // indeterminate bar while saving). Renders no DOM while closed.
            {move || {
                save_open
                    .get()
                    .then(|| {
                        let estimate = {
                            #[cfg(target_arch = "wasm32")]
                            {
                                crate::editor_ops::slots_json()
                                    .as_deref()
                                    .and_then(crate::mission_size::estimate_compiled_bytes)
                            }
                            #[cfg(not(target_arch = "wasm32"))]
                            {
                                None::<usize>
                            }
                        };
                        let obj = obj_count.map_or(0, |o| o.get());
                        let size_line = match estimate {
                            Some(b) => {
                                format!(
                                    "~{} · {} objects",
                                    crate::mission_size::format_bytes(b),
                                    obj,
                                )
                            }
                            None => format!("{obj} objects"),
                        };
                        let big = estimate.is_some_and(|b| b > 200_000_000);
                        view! {
                            <div
                                class="animate-overlay-fade fixed inset-0 z-50 bg-black/50 backdrop-blur-sm"
                                on:click=move |_| save_open.set(false)
                            ></div>
                            <div class="glass animate-dialog-in fixed top-1/2 left-1/2 z-50 flex max-h-[85vh] w-[92vw] max-w-md -translate-x-1/2 -translate-y-1/2 flex-col rounded-xl shadow-2xl outline-none">
                                <div class="flex items-start justify-between gap-4 border-b border-outline-variant/30 px-6 py-4">
                                    <div class="min-w-0">
                                        <h2 class="text-headline-sm text-on-surface">"Save Version"</h2>
                                        <p class="mt-1 text-label-md text-on-surface-variant">
                                            "Versions are immutable — pick a new semver."
                                        </p>
                                    </div>
                                    <button
                                        type="button"
                                        aria-label="Close"
                                        on:click=move |_| save_open.set(false)
                                        class="shrink-0 rounded-md p-1 text-outline transition-colors hover:bg-surface-variant/50 hover:text-on-surface"
                                    >
                                        <MaterialIcon name="close" />
                                    </button>
                                </div>
                                <div class="flex flex-col gap-3 px-6 py-5">
                                    <label class="flex flex-col gap-1">
                                        <span class="text-label-sm uppercase tracking-wider text-outline">
                                            "Version"
                                        </span>
                                        <input
                                            type="text"
                                            aria-label="Version"
                                            class="w-32 rounded border border-outline-variant/40 bg-surface-container px-2 py-1 font-mono text-xs text-on-surface"
                                            prop:value=move || save_semver.get()
                                            on:input=move |ev| save_semver.set(event_target_value(&ev))
                                        />
                                    </label>
                                    <label class="flex flex-col gap-1">
                                        <span class="text-label-sm uppercase tracking-wider text-outline">
                                            "Notes"
                                        </span>
                                        <textarea
                                            aria-label="Editor notes"
                                            rows="2"
                                            class="w-full resize-none rounded border border-outline-variant/40 bg-surface-container px-2 py-1 text-xs text-on-surface"
                                            prop:value=move || save_notes.get()
                                            on:input=move |ev| save_notes.set(event_target_value(&ev))
                                        ></textarea>
                                    </label>
                                    <p class=if big {
                                        "font-mono text-xs text-tactical-yellow"
                                    } else {
                                        "font-mono text-xs text-on-surface-variant"
                                    }>{size_line}</p>
                                    {move || {
                                        save_status
                                            .get()
                                            .starts_with("Saving")
                                            .then(|| {
                                                view! {
                                                    <div class="h-1 w-full overflow-hidden rounded-full bg-surface-variant/40">
                                                        <div class="animate-mc-load-bar h-full w-1/4 rounded-full bg-primary"></div>
                                                    </div>
                                                }
                                            })
                                    }}
                                    <p class="min-h-4 font-mono text-xs text-on-surface-variant">
                                        {move || save_status.get()}
                                    </p>
                                    <button
                                        type="button"
                                        class="self-end rounded bg-primary px-4 py-1.5 text-xs font-medium text-on-primary"
                                        on:click=move |_| {
                                            #[cfg(target_arch = "wasm32")]
                                            crate::mission_commands::save_now(
                                                save_semver.get_untracked(),
                                                save_notes.get_untracked(),
                                                save_status,
                                            );
                                        }
                                    >
                                        "Save"
                                    </button>
                                </div>
                            </div>
                        }
                    })
            }}
        </div>
    }
}

// ── Tree rows (T-159.22 / T-172 B6+B7) ──────────────────────────────────────────────────────────
// Both trees collapse: container rows carry a chevron toggle (span, not a nested button — rows are
// `<button>`s) + open/closed folder icons, and depth renders as border-l guide-line runs instead of
// bare padding (the React `TreeView` look). The outliner/ORBAT collapsed sets start EMPTY (fully
// expanded — the T-169 windowing smoke's totals depend on it); the palette seeds from
// `CatalogNode::default_expanded` (only depth-0 faction folders open, `buildCatalogTree` rule 3).
//
// Rows are `<button>`s with a real `aria-label` — focusable, activatable, and the gates' DOM handle,
// the `aria-label="Undo"` precedent above (NOT a test-only attribute).

/// A tree row's shared recipe; depth renders as leading guide-line spans (see `guide_spans`).
const ROW: &str = "relative flex w-full items-center gap-1.5 rounded px-1.5 py-1 text-left text-label-sm text-on-surface-variant transition-colors hover:bg-white/10 hover:text-on-surface";
const ROW_ACTIVE: &str = "relative flex w-full items-center gap-1.5 rounded px-1.5 py-1 text-left text-label-sm transition-colors bg-primary/20 text-primary";
/// T-177 A2 — the palette-leaf variant of [`ROW`]: adds `cursor-grab` (→ `cursor-grabbing` while
/// pressed) so hovering a placeable role advertises the drag affordance. Folders keep `cursor-pointer`
/// and outliner slots keep the plain [`ROW`] default (only palette leaves are drag-to-place).
const PALETTE_LEAF: &str = "relative flex w-full items-center gap-1.5 rounded px-1.5 py-1 text-left text-label-sm text-on-surface-variant transition-colors hover:bg-white/10 hover:text-on-surface cursor-grab active:cursor-grabbing";

/// Hierarchy guide lines — YouTube-comment-style nested connectors (T-177 A1; supersedes the T-173 P7
/// straight rails). `ancestors` comes from `FlatRow` (`len == depth`) and says, per guide column,
/// whether that vertical line CONTINUES below this row. Columns `0..depth-1` are **ancestor spines**:
/// a full `inset-y-0` hairline drawn only where the branch continues (`ancestors[k]`), so there is no
/// orphan rail beneath a last child. The deepest column `depth-1` is THIS row's own connector — a
/// rounded **elbow** (a `border-l`+`border-b`+`rounded-bl` box over the row's top half that curves the
/// spine right into the chevron/icon) plus, only when this row has a following sibling
/// (`ancestors[depth-1]`), a `top-1/2 bottom-0` tail down to the next sibling (else the spine trims at
/// the last child — the YouTube look). Percentage heights (`h-1/2`/`top-1/2`) track `ROW_H`; the `w-3`
/// spacers still carry the indentation. Rows stack flush (`ROW_H` == the border-box height), so a
/// column's spans meet across rows into one continuous stem — the T-174 S3 per-row clip (no dock-tall
/// rails) is preserved. `bg-white/25` / `border-white/25` = the same subtle Aegis-legal grey. The row
/// must be `relative` (ROW already is). Column k center = left padding (0.375rem) + k·0.75rem + half
/// a column (0.375rem).
fn guide_spans(ancestors: &[bool]) -> AnyView {
    let depth = ancestors.len();
    if depth == 0 {
        return ().into_any();
    }
    let col_left = |k: usize| format!("left:calc(0.375rem + {:.3}rem)", (k as f64) * 0.75 + 0.375);
    let mut lines: Vec<AnyView> = Vec::new();
    // Ancestor spines: full-height hairline only where that ancestor's branch continues.
    for (k, cont) in ancestors.iter().enumerate().take(depth - 1) {
        if *cont {
            lines.push(
                view! {
                    <span
                        class="pointer-events-none absolute inset-y-0 w-px bg-white/25"
                        style=col_left(k)
                    ></span>
                }
                .into_any(),
            );
        }
    }
    // This row's own connector: a rounded elbow over the top half, curving into the row.
    let last = depth - 1;
    lines.push(
        view! {
            <span
                class="pointer-events-none absolute top-0 h-1/2 w-2 rounded-bl-[6px] border-b border-l border-white/25"
                style=col_left(last)
            ></span>
        }
        .into_any(),
    );
    // Tail to the next sibling — omitted at the last child so the spine stops (YouTube trim).
    if ancestors[last] {
        lines.push(
            view! {
                <span
                    class="pointer-events-none absolute top-1/2 bottom-0 w-px bg-white/25"
                    style=col_left(last)
                ></span>
            }
            .into_any(),
        );
    }
    let spacers = (0..depth)
        .map(|_| view! { <span class="w-3 shrink-0"></span> })
        .collect::<Vec<_>>();
    view! { {lines}{spacers} }.into_any()
}

/// Chevron toggle for container rows (`expand_more` open / `chevron_right` closed) — a
/// `role="button"` span so it can nest inside the row `<button>`; leaves get an alignment
/// spacer. Clicking toggles the id in `collapsed` without firing the row action.
fn chevron_or_spacer(
    has_children: bool,
    open: bool,
    id: &str,
    collapsed: RwSignal<std::collections::HashSet<String>>,
) -> AnyView {
    if !has_children {
        return view! { <span class="size-4 shrink-0"></span> }.into_any();
    }
    let cid = id.to_string();
    let icon = if open { "expand_more" } else { "chevron_right" };
    view! {
        <span
            role="button"
            tabindex="-1"
            aria-expanded=if open { "true" } else { "false" }
            class="flex size-4 shrink-0 cursor-pointer items-center justify-center rounded text-outline transition-colors hover:bg-white/10 hover:text-on-surface"
            on:click=move |ev| {
                ev.stop_propagation();
                collapsed
                    .update(|c| {
                        if !c.remove(&cid) {
                            c.insert(cid.clone());
                        }
                    });
            }
        >
            <MaterialIcon name=icon class="block text-sm" />
        </span>
    }
    .into_any()
}

/// T-169 — window geometry. `ROW_H` is the flow height of one row (`px-1.5 py-1 text-label-sm`);
/// the spacers use it to reserve the off-screen rows. `OVERSCAN` renders a few rows past the
/// viewport each way so a fast scroll never flashes blank.
const ROW_H: f64 = 24.0;
const CONTAINER_H: f64 = 420.0;
const OVERSCAN: usize = 6;

/// Render ONE flattened outliner row (no recursion — the windowed list draws a flat slice).
/// Header kinds (Unfiled / Faction / Squad) are inert; Folder → active-layer, Slot → select +
/// dbl-click→Attributes (SEL-ORBAT-DBL-001).
fn single_row(
    row: &FlatRow,
    selected: RwSignal<Vec<String>>,
    active_layer: RwSignal<Option<String>>,
    collapsed: RwSignal<std::collections::HashSet<String>>,
) -> AnyView {
    let label = row.label.clone();
    let aria = row.label.clone();
    let id = row.id.clone();
    // T-177 A1 — per-row guide continuation (see `guide_spans`); replaces the bare `depth`.
    let ancestors: &[bool] = &row.ancestors;
    // Static per build — a chevron toggle bumps `collapsed`, which re-flattens + re-renders
    // the slice (the virtual_tree Effect tracks it), so open state never goes stale.
    let open = !collapsed.with_untracked(|c| c.contains(&row.id));
    let toggle = chevron_or_spacer(row.has_children, open, &row.id, collapsed);
    match row.kind {
        NodeKind::Unfiled => view! {
            <div class="relative flex items-center gap-1.5 px-1.5 py-1 text-label-sm text-outline">
                {guide_spans(ancestors)}
                {toggle}
                <MaterialIcon name="inbox" class="block text-sm" />
                <span>{label}</span>
            </div>
        }
        .into_any(),
        NodeKind::Faction => view! {
            <div class="relative flex items-center gap-1.5 px-1.5 py-1 text-label-sm font-semibold uppercase tracking-wide text-on-surface-variant">
                {guide_spans(ancestors)}
                {toggle}
                <MaterialIcon name="flag" class="block text-sm" />
                <span class="truncate">{label}</span>
            </div>
        }
        .into_any(),
        NodeKind::Squad => view! {
            <div class="relative flex items-center gap-1.5 px-1.5 py-1 text-label-sm text-on-surface-variant">
                {guide_spans(ancestors)}
                {toggle}
                <MaterialIcon name="groups" class="block text-sm" />
                <span class="truncate">{label}</span>
            </div>
        }
        .into_any(),
        NodeKind::Folder => {
            let is_active = {
                let id = id.clone();
                move || active_layer.get().as_deref() == Some(id.as_str())
            };
            let folder_icon = if open { "folder_open" } else { "folder" };
            view! {
                <button
                    type="button"
                    aria-label=aria
                    title="Make this the drop target"
                    class=move || if is_active() { ROW_ACTIVE } else { ROW }
                    on:click=move |_| {
                        #[cfg(target_arch = "wasm32")]
                        crate::editor_ops::set_active_layer(Some(id.clone()));
                    }
                >
                    {guide_spans(ancestors)}
                    {toggle}
                    <MaterialIcon name=folder_icon class="block text-sm" />
                    <span class="truncate">{label}</span>
                </button>
            }
            .into_any()
        }
        NodeKind::Slot => {
            let is_sel = {
                let id = id.clone();
                move || selected.get().iter().any(|s| s == &id)
            };
            let id_dbl = id.clone();
            view! {
                <button
                    type="button"
                    aria-label=aria
                    class=move || if is_sel() { ROW_ACTIVE } else { ROW }
                    on:click=move |_| {
                        #[cfg(target_arch = "wasm32")]
                        crate::editor_ops::select_slot(id.clone());
                    }
                    // T-159.26 A1 — outliner activate (native dblclick) opens Attributes,
                    // the SEL-ORBAT-DBL-001 contract.
                    on:dblclick=move |_| {
                        #[cfg(target_arch = "wasm32")]
                        crate::editor_ops::open_attributes(id_dbl.clone());
                        #[cfg(not(target_arch = "wasm32"))]
                        let _ = &id_dbl;
                    }
                >
                    {guide_spans(ancestors)}
                    {toggle}
                    <MaterialIcon name="person" class="block text-sm" />
                    <span class="truncate">{label}</span>
                </button>
            }
            .into_any()
        }
    }
}

/// T-169 — publish `window.__outlinerStats[key] = {total, rendered, threshold}` for the gate.
#[cfg(target_arch = "wasm32")]
fn set_outliner_stats(key: &str, total: usize, rendered: usize) {
    use wasm_bindgen::JsValue;
    let Some(win) = web_sys::window() else { return };
    let stats = match js_sys::Reflect::get(&win, &JsValue::from_str("__outlinerStats")) {
        Ok(v) if v.is_object() => v,
        _ => {
            let o = js_sys::Object::new();
            let _ = js_sys::Reflect::set(&win, &JsValue::from_str("__outlinerStats"), &o);
            o.into()
        }
    };
    let entry = js_sys::Object::new();
    let set = |k: &str, n: usize| {
        let _ = js_sys::Reflect::set(&entry, &JsValue::from_str(k), &JsValue::from_f64(n as f64));
    };
    set("total", total);
    set("rendered", rendered);
    set("threshold", VIRTUAL_SLOT_THRESHOLD);
    let _ = js_sys::Reflect::set(&stats, &JsValue::from_str(key), &entry);
}
#[cfg(not(target_arch = "wasm32"))]
fn set_outliner_stats(_key: &str, _total: usize, _rendered: usize) {}

/// T-169 — render a dock tree, windowed above [`VIRTUAL_SLOT_THRESHOLD`]. Below it the whole
/// flattened list renders eagerly; above it a fixed-height scroll container draws only the visible
/// slice (+ overscan) between two spacer divs, so a mission-scale tree never builds N DOM rows.
/// `stats_key` names this tree in `window.__outlinerStats`.
fn virtual_tree(
    nodes: RwSignal<Vec<OutlinerNode>>,
    selected: RwSignal<Vec<String>>,
    active_layer: RwSignal<Option<String>>,
    stats_key: &'static str,
    empty_msg: &'static str,
) -> AnyView {
    // Per-tree collapse state (T-172 B6). Starts EMPTY = fully expanded, exactly the pre-collapse
    // render — the T-169 windowing smoke's totals depend on the default-expanded boot state.
    let collapsed = RwSignal::new(std::collections::HashSet::<String>::new());
    // Flatten once per doc/collapse change (O(n), like the mutation itself); the scroll path only
    // re-slices. Created ONCE per mount (this fn is called outside any reactive closure), so the
    // Effect never leaks — it re-runs on `nodes`/`collapsed` change, and the render `move ||`
    // re-slices on `rev`/scroll.
    let flat = StoredValue::new(Vec::<FlatRow>::new());
    let rev = RwSignal::new(0u64);
    Effect::new(move |_| {
        let f = collapsed.with(|c| flatten_visible(&nodes.get(), c));
        flat.set_value(f);
        rev.update(|r| *r = r.wrapping_add(1));
    });
    let scroll_top = RwSignal::new(0.0_f64);
    (move || {
        rev.track(); // re-render the slice when the tree changes
        let st = scroll_top.get();
        flat.with_value(|f| {
            let total = f.len();
            if total == 0 {
                set_outliner_stats(stats_key, 0, 0);
                return view! { <p class="text-label-sm text-outline">{empty_msg}</p> }.into_any();
            }
            if total <= VIRTUAL_SLOT_THRESHOLD {
                set_outliner_stats(stats_key, total, total);
                return view! {
                    <div>
                        {f
                            .iter()
                            .map(|r| single_row(r, selected, active_layer, collapsed))
                            .collect::<Vec<_>>()}
                    </div>
                }
                .into_any();
            }
            let per_screen = (CONTAINER_H / ROW_H).ceil() as usize;
            let start = ((st / ROW_H).floor() as usize).saturating_sub(OVERSCAN);
            let end = (start + per_screen + 2 * OVERSCAN).min(total);
            set_outliner_stats(stats_key, total, end - start);
            let top = start as f64 * ROW_H;
            let bottom = (total - end) as f64 * ROW_H;
            let rows: Vec<AnyView> = f[start..end]
                .iter()
                .map(|r| single_row(r, selected, active_layer, collapsed))
                .collect();
            view! {
                <div
                    class="overflow-y-auto"
                    style=format!("height:{CONTAINER_H}px")
                    on:scroll=move |ev| {
                        #[cfg(target_arch = "wasm32")]
                        {
                            use wasm_bindgen::JsCast;
                            if let Some(el) = ev.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()) {
                                scroll_top.set(el.scroll_top() as f64);
                            }
                        }
                        #[cfg(not(target_arch = "wasm32"))]
                        let _ = &ev;
                    }
                >
                    <div style=format!("height:{top}px")></div>
                    {rows}
                    <div style=format!("height:{bottom}px")></div>
                </div>
            }
            .into_any()
        })
    })
    .into_any()
}

/// Render the palette recursively. A leaf (`payload.is_some()`) arms a place on `pointerdown` —
/// **pointer-drag, not HTML5 DnD**: the gates drive trusted `Input.dispatchMouseEvent`, which
/// synthesizes real pointer events into these handlers, where DnD would need `Input.setInterceptDrags`.
/// The chrome host stops `pointerdown` propagation, so this press cannot also open a map gesture; the
/// release is consumed by the container's `pointerup` (see `mission_editor`).
fn palette_rows(
    nodes: &[CatalogNode],
    depth: usize,
    // T-177 A1 — the parent row's guide-continuation vector (see `guide_spans`); `&[]` at the root.
    prefix: &[bool],
    collapsed: RwSignal<std::collections::HashSet<String>>,
) -> AnyView {
    let len = nodes.len();
    nodes
        .iter()
        .enumerate()
        .map(|(i, n)| {
            let label = n.label.clone();
            let aria = n.label.clone();
            // T-177 A1 — same continuation rule as the outliner's `flatten_visible`: roots draw no
            // column; every deeper row extends its parent's vector with its own `!is_last` bit.
            let anc: Vec<bool> = if depth == 0 {
                Vec::new()
            } else {
                let mut v = Vec::with_capacity(depth);
                v.extend_from_slice(prefix);
                v.push(i + 1 != len);
                v
            };
            match n.payload.clone() {
                None => {
                    // Folder — collapsible (T-172 B6): chevron + open/closed icon; kids render
                    // only while open. The whole palette re-renders on a toggle (the DockRight
                    // closure tracks `collapsed`), so open state is read untracked here.
                    let open = !collapsed.with_untracked(|c| c.contains(&n.id));
                    let toggle =
                        chevron_or_spacer(!n.children.is_empty(), open, &n.id, collapsed);
                    let folder_icon = if open { "folder_open" } else { "folder" };
                    let kids = if open {
                        palette_rows(&n.children, depth + 1, &anc, collapsed)
                    } else {
                        ().into_any()
                    };
                    let cid = n.id.clone();
                    view! {
                        <div
                            role="button"
                            tabindex="-1"
                            aria-label=aria
                            class="relative flex cursor-pointer items-center gap-1.5 px-1.5 py-1 text-label-sm text-outline transition-colors hover:text-on-surface"
                            on:click=move |_| {
                                collapsed
                                    .update(|c| {
                                        if !c.remove(&cid) {
                                            c.insert(cid.clone());
                                        }
                                    });
                            }
                        >
                            {guide_spans(&anc)}
                            {toggle}
                            <MaterialIcon name=folder_icon class="block text-sm" />
                            <span class="truncate">{label}</span>
                        </div>
                        {kids}
                    }
                    .into_any()
                }
                // T-177 A2 — a placeable role: PALETTE_LEAF adds `cursor-grab`/`active:cursor-grabbing`
                // over ROW so hovering shows the drag affordance (folders keep `cursor-pointer`).
                Some(payload) => view! {
                    <button
                        type="button"
                        aria-label=aria
                        title="Drag onto the map to place"
                        class=PALETTE_LEAF
                        on:pointerdown=move |_| {
                            #[cfg(target_arch = "wasm32")]
                            crate::editor_ops::begin_place(payload.clone());
                            // `editor_ops` is wasm-only, so the native view shell would see an
                            // unused capture (the `announcements.rs` `let _ = store;` idiom).
                            #[cfg(not(target_arch = "wasm32"))]
                            let _ = &payload;
                        }
                    >
                        {guide_spans(&anc)}
                        <span class="size-4 shrink-0"></span>
                        <MaterialIcon name="person" class="block text-sm" />
                        <span class="truncate">{label}</span>
                    </button>
                }
                .into_any(),
            }
        })
        .collect::<Vec<_>>()
        .into_any()
}

/// Collect the folder ids whose `default_expanded` is false — the palette's initial collapsed
/// set (`buildCatalogTree` rule 3: only depth-0 faction folders start open). T-172 B6.
fn collapsed_seed(nodes: &[CatalogNode], out: &mut std::collections::HashSet<String>) {
    for n in nodes {
        if n.payload.is_none() && !n.children.is_empty() && !n.default_expanded {
            out.insert(n.id.clone());
        }
        collapsed_seed(&n.children, out);
    }
}

/// Left dock — the live **Editor Layers** outliner (spec O1). Click a folder to make it the drop
/// target, a slot to select it (no camera move — React parity).
///
/// T-177 B1 — the ORBAT browse/select tree moved OUT of this dock (the dual-tree split was bad UX)
/// into the top-strip **ORBAT Manager** modal ([`OrbatManagerDialog`], the T-071.0 cutover). Squad
/// MANAGEMENT (reparent/rename/delete) stays T-071.1+. This dock is now Editor Layers only.
#[component]
pub fn DockLeft(
    /// The Editor Layers tree, rebuilt from the doc at every mutation (`editor_ops::refresh_docks`).
    nodes: RwSignal<Vec<OutlinerNode>>,
    selected: RwSignal<Vec<String>>,
    active_layer: RwSignal<Option<String>>,
) -> impl IntoView {
    // T-172 B9 — screen-05 bottom icon strip: React's LeftSidebar BOTTOM_TABS were explicitly
    // visual-only (Hierarchy active), so present-but-disabled is the honest parity.
    let strip_btn = |icon: &'static str, label: &'static str, active: bool| {
        view! {
            <button
                type="button"
                disabled=true
                title=label
                aria-label=label
                class=if active {
                    "rounded-md p-1.5 text-primary"
                } else {
                    "rounded-md p-1.5 text-outline"
                }
            >
                <MaterialIcon name=icon class="block text-base" />
            </button>
        }
    };
    view! {
        <aside class=DOCK_L>
            <h2 class="text-label-sm font-semibold uppercase tracking-wide text-on-surface">
                "Outliner"
            </h2>
            <h2 class="mt-4 text-label-sm font-semibold uppercase tracking-wide text-on-surface-variant">
                "Editor Layers"
            </h2>
            <div class="mt-1">
                {virtual_tree(nodes, selected, active_layer, "editorLayers", "No objects placed yet.")}
            </div>
            <div class="mt-auto flex items-center justify-between border-t border-outline-variant/20 pt-2">
                {strip_btn("account_tree", "Hierarchy (visual only)", true)}
                {strip_btn("layers", "Layers (visual only)", false)}
                {strip_btn("inventory_2", "Assets (visual only)", false)}
                {strip_btn("history", "History (visual only)", false)}
                {strip_btn("settings", "Settings (visual only)", false)}
            </div>
        </aside>
    }
}

/// T-177 B2 / T-071.0 — the **ORBAT Manager** modal shell. Opened from the top-strip ORBAT Manager
/// button (`orbat_open`), it hosts the same live faction → squad → slot browse/select tree that used
/// to sit in the left dock: a slot leaf click-selects it on the map and dbl-click opens Attributes
/// (reused `single_row`), so the mission maker keeps full ORBAT visibility after the left tree's
/// removal (the B1-not-a-regression bar). Squad MANAGEMENT (create / rename / reorder, move a slot
/// between squads, slot numbering) is the next slices — **T-071.1+** — deliberately not in this shell.
#[component]
pub fn OrbatManagerDialog(
    /// Open flag, toggled by the top-strip ORBAT Manager button (Esc / backdrop close via `Dialog`).
    open: RwSignal<bool>,
    /// The ORBAT tree (faction / squad / slot), rebuilt on every mutation by `editor_ops::refresh_docks`.
    orbat: RwSignal<Vec<OutlinerNode>>,
    selected: RwSignal<Vec<String>>,
    active_layer: RwSignal<Option<String>>,
) -> impl IntoView {
    view! {
        <crate::ui::Dialog
            open
            title="ORBAT Manager"
            description="Browse factions, squads, and slots. Select a slot to highlight it on the map."
            class="max-w-xl"
        >
            <div class="min-h-40">
                {virtual_tree(
                    orbat,
                    selected,
                    active_layer,
                    "orbat",
                    "No squads yet — place a unit to build the ORBAT.",
                )}
            </div>
            <p class="mt-3 border-t border-outline-variant/20 pt-3 text-label-sm text-outline">
                "Squad management — create, rename, reorder, move slots between squads — arrives in T-071.1."
            </p>
        </crate::ui::Dialog>
    }
}

/// Right dock — the **Factions** palette (spec O2), off the live `GET /api/v1/registry`. Leaves drag
/// onto the map to place their slot. `fm_open` toggles the T-167 Faction Manager dialog.
#[component]
pub fn DockRight(catalog: RwSignal<CatalogState>, fm_open: RwSignal<bool>) -> impl IntoView {
    // Palette collapse state (T-172 B6), seeded ONCE from `default_expanded` when the catalog
    // turns Ready (only depth-0 faction folders open — screen-05 parity); user toggles stick.
    let palette_collapsed = RwSignal::new(std::collections::HashSet::<String>::new());
    let seeded = StoredValue::new(false);
    Effect::new(move |_| {
        if seeded.get_value() {
            return;
        }
        if let CatalogState::Ready(nodes) = catalog.get() {
            let mut set = std::collections::HashSet::new();
            collapsed_seed(&nodes, &mut set);
            palette_collapsed.set(set);
            seeded.set_value(true);
        }
    });
    // T-172 B9 — screen-05 palette chrome: FACTIONS / VEHICLES / MARKERS tabs + Asset Browser
    // search. Vehicles/Markers placement stays T-070/T-069 — React's tabs were stubs too, so the
    // panels say exactly that. Search filters the catalog (T-055 behavior) and force-expands
    // matches (an empty collapse set while a query is live).
    let tab = RwSignal::new(0usize);
    let search = RwSignal::new(String::new());
    let no_collapse = RwSignal::new(std::collections::HashSet::<String>::new());
    let tab_btn = move |i: usize, label: &'static str| {
        view! {
            <button
                type="button"
                class=move || {
                    if tab.get() == i {
                        "border-b-2 border-primary px-1.5 pb-1 text-label-sm font-semibold uppercase tracking-wide text-on-surface"
                    } else {
                        "border-b-2 border-transparent px-1.5 pb-1 text-label-sm font-semibold uppercase tracking-wide text-on-surface-variant transition-colors hover:text-on-surface"
                    }
                }
                on:click=move |_| tab.set(i)
            >
                {label}
            </button>
        }
    };
    view! {
        <aside class=DOCK_R>
            <div class="flex items-center justify-between">
                <div class="flex items-center gap-1">
                    {tab_btn(0, "Factions")}
                    {tab_btn(1, "Vehicles")}
                    {tab_btn(2, "Markers")}
                </div>
                <button
                    type="button"
                    aria-label="Manage factions"
                    on:click=move |_| fm_open.set(true)
                    class="rounded-md px-1.5 py-0.5 text-label-sm font-semibold uppercase tracking-wide text-primary transition-colors hover:bg-primary/15"
                >
                    "Manage"
                </button>
            </div>
            {move || match tab.get() {
                0 => view! {
                    <h3 class="mt-2 text-label-md font-semibold text-on-surface">"Asset Browser"</h3>
                    <p class="mt-0.5 text-label-sm normal-case text-outline">
                        "Drag a role onto the map to place its slot."
                    </p>
                    <input
                        type="search"
                        aria-label="Search assets"
                        placeholder="Search assets…"
                        class="mt-2 w-full rounded-md border border-outline-variant/40 bg-surface-container-lowest/60 px-2.5 py-1.5 text-label-sm text-on-surface outline-none transition-colors placeholder:text-outline focus:border-primary/60"
                        on:input=move |ev| search.set(event_target_value(&ev))
                    />
                    <div class="mt-2">
                        {move || match catalog.get() {
                            CatalogState::Loading => {
                                view! {
                                    <p class="text-label-sm text-outline">"Loading assets…"</p>
                                }
                                    .into_any()
                            }
                            CatalogState::Failed => {
                                view! {
                                    <p class="text-label-sm text-outline">
                                        "Could not load the catalog."
                                    </p>
                                }
                                    .into_any()
                            }
                            CatalogState::Ready(nodes) if nodes.is_empty() => {
                                view! {
                                    <p class="text-label-sm text-outline">"No placeable assets."</p>
                                }
                                    .into_any()
                            }
                            CatalogState::Ready(nodes) => {
                                let q = search.get();
                                if q.trim().is_empty() {
                                    // Track the collapse set so a chevron toggle re-renders the
                                    // tree (palette_rows reads it untracked).
                                    palette_collapsed.track();
                                    palette_rows(&nodes, 0, &[], palette_collapsed)
                                } else {
                                    let filtered =
                                        crate::asset_catalog::filter_catalog(&nodes, &q);
                                    if filtered.is_empty() {
                                        view! {
                                            <p class="text-label-sm text-outline">
                                                "No assets match."
                                            </p>
                                        }
                                            .into_any()
                                    } else {
                                        palette_rows(&filtered, 0, &[], no_collapse)
                                    }
                                }
                            }
                        }}
                    </div>
                }
                    .into_any(),
                1 => view! {
                    <p class="mt-3 text-label-sm normal-case text-outline">
                        "Vehicle placement lands in T-070."
                    </p>
                }
                    .into_any(),
                _ => view! {
                    <p class="mt-3 text-label-sm normal-case text-outline">
                        "Marker placement lands in T-069."
                    </p>
                }
                    .into_any(),
            }}
        </aside>
    }
}

/// Bottom Toolbelt — Select (active) + Ruler/LoS disabled stubs, then the mono CUR X/Y/Z +
/// SEL/OBJ readout.
///
/// T-172 B2/B9: Z is DEM-fed (em-dash until the grid publishes / off-coverage), and with exactly
/// one slot selected the readout swaps CUR→SEL and shows that slot's x/y/z (React parity). The
/// per-axis `title="Cursor …"` handles stay constant — they are the frozen cur-smoke's DOM hooks.
#[component]
pub fn BottomToolbelt(
    /// Cursor world position + DEM z, `None` when the pointer is off the map (em-dash cells).
    cursor: RwSignal<Option<(f64, f64, Option<f64>)>>,
    sel_count: RwSignal<usize>,
    obj_count: RwSignal<usize>,
    /// Live selection mirror — drives the CUR↔SEL swap.
    selected_ids: RwSignal<Vec<String>>,
    /// T-172 B9 — debounced compiled-payload estimate (None → `—`).
    #[prop(optional)]
    sz_bytes: Option<RwSignal<Option<usize>>>,
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
    let axis_val = move |i: usize| match sel_xyz.get() {
        Some((x, y, z)) => fmt_coord(Some([x, y, z][i])),
        None => fmt_coord(cursor.get().and_then(|c| match i {
            0 => Some(c.0),
            1 => Some(c.1),
            _ => c.2,
        })),
    };
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
            </div>
        </div>
    }
}

/// Mission Settings dialog (MissionSettingsDialog.tsx — environment half). Terrain (readonly) +
/// time / weather / view distance / thermals flow through `editor_ops::update_environment` (one
/// undo step each). The render-pref controls (map style, grid, hillshade, world-layer toggles) land
/// with the map-asset host (T-159.28) — noted in the dialog rather than shown as inert toggles.
/// Renders no DOM while closed. T-159.26.
#[component]
pub fn MissionSettingsDialog(open: RwSignal<bool>, doc_tick: RwSignal<u64>) -> impl IntoView {
    // Esc closes (the suite Dialog behavior).
    #[cfg(target_arch = "wasm32")]
    {
        let esc = window_event_listener(leptos::ev::keydown, move |ev| {
            if open.get_untracked() && ev.key() == "Escape" {
                open.set(false);
            }
        });
        on_cleanup(move || esc.remove());
    }
    let ctrl = "w-full rounded-md border border-outline-variant/40 bg-surface-container-lowest/60 px-2.5 py-1.5 text-label-md text-on-surface outline-none transition-colors focus:border-primary/60";
    move || {
        if !open.get() {
            return None;
        }
        let _ = doc_tick.get(); // re-read env on undo/redo while open
        #[cfg(target_arch = "wasm32")]
        let env = crate::editor_ops::read_env();
        #[cfg(not(target_arch = "wasm32"))]
        let env = crate::dto::MissionEnv::default();
        Some(view! {
            <div
                class="animate-overlay-fade fixed inset-0 z-50 bg-black/50 backdrop-blur-sm transition-opacity duration-200"
                on:click=move |_| open.set(false)
            ></div>
            <div class="glass animate-dialog-in fixed top-1/2 left-1/2 z-50 flex max-h-[85vh] w-[92vw] max-w-lg -translate-x-1/2 -translate-y-1/2 flex-col rounded-xl shadow-2xl outline-none transition-all duration-200">
                <div class="flex items-start justify-between gap-4 border-b border-outline-variant/30 px-6 py-4">
                    <div class="min-w-0">
                        <h2 class="text-headline-sm text-on-surface">"Mission Settings"</h2>
                        <p class="mt-1 text-label-md text-on-surface-variant">
                            "Global environment for this mission."
                        </p>
                    </div>
                    <button
                        type="button"
                        aria-label="Close"
                        on:click=move |_| open.set(false)
                        class="shrink-0 rounded-md p-1 text-outline transition-colors hover:bg-surface-variant/50 hover:text-on-surface"
                    >
                        <MaterialIcon name="close" />
                    </button>
                </div>
                <div class="custom-scrollbar flex-1 overflow-y-auto px-6 py-5">
                    <div class="flex flex-col gap-4">
                        <label class="flex flex-col gap-1">
                            <span class="text-label-sm uppercase tracking-wider text-outline">
                                "Terrain"
                            </span>
                            <div class="rounded-md border border-outline-variant/20 bg-surface-container-lowest/30 px-2.5 py-1.5 font-mono text-code-md text-on-surface-variant">
                                {env.terrain.clone()}
                            </div>
                        </label>
                        <div class="grid grid-cols-2 gap-3">
                            <label class="flex flex-col gap-1">
                                <span class="text-label-sm uppercase tracking-wider text-outline">
                                    "Time"
                                </span>
                                <input
                                    type="time"
                                    value=env.time.clone()
                                    on:input=move |ev| {
                                        #[cfg(target_arch = "wasm32")]
                                        crate::editor_ops::update_environment(
                                            serde_json::json!({ "time": event_target_value(&ev) })
                                                .to_string(),
                                        );
                                        #[cfg(not(target_arch = "wasm32"))]
                                        let _ = &ev;
                                    }
                                    class=ctrl
                                />
                            </label>
                            <label class="flex flex-col gap-1">
                                <span class="text-label-sm uppercase tracking-wider text-outline">
                                    "View Distance (m)"
                                </span>
                                <input
                                    type="number"
                                    value=env.view_distance.to_string()
                                    on:input=move |ev| {
                                        #[cfg(target_arch = "wasm32")]
                                        {
                                            let v: i64 = event_target_value(&ev).parse().unwrap_or(0);
                                            crate::editor_ops::update_environment(
                                                serde_json::json!({ "viewDistance": v }).to_string(),
                                            );
                                        }
                                        #[cfg(not(target_arch = "wasm32"))]
                                        let _ = &ev;
                                    }
                                    class=ctrl
                                />
                            </label>
                        </div>
                        <label class="flex flex-col gap-1">
                            <span class="text-label-sm uppercase tracking-wider text-outline">
                                "Weather"
                            </span>
                            <select
                                prop:value=env.weather.clone()
                                on:change=move |ev| {
                                    #[cfg(target_arch = "wasm32")]
                                    crate::editor_ops::update_environment(
                                        serde_json::json!({ "weather": event_target_value(&ev) })
                                            .to_string(),
                                    );
                                    #[cfg(not(target_arch = "wasm32"))]
                                    let _ = &ev;
                                }
                                class=ctrl
                            >
                                <option value="clear">"Clear"</option>
                                <option value="overcast">"Overcast"</option>
                                <option value="heavy_rain">"Heavy Rain"</option>
                                <option value="dense_fog">"Dense Fog"</option>
                            </select>
                        </label>
                        <div class="flex items-center justify-between py-0.5">
                            <span class="text-label-md text-on-surface-variant">"Thermals enabled"</span>
                            <input
                                type="checkbox"
                                prop:checked=env.thermals
                                on:change=move |ev| {
                                    #[cfg(target_arch = "wasm32")]
                                    {
                                        let on = event_target_checked(&ev);
                                        crate::editor_ops::update_environment(
                                            serde_json::json!({ "thermals": on }).to_string(),
                                        );
                                    }
                                    #[cfg(not(target_arch = "wasm32"))]
                                    let _ = &ev;
                                }
                                class="accent-primary"
                            />
                        </div>
                        {render_prefs_section(&env)}
                    </div>
                </div>
            </div>
        })
    }
}

/// T-173 P6 — the render-pref half of Mission Settings, restored from the React
/// `MissionSettingsDialog`: basemap view (Satellite / Map), hillshade on/off + strength slider,
/// grid, and the 12 world-layer toggles. Per-mission prefs (hillshade / grid) persist to
/// `meta.environment`; per-user prefs (basemap view + layer toggles) persist to localStorage. Each
/// control applies live to the map host (no reload). On the native view-shell these are inert
/// (no engine), which is fine — the dialog is a wasm surface.
fn render_prefs_section(env: &crate::dto::MissionEnv) -> AnyView {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = env;
        return ().into_any();
    }
    #[cfg(target_arch = "wasm32")]
    {
        use crate::world_layer_prefs as wlp;
        let hillshade_on = env.show_hillshade;
        let hillshade_pct = (env.hillshade_opacity * 100.0).round() as i64;
        let grid_on = env.show_grid;
        let basemap = wlp::load_basemap_view();
        let prefs = wlp::load_prefs();
        let sect = "text-label-sm uppercase tracking-wider text-outline";

        let layer_rows = prefs
            .rows()
            .into_iter()
            .map(|(key, on, label)| {
                view! {
                    <div class="flex items-center justify-between py-0.5">
                        <span class="text-label-md text-on-surface-variant">{label}</span>
                        <input
                            type="checkbox"
                            prop:checked=on
                            on:change=move |ev| {
                                let checked = event_target_checked(&ev);
                                let mut p = wlp::load_prefs();
                                p.set(key, checked);
                                wlp::save_prefs(&p);
                                crate::world_assets::refresh_world_layers();
                            }
                            class="accent-primary"
                        />
                    </div>
                }
            })
            .collect::<Vec<_>>();

        view! {
            <div class="mt-2 flex flex-col gap-4 border-t border-outline-variant/30 pt-4">
                <span class=sect>"Basemap"</span>
                <div class="flex gap-2">
                    {["satellite", "map"]
                        .into_iter()
                        .map(|v| {
                            let active = basemap == v;
                            let label = if v == "satellite" { "Satellite" } else { "Map" };
                            view! {
                                <button
                                    type="button"
                                    class=if active {
                                        "flex-1 rounded-md border border-primary/60 bg-primary/20 px-2.5 py-1.5 text-label-md text-primary"
                                    } else {
                                        "flex-1 rounded-md border border-outline-variant/40 bg-surface-container-lowest/60 px-2.5 py-1.5 text-label-md text-on-surface-variant transition-colors hover:border-primary/40"
                                    }
                                    on:click=move |_| {
                                        wlp::save_basemap_view(v);
                                        crate::world_assets::apply_basemap_view(v);
                                    }
                                >
                                    {label}
                                </button>
                            }
                        })
                        .collect::<Vec<_>>()}
                </div>

                <div class="flex items-center justify-between py-0.5">
                    <span class="text-label-md text-on-surface-variant">"Show hillshade"</span>
                    <input
                        type="checkbox"
                        prop:checked=hillshade_on
                        on:change=move |ev| {
                            let on = event_target_checked(&ev);
                            crate::editor_ops::update_environment(
                                serde_json::json!({ "showHillshade": on }).to_string(),
                            );
                            let op = crate::editor_ops::read_env().hillshade_opacity;
                            crate::world_assets::apply_hillshade(on, op);
                        }
                        class="accent-primary"
                    />
                </div>
                <label class="flex flex-col gap-1" class:opacity-40=move || !hillshade_on>
                    <span class=sect>{move || format!("Hillshade strength — {hillshade_pct}%")}</span>
                    <input
                        type="range"
                        min="0"
                        max="100"
                        step="1"
                        prop:disabled=!hillshade_on
                        value=hillshade_pct.to_string()
                        on:input=move |ev| {
                            let pct: f64 = event_target_value(&ev).parse().unwrap_or(40.0);
                            let op = (pct / 100.0).clamp(0.0, 1.0);
                            crate::editor_ops::update_environment(
                                serde_json::json!({ "hillshadeOpacity": op }).to_string(),
                            );
                            crate::world_assets::apply_hillshade(true, op);
                        }
                        class="accent-primary"
                    />
                </label>

                <div class="flex items-center justify-between py-0.5">
                    <span class="text-label-md text-on-surface-variant">"Grid"</span>
                    <input
                        type="checkbox"
                        prop:checked=grid_on
                        on:change=move |ev| {
                            let on = event_target_checked(&ev);
                            crate::editor_ops::update_environment(
                                serde_json::json!({ "showGrid": on }).to_string(),
                            );
                            crate::world_assets::apply_grid(on);
                        }
                        class="accent-primary"
                    />
                </div>

                <span class=sect>"World layers"</span>
                <div class="flex flex-col gap-1">{layer_rows}</div>
            </div>
        }
        .into_any()
    }
}

#[cfg(test)]
mod tests {
    use super::{hhmm_to_minutes, minutes_to_hhmm};

    #[test]
    fn time_scrubber_roundtrip() {
        assert_eq!(minutes_to_hhmm(0), "00:00");
        assert_eq!(minutes_to_hhmm(360), "06:00");
        assert_eq!(minutes_to_hhmm(1439), "23:59");
        assert_eq!(hhmm_to_minutes("06:00"), Some(360));
        assert_eq!(hhmm_to_minutes("23:59"), Some(1439));
        assert_eq!(hhmm_to_minutes("24:00"), None);
        assert_eq!(hhmm_to_minutes("nope"), None);
        for m in [0u32, 1, 59, 60, 719, 1439] {
            assert_eq!(hhmm_to_minutes(&minutes_to_hhmm(m)), Some(m));
        }
    }
}
