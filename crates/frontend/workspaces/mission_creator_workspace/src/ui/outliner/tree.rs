//! Shared dock-tree rendering: guides, windowed rows, and row actions.
//!
//! `virtual_tree` is the windowed outliner both docks draw with; `guide_spans` / `chevron_or_spacer`
//! and the row-class recipes are shared by the outliner (`single_row`) and the palette
//! (`eden_dock_right::palette_rows`). Not cfg-gated (the doc-driving `on:click` bodies are wasm-gated
//! inside their closures).
#[cfg(target_arch = "wasm32")]
use leptos::prelude::*;

#[cfg(target_arch = "wasm32")]
use frontend_ui::MaterialIcon;
#[cfg(target_arch = "wasm32")]
use mission_creator_engine_bridge::bridge::host_state::entity_selection;
#[cfg(target_arch = "wasm32")]
use mission_creator_state::outliner_model::{FlatRow, VIRTUAL_SLOT_THRESHOLD, flatten_visible};
#[cfg(any(test, target_arch = "wasm32"))]
use mission_creator_state::outliner_model::{NodeKind, OutlinerNode};
#[cfg(target_arch = "wasm32")]
use mission_editing_commands::hosted_commands as engine_ops;
#[cfg(test)]
use mission_operations::rows::LayerRow;

mod comment_row;
mod row_actions;
mod row_geometry;
mod selection;
mod single_row;
mod vehicle_rows;
mod virtual_tree;

#[cfg(target_arch = "wasm32")]
use comment_row::*;
#[cfg(test)]
use mission_creator_state::outliner_model::{layer_descendant_slots, layer_direct_slot_children};
#[cfg(test)]
pub(crate) use row_actions::row_router_subject;
#[cfg(target_arch = "wasm32")]
use row_actions::*;
#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) use row_actions::{inert_row_reason, row_routes};
#[cfg(test)]
pub(crate) use row_geometry::ROW_GEOM;
#[cfg(any(test, target_arch = "wasm32"))]
use row_geometry::*;
#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) use row_geometry::{
    PALETTE_LEAF, ROW, ROW_ACTIVE, ROW_BADGE, ROW_FACTION, ROW_STATIC, ROW_UNFILED,
};
#[cfg(target_arch = "wasm32")]
pub(crate) use row_geometry::{chevron_or_spacer, guide_spans};
#[cfg(test)]
pub(crate) use selection::folder_holds_slots;
#[cfg(any(test, target_arch = "wasm32"))]
use selection::*;
#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) use selection::{drag_set_for, node_descendant_ids};
#[cfg(target_arch = "wasm32")]
use single_row::*;
#[cfg(target_arch = "wasm32")]
use vehicle_rows::*;
#[cfg(target_arch = "wasm32")]
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
