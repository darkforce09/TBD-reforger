//! Floating editor controls and dialogs.
use leptos::prelude::*;
#[cfg(target_arch = "wasm32")]
use map_editing_tools::selection;

#[cfg(any(target_arch = "wasm32", test))]
use mission_creator_state::transform;

mod z_drag;
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) use z_drag::{ZDrag, take_z_drag, z_drag_elevation_delta, z_drag_snap_step};
#[cfg(target_arch = "wasm32")]
pub(crate) use z_drag::{read_z_drag_readout, set_z_drag_readout};
/// The transform widget, its mode hint and snap readout, and the widget-pivot registry the gizmo
/// reads.
pub mod transform_widget;
#[cfg(target_arch = "wasm32")]
pub use asset_picker::AssetPickerState;
#[cfg(target_arch = "wasm32")]
pub(crate) use transform_widget::read_widget_pivot;
#[cfg(target_arch = "wasm32")]
pub use transform_widget::register_widget_pivot;
#[cfg(target_arch = "wasm32")]
pub use transform_widget::{SnapReadout, TransformWidgetOverlay, WidgetModeHint};
/// The empty-ground asset picker and the map and screen anchors it opens at.
pub mod asset_picker;
#[cfg(target_arch = "wasm32")]
pub use asset_picker::AssetPickerOverlay;
/// The comment editor laid over a placed map comment.
pub mod comment_editor;
#[cfg(target_arch = "wasm32")]
pub use comment_editor::CommentEditorOverlay;
/// The connections panel listing the selected entity's links.
pub mod connections_panel;
#[cfg(target_arch = "wasm32")]
pub use connections_panel::ConnectionsPanelOverlay;
#[cfg(test)]
#[path = "tests/overlays/z_arm_gesture.rs"]
mod t946_86_z_arm;
