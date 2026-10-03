//! Floating editor controls and dialogs.
use leptos::prelude::*;
#[cfg(target_arch = "wasm32")]
use map_editing_tools::selection;

use crate::workspaces::editor::mission_editor::transform;

mod z_drag;
#[cfg(target_arch = "wasm32")]
pub(crate) use z_drag::{read_z_drag_readout, set_z_drag_readout};
pub(crate) use z_drag::{take_z_drag, z_drag_elevation_delta, z_drag_snap_step, ZDrag};
mod transform_widget;
#[cfg(target_arch = "wasm32")]
pub use asset_picker::AssetPickerState;
#[cfg(target_arch = "wasm32")]
pub(crate) use transform_widget::read_widget_pivot;
#[cfg(target_arch = "wasm32")]
pub(crate) use transform_widget::register_widget_pivot;
#[cfg(target_arch = "wasm32")]
pub(crate) use transform_widget::{SnapReadout, TransformWidgetOverlay, WidgetModeHint};
mod asset_picker;
#[cfg(target_arch = "wasm32")]
pub(crate) use asset_picker::AssetPickerOverlay;
mod comment_editor;
#[cfg(target_arch = "wasm32")]
pub(crate) use comment_editor::CommentEditorOverlay;
mod connections_panel;
#[cfg(target_arch = "wasm32")]
pub(crate) use connections_panel::ConnectionsPanelOverlay;
mod conflict_dialog;
#[cfg(target_arch = "wasm32")]
pub(crate) use conflict_dialog::ConflictDialog;
#[cfg(target_arch = "wasm32")]
pub use conflict_dialog::ConflictInfo;
#[cfg(test)]
#[path = "tests/overlays/z_arm_gesture.rs"]
mod t946_86_z_arm;
