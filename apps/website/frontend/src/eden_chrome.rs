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
    /// T-159.26 — unsaved-changes flag; a `•` after the title marks dirty (React's `isDirty` dot).
    #[prop(optional)]
    dirty: Option<RwSignal<bool>>,
    /// T-159.26 — the Mission Settings dialog's open flag (gear button toggles it).
    #[prop(optional)]
    settings_open: Option<RwSignal<bool>>,
) -> impl IntoView {
    view! {
        <div class=STRIP>
            <span class="min-w-0 flex-1 truncate text-label-md font-semibold text-on-surface">
                {title}
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
const ROW: &str = "flex w-full items-center gap-1.5 rounded px-1.5 py-1 text-left text-label-sm text-on-surface-variant transition-colors hover:bg-white/10 hover:text-on-surface";
const ROW_ACTIVE: &str = "flex w-full items-center gap-1.5 rounded px-1.5 py-1 text-left text-label-sm transition-colors bg-primary/20 text-primary";

/// One `w-3` guide span per depth level — the hierarchy guide lines (T-172 B7). `self-stretch`
/// makes the border span the full row height inside the `items-center` flex row.
fn guide_spans(depth: usize) -> AnyView {
    (0..depth)
        .map(|_| {
            view! { <span class="w-3 shrink-0 self-stretch border-l border-white/10"></span> }
        })
        .collect::<Vec<_>>()
        .into_any()
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
    let depth = row.depth;
    // Static per build — a chevron toggle bumps `collapsed`, which re-flattens + re-renders
    // the slice (the virtual_tree Effect tracks it), so open state never goes stale.
    let open = !collapsed.with_untracked(|c| c.contains(&row.id));
    let toggle = chevron_or_spacer(row.has_children, open, &row.id, collapsed);
    match row.kind {
        NodeKind::Unfiled => view! {
            <div class="flex items-center gap-1.5 px-1.5 py-1 text-label-sm text-outline">
                {guide_spans(depth)}
                {toggle}
                <MaterialIcon name="inbox" class="block text-sm" />
                <span>{label}</span>
            </div>
        }
        .into_any(),
        NodeKind::Faction => view! {
            <div class="flex items-center gap-1.5 px-1.5 py-1 text-label-sm font-semibold uppercase tracking-wide text-on-surface-variant">
                {guide_spans(depth)}
                {toggle}
                <MaterialIcon name="flag" class="block text-sm" />
                <span class="truncate">{label}</span>
            </div>
        }
        .into_any(),
        NodeKind::Squad => view! {
            <div class="flex items-center gap-1.5 px-1.5 py-1 text-label-sm text-on-surface-variant">
                {guide_spans(depth)}
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
                    {guide_spans(depth)}
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
                    {guide_spans(depth)}
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
    collapsed: RwSignal<std::collections::HashSet<String>>,
) -> AnyView {
    nodes
        .iter()
        .map(|n| {
            let label = n.label.clone();
            let aria = n.label.clone();
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
                        palette_rows(&n.children, depth + 1, collapsed)
                    } else {
                        ().into_any()
                    };
                    let cid = n.id.clone();
                    view! {
                        <div
                            role="button"
                            tabindex="-1"
                            aria-label=aria
                            class="flex cursor-pointer items-center gap-1.5 px-1.5 py-1 text-label-sm text-outline transition-colors hover:text-on-surface"
                            on:click=move |_| {
                                collapsed
                                    .update(|c| {
                                        if !c.remove(&cid) {
                                            c.insert(cid.clone());
                                        }
                                    });
                            }
                        >
                            {guide_spans(depth)}
                            {toggle}
                            <MaterialIcon name=folder_icon class="block text-sm" />
                            <span class="truncate">{label}</span>
                        </div>
                        {kids}
                    }
                    .into_any()
                }
                Some(payload) => view! {
                    <button
                        type="button"
                        aria-label=aria
                        title="Drag onto the map to place"
                        class=ROW
                        on:pointerdown=move |_| {
                            #[cfg(target_arch = "wasm32")]
                            crate::editor_ops::begin_place(payload.clone());
                            // `editor_ops` is wasm-only, so the native view shell would see an
                            // unused capture (the `announcements.rs` `let _ = store;` idiom).
                            #[cfg(not(target_arch = "wasm32"))]
                            let _ = &payload;
                        }
                    >
                        {guide_spans(depth)}
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
/// T-168 — the ORBAT browse/select tree (faction → squad → slot) is live; squad MANAGEMENT
/// (reparent/rename/delete) stays T-071. Slot leaves click-select + dbl-click → Attributes.
#[component]
pub fn DockLeft(
    /// The Editor Layers tree, rebuilt from the doc at every mutation (`editor_ops::refresh_docks`).
    nodes: RwSignal<Vec<OutlinerNode>>,
    /// T-168 — the ORBAT tree (faction/squad/slot), rebuilt alongside `nodes`.
    orbat: RwSignal<Vec<OutlinerNode>>,
    selected: RwSignal<Vec<String>>,
    active_layer: RwSignal<Option<String>>,
) -> impl IntoView {
    view! {
        <aside class=DOCK_L>
            <h2 class="text-label-sm font-semibold uppercase tracking-wide text-on-surface-variant">
                "ORBAT"
            </h2>
            <div class="mt-1">
                {virtual_tree(orbat, selected, active_layer, "orbat", "No squads yet — place a unit to build the ORBAT.")}
            </div>
            <h2 class="mt-4 text-label-sm font-semibold uppercase tracking-wide text-on-surface-variant">
                "Editor Layers"
            </h2>
            <div class="mt-1">
                {virtual_tree(nodes, selected, active_layer, "editorLayers", "No objects placed yet.")}
            </div>
        </aside>
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
    view! {
        <aside class=DOCK_R>
            <div class="flex items-center justify-between">
                <h2 class="text-label-sm font-semibold uppercase tracking-wide text-on-surface-variant">
                    "Factions"
                </h2>
                <button
                    type="button"
                    aria-label="Manage factions"
                    on:click=move |_| fm_open.set(true)
                    class="rounded-md px-1.5 py-0.5 text-label-sm font-semibold uppercase tracking-wide text-primary transition-colors hover:bg-primary/15"
                >
                    "Manage"
                </button>
            </div>
            <p class="mt-1 text-label-sm normal-case text-outline">
                "Drag a role onto the map to place its slot."
            </p>
            <div class="mt-2">
                {move || match catalog.get() {
                    CatalogState::Loading => {
                        view! { <p class="text-label-sm text-outline">"Loading assets…"</p> }
                            .into_any()
                    }
                    CatalogState::Failed => {
                        view! {
                            <p class="text-label-sm text-outline">"Could not load the catalog."</p>
                        }
                            .into_any()
                    }
                    CatalogState::Ready(nodes) if nodes.is_empty() => {
                        view! { <p class="text-label-sm text-outline">"No placeable assets."</p> }
                            .into_any()
                    }
                    CatalogState::Ready(nodes) => {
                        // Track the collapse set so a chevron toggle re-renders the tree
                        // (palette_rows reads it untracked).
                        palette_collapsed.track();
                        palette_rows(&nodes, 0, palette_collapsed)
                    }
                }}
            </div>
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
                        <p class="mt-2 text-label-sm normal-case text-outline">
                            "Map style, grid, hillshade, and world-layer toggles arrive with the terrain render host."
                        </p>
                    </div>
                </div>
            </div>
        })
    }
}
