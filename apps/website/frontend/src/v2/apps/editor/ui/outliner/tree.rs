//! Shared dock-tree rendering: guides, windowed rows, and row actions.
//!
//! `virtual_tree` is the windowed outliner both docks draw with; `guide_spans` / `chevron_or_spacer`
//! and the row-class recipes are shared by the outliner (`single_row`) and the palette
//! (`eden_dock_right::palette_rows`). Not cfg-gated (the doc-driving `on:click` bodies are wasm-gated
//! inside their closures).
#![allow(dead_code)]
use leptos::prelude::*;

#[cfg(target_arch = "wasm32")]
use crate::v2::apps::editor::bridge::host_state::entity_selection;
#[cfg(target_arch = "wasm32")]
use crate::v2::apps::editor::ui::outliner::outliner;
use crate::v2::apps::editor::ui::outliner::outliner::{
    flatten_visible, FlatRow, LayerRow, NodeKind, OutlinerNode, VIRTUAL_SLOT_THRESHOLD,
};
use crate::v2::core::ui::MaterialIcon;
use website_map_engine::editing::hosted_commands as engine_ops;

mod comment_row;
mod row_actions;
mod row_geometry;
mod selection;
mod single_row;
mod vehicle_rows;
mod virtual_tree;

use comment_row::*;
use row_actions::*;
pub(crate) use row_actions::{inert_row_reason, row_router_subject, row_routes};
use row_geometry::*;
pub(crate) use row_geometry::{
    chevron_or_spacer, guide_spans, PALETTE_LEAF, ROW, ROW_ACTIVE, ROW_BADGE, ROW_FACTION,
    ROW_GEOM, ROW_STATIC, ROW_UNFILED,
};
use selection::*;
pub(crate) use selection::{
    drag_set_for, folder_holds_slots, layer_descendant_slots, layer_direct_slot_children,
    node_descendant_ids,
};
use single_row::*;
use vehicle_rows::*;
pub(crate) use virtual_tree::virtual_tree;

#[cfg(test)]
/// Live tree production source for source-inspection tests.
pub(super) const TREE_PRODUCTION_SOURCE: &str = concat!(
    include_str!("tree/selection.rs"),
    include_str!("tree/row_geometry.rs"),
    include_str!("tree/row_actions.rs"),
    include_str!("tree/comment_row.rs"),
    include_str!("tree/single_row.rs"),
    include_str!("tree/vehicle_rows.rs"),
    include_str!("tree/virtual_tree.rs"),
);

#[cfg(test)]
#[path = "tests/tree/selection_and_layer_authoring.rs"]
mod selection_and_layer_authoring;

#[cfg(test)]
#[path = "tests/tree/dense_row_geometry.rs"]
mod dense_row_geometry;

#[cfg(test)]
#[path = "tests/tree/comment_row_routing.rs"]
mod comment_row_routing;

#[cfg(test)]
#[path = "tests/tree/multi_entity_drag_and_drop.rs"]
mod multi_entity_drag_and_drop;
