//! Toolbelt.
use camera_math::ortho::state::OrthoCamera;
#[cfg(target_arch = "wasm32")]
use leptos::prelude::*;
#[cfg(target_arch = "wasm32")]
use map_editing_tools::line_of_sight::capture::LosMode;
#[cfg(target_arch = "wasm32")]
use map_editing_tools::selection;

#[cfg(target_arch = "wasm32")]
use frontend_ui::tokens::HOVER_FILL;
#[cfg(target_arch = "wasm32")]
use frontend_ui::{MaterialIcon, cn};
#[cfg(target_arch = "wasm32")]
use mission_creator_state::layout::TOGGLED_PLATE;

#[cfg(test)]
use map_coordinates::grid_reference::GRID_STEP_M;
#[cfg(test)]
use mission_creator_state::scale_math::SCALE_MAX_PX;
#[cfg(target_arch = "wasm32")]
use mission_creator_state::scale_math::ScaleBarSpec;
use mission_creator_state::scale_math::TERRAIN_SPAN_M;
#[cfg(any(test, target_arch = "wasm32"))]
use mission_creator_state::scale_math::{format_m_per_px, m_per_px, pick_scale_bar};
mod grid_reference;
#[cfg(target_arch = "wasm32")]
pub use grid_reference::EdgeLabel;
pub use grid_reference::{edge_eastings, edge_northings};
#[cfg(test)]
pub(crate) use grid_reference::{grid_lines_in_range, grid_ref_3digit};
pub mod toolbar_and_status;
#[cfg(target_arch = "wasm32")]
pub use toolbar_and_status::{ModeToolbar, StatusBar};
pub mod map_furniture;
#[cfg(target_arch = "wasm32")]
pub use map_furniture::{MapGridRefs, ScaleBar};

#[cfg(test)]
#[path = "tests/toolbelt/furniture_geometry.rs"]
mod t667_furniture_math;

#[cfg(test)]
#[path = "tests/toolbelt/live_grid_labels.rs"]
mod t793_grid_labels_live_camera;

#[cfg(test)]
#[path = "tests/toolbelt/scale_readout.rs"]
mod t670_scale_readout;
