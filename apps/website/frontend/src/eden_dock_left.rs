//! T-661 — the left dock (Editor Layers outliner), split from `eden_chrome.rs`.
//!
//! Click a folder to make it the drop target, a slot to select it. The ORBAT browse/select tree
//! moved to the top-strip ORBAT Manager modal (T-177 B1); this dock is Editor Layers only.
#![allow(dead_code)]
use leptos::prelude::*;

use crate::eden_layout::DOCK_L;
use crate::eden_tree::virtual_tree;
use crate::outliner::OutlinerNode;
use crate::ui::MaterialIcon;

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
            // T-666 — header: the title doubles as a ROOT DROPZONE (drop a dragged folder here to
            // move it to the root — `complete_layer_drop_onto_root`), plus a "+" create button
            // (LAYER-CREATE-001: new folder under the active layer, or a root when none is active).
            // The dropzone is the whole header row so it is an easy target; `pointerup` completes an
            // armed folder drag and no-ops otherwise.
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
                <h2 class="text-label-sm font-semibold uppercase tracking-wide text-on-surface">
                    "Editor Layers"
                </h2>
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
}
