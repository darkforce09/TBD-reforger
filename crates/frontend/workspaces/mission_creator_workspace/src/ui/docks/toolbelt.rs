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
#[cfg(test)]
use mission_creator_state::layout::STATUSBAR_H_PX;
#[cfg(target_arch = "wasm32")]
pub use toolbar_and_status::{ModeToolbar, StatusBar};
#[cfg(test)]
use toolbar_and_status::{STATUSBAR, fmt_coord, fmt_coord_eden};
pub mod map_furniture;
#[cfg(target_arch = "wasm32")]
pub use map_furniture::{MapGridRefs, ScaleBar};

#[cfg(test)]
#[path = "tests/toolbelt/status_bar.rs"]
mod t636_status_bar;

#[cfg(test)]
#[path = "tests/toolbelt/ruler_controls.rs"]
mod t642_ruler;

#[cfg(test)]
#[path = "tests/toolbelt/furniture_geometry.rs"]
mod t667_furniture_math;

#[cfg(test)]
#[path = "tests/toolbelt/live_grid_labels.rs"]
mod t793_grid_labels_live_camera;

#[cfg(test)]
#[path = "tests/toolbelt/toolbar_state.rs"]
mod t668_state_vocabulary;

#[cfg(test)]
#[path = "tests/toolbelt/scale_readout.rs"]
mod t670_scale_readout;

/// The grid-reference exporter is pinned against the map furniture's own edge labels, which only
/// the frontend draws, so the pin sits beside the furniture, on the side of the wall that can read
/// both.
#[cfg(test)]
#[path = "tests/toolbelt/exporter_grid_reference.rs"]
mod exporter_grid_reference_tests;

#[cfg(test)]
#[path = "tests/toolbelt/source.rs"]
mod test_source;
