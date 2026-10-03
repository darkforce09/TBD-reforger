//! Toolbelt.
use camera_math::ortho::state::OrthoCamera;
#[cfg(target_arch = "wasm32")]
use leptos::prelude::*;
#[cfg(target_arch = "wasm32")]
use map_engine::editing::tools::line_of_sight::capture::LosMode;
#[cfg(target_arch = "wasm32")]
use map_engine::editing::tools::selection;

#[cfg(target_arch = "wasm32")]
use crate::foundation::ui::{cn, MaterialIcon};
#[cfg(target_arch = "wasm32")]
use crate::workspaces::editor::session::layout::{HOVER_FILL, TOGGLED_PLATE};

mod scale_math;
#[cfg(target_arch = "wasm32")]
pub use scale_math::ScaleBarSpec;
pub use scale_math::{format_m_per_px, m_per_px, pick_scale_bar, TERRAIN_SPAN_M};
#[cfg(test)]
pub use scale_math::{GRID_STEP_M, SCALE_MAX_PX};
mod grid_reference;
#[cfg(target_arch = "wasm32")]
pub use grid_reference::EdgeLabel;
pub use grid_reference::{edge_eastings, edge_northings};
#[cfg(test)]
pub use grid_reference::{grid_lines_in_range, grid_ref_3digit};
mod toolbar_and_status;
#[cfg(test)]
pub use toolbar_and_status::STATUSBAR_H_PX;
#[cfg(test)]
use toolbar_and_status::{fmt_coord, fmt_coord_eden, STATUSBAR};
#[cfg(target_arch = "wasm32")]
pub use toolbar_and_status::{ModeToolbar, StatusBar};
mod map_furniture;
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

#[cfg(test)]
#[path = "tests/toolbelt/source.rs"]
mod test_source;
