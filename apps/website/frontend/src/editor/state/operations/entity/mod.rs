//! Role: Module boundary for editor/state/operations/entity.
//! Position: `editor/state/operations/entity` in the frontend editor adapter.
//! Signals & state: host signals, input state, and explicit map-engine `data::store` calls.
//! Invariants: preserve input routing, borrow lifetimes, and post-edit refresh order.

#[allow(unused_imports)]
use super::{cargo::*, context::*};
use crate::editor::arsenal::asset_catalog::PlacePayload;
use crate::editor::eden_chrome::{
    circle_from_clicks, polygon_flat, polygon_is_committable, zone_types, ZoneShape,
};

use crate::editor::panels::zones_panel::DrawTarget;
use crate::editor::state::history as mission_history;
use crate::v2::core::api::dto::FactionDoc;
use leptos::prelude::{GetUntracked, Set};
use website_map_engine::data::store::place_character_under_side;
use website_map_engine::data::store::MissionDocCore;

/// Expose website mission core :: doc :: operations :: entity :: comment details at this domain boundary.
pub use website_map_engine::data::store::operations::entity::comment_details;

/// Expose website mission core :: doc :: operations :: entity :: comment rows at this domain boundary.
pub(super) use website_map_engine::data::store::operations::entity::comment_rows;

/// Expose website mission core :: doc :: operations :: entity :: connection id in doc at this domain boundary.
pub(super) use website_map_engine::data::store::operations::entity::connection_id_in_doc;

use website_map_engine::data::store::operations::entity::faction_doc_from_side_core;

use website_map_engine::data::store::operations::entity::mint_marker_id;

use website_map_engine::data::store::operations::entity::place_object_in_core;
use website_map_engine::data::store::operations::entity::place_vehicle_in_core;

/// Expose website mission core :: doc :: operations :: entity :: selected slot ids at this domain boundary.
pub(super) use website_map_engine::data::store::operations::entity::selected_slot_ids;

use website_map_engine::data::store::operations::entity::side_faction_id;

/// Expose website mission core :: doc :: operations :: entity :: slot hidden rows at this domain boundary.
pub use website_map_engine::data::store::operations::entity::slot_hidden_rows;

/// Expose website mission core :: doc :: operations :: entity :: terrain bounds of at this domain boundary.
pub(super) use website_map_engine::data::store::operations::entity::terrain_bounds_of;

/// Expose website mission core :: doc :: operations :: entity :: terrain key of at this domain boundary.
pub(super) use website_map_engine::data::store::operations::entity::terrain_key_of;

/// Expose website mission core :: doc :: operations :: entity ::  marker row at this domain boundary.
pub use website_map_engine::data::store::operations::entity::MarkerRow;

/// Expose website mission core :: doc :: operations :: entity ::  trigger row at this domain boundary.
pub use website_map_engine::data::store::operations::entity::TriggerRow;

/// Expose website mission core :: doc :: operations :: entity :: trigger activations at this domain boundary.
pub use website_map_engine::data::store::operations::entity::TRIGGER_ACTIVATIONS;
mod layers;

/// Expose layers :: { create layer , delete layer , hide selection , refile slot to layer , rename layer , reparent layer , set active layer , set layer hidden , set layer locked , show all hidden , show selection , take rename armed , toggle hidden , } at this domain boundary.
pub use layers::{
    create_layer, delete_layer, hide_selection, refile_slot_to_layer, rename_layer, reparent_layer,
    set_active_layer, set_layer_hidden, set_layer_locked, show_all_hidden, show_selection,
    take_rename_armed, toggle_hidden,
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

/// Expose arming :: mint id at this domain boundary.
pub(super) use arming::mint_id;

/// Expose arming :: { armed composition id , armed marker icon , begin place , begin place composition , begin place marker , begin place object , begin place vehicle , cancel pending , debug seed slots , ensure active layer , has pending , selection len , } at this domain boundary.
pub use arming::{
    armed_composition_id, armed_marker_icon, begin_place, begin_place_composition,
    begin_place_marker, begin_place_object, begin_place_vehicle, cancel_pending, debug_seed_slots,
    ensure_active_layer, has_pending, selection_len,
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

/// Expose zone draw :: { add whole terrain zone , begin zone draw , begin zone reshape , cancel zone draw , close zone polygon , zone draft , zone draw armed , zone draw pop vertex , } at this domain boundary.
pub use zone_draw::{
    add_whole_terrain_zone, begin_zone_draw, begin_zone_reshape, cancel_zone_draw,
    close_zone_polygon, zone_draft, zone_draw_armed, zone_draw_pop_vertex,
};
mod triggers;

/// Expose triggers :: { delete trigger , owner line world , set trigger activation , set trigger name , set trigger owner , set trigger rule , trigger count , trigger rows , } at this domain boundary.
pub use triggers::{
    delete_trigger, owner_line_world, set_trigger_activation, set_trigger_name, set_trigger_owner,
    set_trigger_rule, trigger_count, trigger_rows,
};

mod selection_index;

/// Expose selection index :: { center on selection , select all in view , select slot , selection entities , set selection ids , } at this domain boundary.
pub use selection_index::{
    center_on_selection, select_all_in_view, select_slot, selection_entities, set_selection_ids,
};
