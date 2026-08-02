//! T-661 — the left dock (Editor Layers outliner), split from `eden_chrome.rs`.
//!
//! Click a folder to make it the drop target, a slot to select it. The ORBAT browse/select tree
//! moved to the top-strip ORBAT Manager modal (T-177 B1); this dock is Editor Layers only.
//!
//! T-638 — the dock COLLAPSES to a 24×24 stub in its outer (top-LEFT) corner, toggled by the tab-strip
//! chevron or the `E` key ([`crate::mission_editor`]'s editor keydown). Collapsed is not a rail and
//! not a vanish: the panel becomes exactly the stub, docked at the corner, overlaying the map, and the
//! freed width reflows into the map pane (the inset accessors in [`crate::eden_layout`] carry it). The
//! chevron glyph points OUTWARD when expanded (« left) and FLIPS when collapsed (» — "expand me"),
//! occupying the same 24×24 box either way (Eden's mechanism, measured across the 75 screenshots).
#![allow(dead_code)]
use leptos::prelude::*;

use crate::eden_layout::{DOCK_L, STUB_PX};
use crate::eden_tree::virtual_tree;
use crate::outliner::OutlinerNode;
use crate::ui::MaterialIcon;

/// T-638 — the collapse/expand chevron shared by both docks. `outward_icon` is the glyph shown while
/// EXPANDED (points out of the dock — `chevron_left` for the left dock, `chevron_right` for the
/// right); the collapsed state shows the OTHER chevron in the SAME 24×24 box (the "flip the glyph"
/// rule). `at_start` places it at the row's start (left dock, outer corner = top-left) vs end (right
/// dock, top-right). The button flips `collapsed`; `mission_editor` observes that signal to mirror the
/// [`crate::eden_layout`] inset latch + run the reflow/centre-hold, so the chevron itself stays a pure
/// toggle.
pub fn collapse_chevron(collapsed: RwSignal<bool>, expanded_is_left: bool) -> impl IntoView {
    let title = move || {
        if collapsed.get() {
            "Expand panel"
        } else {
            "Collapse panel"
        }
    };
    // Expanded → point outward; collapsed → point inward (toward the map, i.e. "open me").
    let icon = move || match (collapsed.get(), expanded_is_left) {
        (false, true) => "chevron_left",   // left dock, expanded: « outward
        (true, true) => "chevron_right",   // left dock, collapsed: » (expand)
        (false, false) => "chevron_right", // right dock, expanded: » outward
        (true, false) => "chevron_left",   // right dock, collapsed: « (expand)
    };
    view! {
        <button
            type="button"
            aria-label=title
            aria-expanded=move || (!collapsed.get()).to_string()
            title=title
            // 24×24 hit-box (STUB_PX), matching the collapsed stub bbox exactly.
            class="flex size-6 shrink-0 cursor-pointer items-center justify-center rounded text-outline transition-colors hover:bg-white/10 hover:text-on-surface"
            on:click=move |ev: web_sys::MouseEvent| {
                ev.stop_propagation();
                collapsed.update(|c| *c = !*c);
            }
        >
            {move || view! { <MaterialIcon name=icon() class="block text-base" /> }}
        </button>
    }
}

/// Left dock — the live **Editor Layers** outliner (spec O1). Click a folder to make it the drop
/// target, a slot to select it (no camera move — React parity).
///
/// T-177 B1 — the ORBAT browse/select tree moved OUT of this dock (the dual-tree split was bad UX)
/// into the top-strip **ORBAT Manager** modal ([`OrbatManagerDialog`], the T-071.0 cutover). Squad
/// MANAGEMENT (reparent/rename/delete) stays T-071.1+. This dock is now Editor Layers only.
///
/// T-638 — `collapsed` collapses this dock to the [`STUB_PX`]-square corner stub; the `E` key and the
/// tab-strip chevron both flip it (see [`collapse_chevron`]).
#[component]
pub fn DockLeft(
    /// The Editor Layers tree, rebuilt from the doc at every mutation (`editor_ops::refresh_docks`).
    nodes: RwSignal<Vec<OutlinerNode>>,
    selected: RwSignal<Vec<String>>,
    active_layer: RwSignal<Option<String>>,
    /// T-638 — collapse latch (owned by `mission_editor`; `E`/chevron toggle it, the accessor + reflow
    /// observe it).
    collapsed: RwSignal<bool>,
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
    let full = move || {
        view! {
            <aside class=DOCK_L>
                // T-666 — header: the title doubles as a ROOT DROPZONE (drop a dragged folder here to
                // move it to the root — `complete_layer_drop_onto_root`), plus a "+" create button
                // (LAYER-CREATE-001: new folder under the active layer, or a root when none is active).
                // The dropzone is the whole header row so it is an easy target; `pointerup` completes an
                // armed folder drag and no-ops otherwise.
                // T-638 — the collapse chevron leads the row (the dock's outer top-LEFT corner, inside the
                // tab strip); « while expanded, flips to » collapsed.
                <div
                    class="group flex items-center justify-between gap-2 rounded px-1 py-0.5"
                    title="Drop a folder here to move it to the top level"
                    on:pointerup=move |ev: web_sys::PointerEvent| {
                        ev.stop_propagation();
                        #[cfg(target_arch = "wasm32")]
                        {
                            let _ = crate::editor_ops::complete_layer_drop_onto_root();
                        }
                        #[cfg(not(target_arch = "wasm32"))]
                        let _ = &ev;
                    }
                >
                    <div class="flex items-center gap-1">
                        {collapse_chevron(collapsed, true)}
                        <h2 class="text-label-sm font-semibold uppercase tracking-wide text-on-surface">
                            "Editor Layers"
                        </h2>
                    </div>
                    <button
                        type="button"
                        aria-label="New layer"
                        title="New layer (child of the selected layer)"
                        class="flex size-5 shrink-0 cursor-pointer items-center justify-center rounded text-outline transition-colors hover:bg-white/10 hover:text-on-surface"
                        on:click=move |ev: web_sys::MouseEvent| {
                            ev.stop_propagation();
                            #[cfg(target_arch = "wasm32")]
                            {
                                let _ = crate::editor_ops::create_layer();
                            }
                            #[cfg(not(target_arch = "wasm32"))]
                            let _ = &ev;
                        }
                    >
                        <MaterialIcon name="add" class="block text-base" />
                    </button>
                </div>
                // T-666 — a release that reaches this wrapper landed on NEITHER a folder row nor the
                // header root-dropzone (both `stop_propagation` + complete their own drop), so it is a
                // stray drag (released over empty tree space or a non-target slot row) — clear the latch
                // so a later click on a folder can't complete a stale reparent/refile.
                <div
                    class="mt-1"
                    on:pointerup=move |_| {
                        #[cfg(target_arch = "wasm32")]
                        crate::editor_ops::cancel_layer_drag();
                    }
                >
                    {virtual_tree(
                        nodes,
                        selected,
                        active_layer,
                        "editorLayers",
                        "No objects placed yet.",
                        false,
                        // T-666 — this is the layer-authoring tree.
                        true,
                    )}
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
    };
    // T-638 — collapsed: render ONLY the 24×24 stub (the expand chevron) at the outer top-left corner,
    // overlaying the map. Its wrapper in `mission_editor` shrinks to STUB_PX so the freed area is
    // click-through to the map. The full outliner is unmounted while collapsed.
    let stub = move || {
        view! {
            <div
                class="pointer-events-auto flex items-start bg-surface-container-lowest/55 backdrop-blur-xl"
                style=format!("width:{STUB_PX}px;height:{STUB_PX}px")
            >
                {collapse_chevron(collapsed, true)}
            </div>
        }
    };
    // T-638 — swap the whole dock for the corner stub while collapsed.
    move || {
        if collapsed.get() {
            stub().into_any()
        } else {
            full().into_any()
        }
    }
}
