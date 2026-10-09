//! The editor page's unit tests: the pure decisions the page and its lower crates make — boot
//! progress, the satellite level, the transform quantiser, the click router, the comment,
//! connection and marker lanes, the hover cursor, the selection universe and the registry session.
//!
//! **Role:** mounts each test file as a child of the page module, and re-exports the lower crates'
//! items those files name through `super::`.
//! **Position:** mounted at the bottom of `mission_editor.rs` under `#[cfg(test)]`.
//! **Signals & state:** none.
//! **Invariants:** test files are mounted, never imported.

use super::*;

pub(crate) use mission_creator_engine_bridge::bridge::pointer_hover::{
    HOVER_CURSOR_PICKABLE, HOVER_CURSOR_PLAIN, HOVER_RELEASE_PX, HOVER_THROTTLE_MS, HoverState,
    hover_cursor_css, hover_due, hover_next, hover_suppressed,
};
pub(crate) use mission_editing_session::lanes::comments::{
    comment_drag_lane_xy, comment_lane_xy, comment_points, dragged_comment_points, pick_comment,
};
pub(crate) use mission_editing_session::lanes::connections::{
    CONN_LINE_RGBA, CONN_LINE_SELECTED_RGBA, ConnSegment, connection_lane_verts,
    connection_segments, pick_connection,
};
pub(crate) use mission_editing_session::lanes::markers::marker_lane_fields;
pub(crate) use mission_editing_session::routing::{RouteTarget, route_availability, route_target};
pub(crate) use mission_editing_session::selection_universe::{
    crewed_slot_ids, map_render_keep_indices, plain_paste_anchor, selectable_ids,
};

#[cfg(not(target_arch = "wasm32"))]
use satellite_imagery as tbd_sat_pure;

#[path = "t245_registry_session.rs"]
mod t245_registry_session;

#[path = "t427_cold_registry_path.rs"]
mod t427_cold_registry_path;

#[path = "t628_boot_progress.rs"]
mod t628_boot_progress;

#[cfg(not(target_arch = "wasm32"))]
#[path = "t629_satellite_resolution.rs"]
mod t629_satellite_resolution;

#[path = "t631_boot_failure_state.rs"]
mod t631_boot_failure_state;

#[path = "t648_transform.rs"]
mod t648_transform;

#[path = "t669_clipboard_completion.rs"]
mod t669_clipboard_completion;

#[path = "t723_armed_place.rs"]
mod t723_armed_place;

#[path = "t754_router_resolves_zones.rs"]
mod t754_router_resolves_zones;

#[path = "t780_connection_line.rs"]
mod t780_connection_line;

#[path = "t784_comment_glyph.rs"]
mod t784_comment_glyph;

#[path = "t790_marker_glyph_caption.rs"]
mod t790_marker_glyph_caption;

#[path = "t796_comment_drag.rs"]
mod t796_comment_drag;

#[path = "t802_hover_cursor.rs"]
mod t802_hover_cursor;

#[path = "t819_crewed_render_hide.rs"]
mod t819_crewed_render_hide;

#[path = "w145_selection_prune.rs"]
mod w145_selection_prune;

#[path = "wave129_f6_probe_and_click_cannot_disagree.rs"]
mod wave129_f6_probe_and_click_cannot_disagree;
