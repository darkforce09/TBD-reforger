//! Floating editor controls and dialogs.
#![allow(dead_code)]
use leptos::prelude::*;
#[cfg(target_arch = "wasm32")]
use website_map_engine::editing::tools::selection;

use crate::v2::apps::editor::mission_editor::transform;

mod z_drag;
pub(crate) use z_drag::{
    read_z_drag_readout, set_z_drag_readout, take_z_drag, z_drag_elevation_delta, z_drag_snap_step,
    ZDrag,
};
mod transform_widget;
pub use asset_picker::AssetPickerState;
#[cfg(target_arch = "wasm32")]
pub(crate) use transform_widget::register_widget_pivot;
pub(crate) use transform_widget::{
    read_widget_pivot, SnapReadout, TransformWidgetOverlay, WidgetModeHint,
};
mod asset_picker;
pub(crate) use asset_picker::AssetPickerOverlay;
mod comment_editor;
pub(crate) use comment_editor::CommentEditorOverlay;
mod connections_panel;
pub(crate) use connections_panel::ConnectionsPanelOverlay;
mod conflict_dialog;
pub(crate) use conflict_dialog::ConflictDialog;
pub use conflict_dialog::ConflictInfo;
#[cfg(test)]
#[path = "tests/overlays/z_arm_gesture.rs"]
mod t946_86_z_arm;
