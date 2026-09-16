//! Role: Module boundary for doc/operations/entity.
//! Position: `doc/operations/entity` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use crate::data::store::MissionDocCore;
use crate::data::store::NONE_IDX;
use crate::data::store::operations::assets::PlacePayload;
use crate::data::store::operations::attrs::slot_z;
use crate::data::store::operations::compositions::{
    composition_entities_json, composition_entity_count, composition_title,
};
use crate::data::store::operations::faction_library::{FactionDoc, FactionRole, FactionVehicle};
use crate::data::store::operations::projections::slot_rows;
use crate::data::store::operations::projections::{faction_rows, layer_rows, squad_rows};
use crate::data::store::operations::rows::CommentRow;
use crate::data::store::operations::zones::DrawTarget;
use crate::data::store::{APPLY_ANCHOR_X, APPLY_ANCHOR_Y};
use crate::data::store::{
    FactionLibraryInput, FactionLibraryRole, FactionLibraryVehicle, apply_faction_library,
};
mod selection;
/// Expose selection :: { selected slot ids , selection all hidden , selection centroid , set selection hidden , slot hidden rows , terrain bounds of , terrain key of , toggle hidden , } at this domain boundary.
pub use selection::{
    selected_slot_ids, selection_all_hidden, selection_centroid, set_selection_hidden,
    slot_hidden_rows, terrain_bounds_of, terrain_key_of, toggle_hidden,
};
mod identity;
/// Expose identity :: { live slot ids , mint id , mint ids , mint layer id , mint layer name , slot attrs exists , } at this domain boundary.
pub use identity::{
    live_slot_ids, mint_id, mint_ids, mint_layer_id, mint_layer_name, slot_attrs_exists,
};
mod comments;
/// Expose comments :: {  comment detail , comment details , comment rows , duplicate comment , mint comment id , place comment , } at this domain boundary.
pub use comments::{
    CommentDetail, comment_details, comment_rows, duplicate_comment, mint_comment_id, place_comment,
};
mod connections;
/// Expose connections :: {  connection finding row ,  connection list row , complete connect , connection findings , connection id in doc , connection list , delete connection , mint connection id , } at this domain boundary.
pub use connections::{
    ConnectionFindingRow, ConnectionListRow, complete_connect, connection_findings,
    connection_id_in_doc, connection_list, delete_connection, mint_connection_id,
};
mod roster;
/// Expose roster :: { orbat slot spacing x ,  orbat manager snapshot ,  orbat slot detail ,  placed slot choice , asset id for role , ensure side faction , mint squad id for side , next slot xy , orbat add squad , orbat manager snapshot , orbat remove slot , orbat update slot fields , placed slot choices , side faction id , slot details , slot xy , squad anchor in , squad anchor xy , } at this domain boundary.
pub use roster::{
    ORBAT_SLOT_SPACING_X, OrbatManagerSnapshot, OrbatSlotDetail, PlacedSlotChoice,
    asset_id_for_role, ensure_side_faction, mint_squad_id_for_side, next_slot_xy, orbat_add_squad,
    orbat_manager_snapshot, orbat_remove_slot, orbat_update_slot_fields, placed_slot_choices,
    side_faction_id, slot_details, slot_xy, squad_anchor_in, squad_anchor_xy,
};
mod vehicles;
/// Expose vehicles :: {  vehicle cargo row ,  vehicle row , place vehicle in core , placed entity pos , placed owner options , set vehicle cargo , set vehicle heading , vehicle rows , } at this domain boundary.
pub use vehicles::{
    VehicleCargoRow, VehicleRow, place_vehicle_in_core, placed_entity_pos, placed_owner_options,
    set_vehicle_cargo, set_vehicle_heading, vehicle_rows,
};
mod markers;
/// Expose markers :: {  marker row , marker rows of , mint marker id } at this domain boundary.
pub use markers::{MarkerRow, marker_rows_of, mint_marker_id};
mod zones;
/// Expose zones :: {  owner option , trigger activations ,  trigger row ,  zone row , mint row id , set trigger rule , set zone rule , trigger rows , write row returning id , zone rows , } at this domain boundary.
pub use zones::{
    OwnerOption, TRIGGER_ACTIVATIONS, TriggerRow, ZoneRow, mint_row_id, set_trigger_rule,
    set_zone_rule, trigger_rows, write_row_returning_id, zone_rows,
};
mod clipboard;
/// Expose clipboard :: { copy selection , delete selection , paste at cursor , place saved composition } at this domain boundary.
pub use clipboard::{copy_selection, delete_selection, paste_at_cursor, place_saved_composition};
mod placement;
/// Expose placement :: { orbat add slot , orbat add vehicle , place object in core } at this domain boundary.
pub use placement::{orbat_add_slot, orbat_add_vehicle, place_object_in_core};
mod factions;
/// Expose factions :: { faction doc from side core , orbat apply faction } at this domain boundary.
pub use factions::{faction_doc_from_side_core, orbat_apply_faction};
