//! Toolbelt.
#![allow(dead_code)]
use leptos::prelude::*;
use website_map_engine::camera::ortho::state::OrthoCamera;
use website_map_engine::editing::tools::line_of_sight::capture::LosMode;
#[cfg(target_arch = "wasm32")]
use website_map_engine::editing::tools::selection;

use crate::v2::apps::editor::shell::layout::{HOVER_FILL, TOGGLED_PLATE};
use crate::v2::core::ui::{cn, MaterialIcon};

mod scale_math;
pub use scale_math::{
    format_distance, format_m_per_px, m_per_px, pick_scale_bar, ScaleBarSpec, GRID_STEP_M,
    SCALE_MAX_PX, SCALE_MIN_PX, TERRAIN_SPAN_M,
};
mod grid_reference;
pub use grid_reference::{
    edge_eastings, edge_northings, grid_lines_in_range, grid_ref_3digit, EdgeLabel,
};
mod toolbar_and_status;
use toolbar_and_status::{fmt_coord, fmt_coord_eden, STATUSBAR};
pub use toolbar_and_status::{ModeToolbar, StatusBar, STATUSBAR_H_PX};
mod map_furniture;
pub use map_furniture::{MapGridRefs, ScaleBar};
mod bottom_toolbelt;
pub use bottom_toolbelt::BottomToolbelt;

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

#[cfg(test)]
#[path = "tests/toolbelt/source.rs"]
mod test_source;
