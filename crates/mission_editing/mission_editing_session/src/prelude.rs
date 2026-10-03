//! The names a reader of the editing session imports with
//! `use mission_editing_session::prelude::*;`.

pub use crate::batch::with_batch;
pub use crate::history::{HistoryHost, after_local_edit, install_host, redo, undo};
pub use crate::host::{
    DocHandle, EditingHost, SelectionHandle, doc_handle, install, retain_selected, selection_ids,
    selection_len, set_selection_ids, with_doc, with_doc_mut, with_host,
};
pub use crate::lanes::comments::{CommentPoint, comment_points, pick_comment};
pub use crate::lanes::connections::{ConnSegment, connection_segments, pick_connection};
pub use crate::lanes::markers::marker_lane_fields;
pub use crate::picking::{
    marquee_ids_with_vehicles, marquee_slot_ids, pick_slot, pick_slot_or_vehicle, squad_link_inputs,
};
pub use crate::routing::{RouteTarget, route_availability, route_target};
pub use crate::selection_universe::{
    crewed_slot_ids, map_render_slot_soa, plain_paste_anchor, selectable_ids,
};
