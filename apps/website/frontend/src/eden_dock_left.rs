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
            <h2 class="text-label-sm font-semibold uppercase tracking-wide text-on-surface">
                "Editor Layers"
            </h2>
            <div class="mt-1">
                {virtual_tree(
                    nodes,
                    selected,
                    active_layer,
                    "editorLayers",
                    "No objects placed yet.",
                    false,
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
