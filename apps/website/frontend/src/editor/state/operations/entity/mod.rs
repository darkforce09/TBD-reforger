//! Role: Module boundary for editor/state/operations/entity.
//! Position: `editor/state/operations/entity` in the frontend editor adapter.
//! Signals & state: host signals, input state, and explicit mission-core calls.
//! Invariants: preserve input routing, borrow lifetimes, and post-edit refresh order.

thread_local! {

    pub(super) static CLIPBOARD: RefCell<Vec<serde_json::Value>> = const { RefCell::new(Vec::new()) };
}
thread_local! {

    static NEXT_LAYER_ID: Cell<u32> = const { Cell::new(0) };

    static RENAME_ARMED: RefCell<Option<String>> = const { RefCell::new(None) };

    static PENDING_LAYER_DRAG: RefCell<Option<LayerDrag>> = const { RefCell::new(None) };
}
thread_local! {

    static PENDING_CONNECT: RefCell<Option<(String, String)>> = const { RefCell::new(None) };
}
thread_local! {

    static PENDING_REFILE: RefCell<Option<String>> = const { RefCell::new(None) };
}
#[allow(unused_imports)]
use super::{attrs::*, cargo::*, compositions::*, context::*, transform::*};
use crate::editor::arsenal::asset_catalog::PlacePayload;
use crate::editor::eden_chrome::{
    circle_from_clicks, polygon_flat, polygon_is_committable, zone_types, ZoneShape,
};

use crate::editor::panels::zones_panel::DrawTarget;
use crate::editor::state::history as mission_history;
use crate::v2::core::api::dto::FactionDoc;
use leptos::prelude::{GetUntracked, Set};
use std::cell::{Cell, RefCell};
use website_mission_core::doc::place_character_under_side;
use website_mission_core::doc::MissionDocCore;

/// Expose website mission core :: doc :: operations :: entity :: comment details at this domain boundary.
pub use website_mission_core::doc::operations::entity::comment_details;

/// Expose website mission core :: doc :: operations :: entity :: comment rows at this domain boundary.
pub(super) use website_mission_core::doc::operations::entity::comment_rows;

/// Expose website mission core :: doc :: operations :: entity :: connection id in doc at this domain boundary.
pub(super) use website_mission_core::doc::operations::entity::connection_id_in_doc;

use website_mission_core::doc::operations::entity::faction_doc_from_side_core;

use website_mission_core::doc::operations::entity::marker_rows_of;

use website_mission_core::doc::operations::entity::mint_layer_id;
use website_mission_core::doc::operations::entity::mint_layer_name;
use website_mission_core::doc::operations::entity::mint_marker_id;

use website_mission_core::doc::operations::entity::place_object_in_core;
use website_mission_core::doc::operations::entity::place_vehicle_in_core;

/// Expose website mission core :: doc :: operations :: entity :: selected slot ids at this domain boundary.
pub(super) use website_mission_core::doc::operations::entity::selected_slot_ids;

use website_mission_core::doc::operations::entity::side_faction_id;

/// Expose website mission core :: doc :: operations :: entity :: slot hidden rows at this domain boundary.
pub use website_mission_core::doc::operations::entity::slot_hidden_rows;

/// Expose website mission core :: doc :: operations :: entity :: terrain bounds of at this domain boundary.
pub(super) use website_mission_core::doc::operations::entity::terrain_bounds_of;

/// Expose website mission core :: doc :: operations :: entity :: terrain key of at this domain boundary.
pub(super) use website_mission_core::doc::operations::entity::terrain_key_of;

/// Expose website mission core :: doc :: operations :: entity ::  comment detail at this domain boundary.
pub use website_mission_core::doc::operations::entity::CommentDetail;

/// Expose website mission core :: doc :: operations :: entity ::  connection finding row at this domain boundary.
pub use website_mission_core::doc::operations::entity::ConnectionFindingRow;

/// Expose website mission core :: doc :: operations :: entity ::  connection list row at this domain boundary.
pub use website_mission_core::doc::operations::entity::ConnectionListRow;

/// Expose website mission core :: doc :: operations :: entity ::  marker row at this domain boundary.
pub use website_mission_core::doc::operations::entity::MarkerRow;

/// Expose website mission core :: doc :: operations :: entity ::  orbat manager snapshot at this domain boundary.
pub use website_mission_core::doc::operations::entity::OrbatManagerSnapshot;

/// Expose website mission core :: doc :: operations :: entity ::  orbat slot detail at this domain boundary.
pub use website_mission_core::doc::operations::entity::OrbatSlotDetail;

/// Expose website mission core :: doc :: operations :: entity ::  owner option at this domain boundary.
pub use website_mission_core::doc::operations::entity::OwnerOption;

/// Expose website mission core :: doc :: operations :: entity ::  placed slot choice at this domain boundary.
pub use website_mission_core::doc::operations::entity::PlacedSlotChoice;

/// Expose website mission core :: doc :: operations :: entity ::  trigger row at this domain boundary.
pub use website_mission_core::doc::operations::entity::TriggerRow;

/// Expose website mission core :: doc :: operations :: entity ::  vehicle cargo row at this domain boundary.
pub use website_mission_core::doc::operations::entity::VehicleCargoRow;

/// Expose website mission core :: doc :: operations :: entity ::  vehicle row at this domain boundary.
pub use website_mission_core::doc::operations::entity::VehicleRow;

/// Expose website mission core :: doc :: operations :: entity ::  zone row at this domain boundary.
pub use website_mission_core::doc::operations::entity::ZoneRow;

/// Expose website mission core :: doc :: operations :: entity :: trigger activations at this domain boundary.
pub use website_mission_core::doc::operations::entity::TRIGGER_ACTIVATIONS;
mod selection;

/// Expose selection :: { center on selection , copy selection , delete selection , document entities , paste at cursor , placed owner options , read comment , select all in view , select slot , } at this domain boundary.
pub use selection::{
    center_on_selection, copy_selection, delete_selection, document_entities, paste_at_cursor,
    placed_owner_options, read_comment, select_all_in_view, select_slot,
};
use selection::{DEFAULT_LAYER_ID, DEFAULT_LAYER_NAME};
mod layers;
use layers::LayerDrag;

/// Expose layers :: { create layer , delete layer , hide selection , refile slot to layer , rename layer , reparent layer , set active layer , set layer hidden , set layer locked , show all hidden , show selection , take rename armed , toggle hidden , } at this domain boundary.
pub use layers::{
    create_layer, delete_layer, hide_selection, refile_slot_to_layer, rename_layer, reparent_layer,
    set_active_layer, set_layer_hidden, set_layer_locked, show_all_hidden, show_selection,
    take_rename_armed, toggle_hidden,
};
mod comments;

/// Expose comments :: { comment count , comment list , delete comment , duplicate comment , move comment , place comment , refile comment to layer , rename comment , set comment tooltip , } at this domain boundary.
pub use comments::{
    comment_count, comment_list, delete_comment, duplicate_comment, move_comment, place_comment,
    refile_comment_to_layer, rename_comment, set_comment_tooltip,
};
mod connections;

/// Expose connections :: { arm connect , cancel connect , complete connect , connection exists , connection findings , connection list , delete connection , force to formation , pending connect , } at this domain boundary.
pub use connections::{
    arm_connect, cancel_connect, complete_connect, connection_exists, connection_findings,
    connection_list, delete_connection, force_to_formation, pending_connect,
};
mod layer_drag;
use layer_drag::set_slot_selection;

/// Expose layer drag :: { begin layer comment drag , begin layer drag , begin layer slot drag , cancel layer drag , complete layer drop onto folder , complete layer drop onto root , select layer children , select layer descendants , } at this domain boundary.
pub use layer_drag::{
    begin_layer_comment_drag, begin_layer_drag, begin_layer_slot_drag, cancel_layer_drag,
    complete_layer_drop_onto_folder, complete_layer_drop_onto_root, select_layer_children,
    select_layer_descendants,
};
mod arming;
use arming::ensure_layer;

/// Expose arming :: { arm , mint id } at this domain boundary.
pub(super) use arming::{arm, mint_id};

/// Expose arming :: { armed composition id , begin place , begin place composition , begin place object , begin place vehicle , cancel pending , debug seed slots , has pending , selection len , } at this domain boundary.
pub use arming::{
    armed_composition_id, begin_place, begin_place_composition, begin_place_object,
    begin_place_vehicle, cancel_pending, debug_seed_slots, has_pending, selection_len,
};
mod roster;

/// Expose roster :: { census input , orbat add slot , orbat add squad , orbat add vehicle , orbat apply faction , orbat manager snapshot , orbat remove slot , orbat remove squad , orbat rename squad , orbat set leader , } at this domain boundary.
pub use roster::{
    census_input, orbat_add_slot, orbat_add_squad, orbat_add_vehicle, orbat_apply_faction,
    orbat_manager_snapshot, orbat_remove_slot, orbat_remove_squad, orbat_rename_squad,
    orbat_set_leader,
};
mod vehicles;

/// Expose vehicles :: { assign crew seat , clear crew seat , crewed slot ids , is vehicle id , move vehicles , placed slot choices , remove vehicle , set vehicle cargo , set vehicle heading , vehicle points , vehicle rows , } at this domain boundary.
pub use vehicles::{
    assign_crew_seat, clear_crew_seat, crewed_slot_ids, is_vehicle_id, move_vehicles,
    placed_slot_choices, remove_vehicle, set_vehicle_cargo, set_vehicle_heading, vehicle_points,
    vehicle_rows,
};
mod refile;

/// Expose refile :: { begin refile , cancel refile , complete refile onto squad , faction doc from side , orbat update slot fields , refile slot , } at this domain boundary.
pub use refile::{
    begin_refile, cancel_refile, complete_refile_onto_squad, faction_doc_from_side,
    orbat_update_slot_fields, refile_slot,
};
mod placement;

/// Expose placement :: { place at , place at alt , place at keep , regroup slot onto } at this domain boundary.
pub use placement::{place_at, place_at_alt, place_at_keep, regroup_slot_onto};

mod zone_draw;

/// Expose zone draw :: advance zone draw at this domain boundary.
pub(super) use zone_draw::advance_zone_draw;
use zone_draw::write_row_returning_id;

/// Expose zone draw :: { begin zone draw , begin zone reshape , cancel zone draw , close zone polygon , zone draft , zone draw armed , zone draw pop vertex , } at this domain boundary.
pub use zone_draw::{
    begin_zone_draw, begin_zone_reshape, cancel_zone_draw, close_zone_polygon, zone_draft,
    zone_draw_armed, zone_draw_pop_vertex,
};
mod zones;
use zones::edit_zone;

/// Expose zones :: { add whole terrain zone , delete zone , set zone faction , set zone kind , set zone label , set zone rule , zone count , zone rows , } at this domain boundary.
pub use zones::{
    add_whole_terrain_zone, delete_zone, set_zone_faction, set_zone_kind, set_zone_label,
    set_zone_rule, zone_count, zone_rows,
};
mod triggers;

/// Expose triggers :: { delete trigger , owner line world , set trigger activation , set trigger name , set trigger owner , set trigger rule , trigger count , trigger rows , } at this domain boundary.
pub use triggers::{
    delete_trigger, owner_line_world, set_trigger_activation, set_trigger_name, set_trigger_owner,
    set_trigger_rule, trigger_count, trigger_rows,
};

mod markers;

/// Expose markers :: { armed marker icon , begin place marker , marker count , marker rows , remove marker , set marker icon , set marker label , set marker position , } at this domain boundary.
pub use markers::{
    armed_marker_icon, begin_place_marker, marker_count, marker_rows, remove_marker,
    set_marker_icon, set_marker_label, set_marker_position,
};
mod selection_index;

/// Expose selection index :: { selection entities , set selection ids } at this domain boundary.
pub use selection_index::{selection_entities, set_selection_ids};
