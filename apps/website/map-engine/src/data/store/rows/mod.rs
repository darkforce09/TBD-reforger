//! Role: Module boundary for doc/store.
//! Position: `doc/store` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use crate::data::store::crdt::id_arrays::{
    ENTITY_IDS, SLOT_IDS, append_id, insert_empty_native, migrate_legacy_id_lists, read_field_ids,
    read_id_array, replace_native, retain_ids, retain_in,
};
use crate::data::store::crdt::soa::{
    Interner, NONE_IDX, STANCE_CROUCH, STANCE_PRONE, STANCE_STAND, SlotSoa,
};
use std::cell::{Cell, RefCell, RefMut};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use yrs::undo::UndoManager;
use yrs::{
    Any, Doc, Map, MapPrelim, MapRef, Origin, Out, ReadTxn, StateVector, Subscription,
    TransactionMut, Update,
};
mod doc_core;
use doc_core::CLIENT_ID_BITS;

/// Expose doc core ::  entity transform patch at this domain boundary.
pub use doc_core::EntityTransformPatch;
use doc_core::INIT_ORIGIN;
use doc_core::LOCAL_ORIGIN;

/// Expose doc core ::  mission doc core at this domain boundary.
pub use doc_core::MissionDocCore;

/// Expose doc core ::  squad membership at this domain boundary.
pub use doc_core::SquadMembership;
mod side_cache;
use side_cache::SideKeyMemo;
mod briefings;
mod comments;
mod compositions;
mod connections;
mod construction;
mod entities;
mod formations;
mod hydrate;
mod layers;
mod lookup;
mod materialize;
mod merge;
mod merge_json;
mod metadata;
mod paste;
mod position_rows;
mod roster;
mod slot_edits;
mod transforms;
mod triggers;
mod undo;
mod vehicles;
mod zones;
use position_rows::circle_shape_any;
use position_rows::json_position;
use position_rows::json_position_map;
use position_rows::merge_shape_rows;
use position_rows::move_entities_in_txn;
use position_rows::move_vehicles_in_txn;
use position_rows::offset_shape_any;
use position_rows::polygon_shape_any;
use position_rows::position_any;
use position_rows::position_any_merged;
use position_rows::read_composition_map;
use position_rows::read_position;
use position_rows::read_position_map;
use position_rows::update_vehicle_position_in_txn;
mod slot_rows;
use slot_rows::PASTE_KNOWN_SLOT_KEYS;
use slot_rows::ensure_leader_invariant_in_txn;
use slot_rows::garbage_collect_squad_in_txn;
use slot_rows::remove_slots_in_txn;
use slot_rows::resolve_slot_side_key;
use slot_rows::rewrite_slot_indices;
use slot_rows::set_leader_in_txn;
use slot_rows::set_slot_editor_hidden_in_txn;
use slot_rows::squad_or_faction_is_merged;
use slot_rows::update_slot_in_txn;
use slot_rows::update_slot_object_in_txn;
use slot_rows::update_slot_position_in_txn;
mod merge_report;

/// Expose merge report ::  merge opts at this domain boundary.
pub use merge_report::MergeOpts;

/// Expose merge report ::  merge report at this domain boundary.
pub use merge_report::MergeReport;
mod remint;
use remint::NamedSideIndex;
use remint::RemintMap;
mod json_rows;
use json_rows::PENDING_BRIEFING_MARKERS;
use json_rows::any_map_str;
use json_rows::any_to_f64;
use json_rows::copy_row_fields_except;
use json_rows::is_known_editor_payload_top_level;
use json_rows::json_num;
use json_rows::json_str;
use json_rows::json_str_to_any;
use json_rows::load_row;
use json_rows::load_rows_ordered;
use json_rows::ordered_rows;
use json_rows::read_any_map;
use json_rows::read_bool;
use json_rows::read_env_map;
use json_rows::read_stance;
use json_rows::read_str;
use json_rows::value_to_any;
mod layer_rows;
use layer_rows::layer_flag_effective;
use layer_rows::remove_id_from_all_layers;
use layer_rows::slot_is_transform_locked;
mod crew_rows;
use crew_rows::read_crew_map;
use crew_rows::write_crew_map;
mod comment_rows;
use comment_rows::comment_row;
use comment_rows::comment_str;
use comment_rows::comment_xz;
use comment_rows::read_comment_map;
mod connection_types;

/// Expose connection types ::  connection finding at this domain boundary.
pub use connection_types::ConnectionFinding;

/// Expose connection types ::  connection kind at this domain boundary.
pub use connection_types::ConnectionKind;

/// Expose connection types ::  connection row at this domain boundary.
pub use connection_types::ConnectionRow;
use connection_types::connection_row;
use connection_types::read_connection_map;

/// Expose connection types :: validate connection rows at this domain boundary.
pub use connection_types::validate_connection_rows;
mod formation_rows;
#[cfg(test)]
use formation_rows::FORMATION_SPACING_M;

/// Expose formation rows :: formation offsets at this domain boundary.
pub use formation_rows::formation_offsets;
mod marker_rows;
use marker_rows::briefing_markers;
use marker_rows::marker_any;
use marker_rows::marker_row_id;
use marker_rows::pending_briefing_markers_map;
use marker_rows::promote_pending_briefing_markers;
use marker_rows::remove_pending_briefing_marker;
use marker_rows::upsert_pending_briefing_marker;
mod merge_index;
#[cfg(test)]
mod tests;
