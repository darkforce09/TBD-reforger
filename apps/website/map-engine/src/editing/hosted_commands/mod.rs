//! Role: Module boundary for the editor commands that run against the installed host.
//! Position: `editing` in the map engine.
//! Signals & state: none of its own; every entry point opens the hosted document, commits, and
//! runs the post-change tail the host installed.
//! Invariants: a caller names WHAT to change and nothing else — no document handle, no selection
//! set, no undo bookkeeping. Each entry point opens exactly one host borrow and drops it before
//! the tail runs, because that tail opens read borrows of the same document. A command that
//! changed nothing runs no tail, and a command over many rows runs exactly one for the whole set.

/// Run one mutator against the hosted document and take the post-change tail.
pub mod document_edit;

/// Read and commit a slot's editable attributes, one slot or a whole selection.
pub mod slot_attributes;

/// The searchable index of everything the document places.
pub mod document_search;

/// Cut, copy and paste over the selection.
pub mod entity_clipboard;

/// The ORBAT roster: squads, the slots in them, and the vehicles attached to them.
pub mod orbat_roster;

/// The vehicles a mission places, and who is boarded in them.
pub mod placed_vehicles;

/// The relations between placed entities, and the formation a group is forced into.
pub mod entity_connections;

/// The briefing markers a faction pins on the map.
pub mod map_markers;

/// The authored zones — the play areas and objective areas a mission declares.
pub mod zone_authoring;

/// The comments an author pins on the map.
pub mod map_comments;

/// Align, distribute, re-orient and pattern the selection, and rotate it to face a point.
pub mod selection_transform;

/// The saved-composition library: capture a selection, and edit or drop a saved row.
pub mod composition_library;

/// Move a whole selection into another faction's squad, and put it back.
pub mod squad_reassignment;

pub use slot_attributes::{
    AttrDiff, SlotAttrs, attrs_locked_count, attrs_multi_ids, attrs_update_position,
    attrs_update_position_multi, attrs_update_slot, attrs_update_slot_multi, read_attrs,
    read_attrs_diff,
};

pub use selection_transform::{
    align_selection, apply_pattern_to_selection, orient_selection, rotate_selection_to_face,
    space_selection,
};

pub use composition_library::{
    CompositionRow, composition_count, composition_rows, delete_composition,
    recategorize_composition, rename_composition, save_composition, set_composition_author,
};

pub use squad_reassignment::{ReassignTarget, reassign_rows, reassign_slots, restore_slot_squads};

pub use document_edit::commit_document_edit;

pub use document_search::{DocEntity, document_entities};

pub use entity_clipboard::{copy_selection, delete_selection, paste_at_cursor};

pub use orbat_roster::{
    OrbatManagerSnapshot, census_input, orbat_add_slot, orbat_add_squad, orbat_add_vehicle,
    orbat_apply_faction, orbat_manager_snapshot, orbat_remove_slot, orbat_remove_squad,
    orbat_rename_squad, orbat_set_leader,
};

pub use placed_vehicles::{
    PlacedSlotChoice, VehicleCargoRow, VehicleRow, assign_crew_seat, clear_crew_seat,
    crewed_slot_ids, is_vehicle_id, move_vehicles, placed_slot_choices, remove_vehicle,
    set_vehicle_cargo, set_vehicle_heading, vehicle_points, vehicle_rows,
};

pub use entity_connections::{
    ConnectionFindingRow, ConnectionListRow, OwnerOption, arm_connect, cancel_connect,
    complete_connect, connection_exists, connection_findings, connection_list, delete_connection,
    force_to_formation, pending_connect, placed_owner_options,
};

pub use map_markers::{
    MarkerRow, marker_count, marker_rows, remove_marker, set_marker_icon, set_marker_label,
    set_marker_position,
};

pub use zone_authoring::{
    ZoneRow, delete_zone, set_zone_faction, set_zone_kind, set_zone_label, set_zone_rule,
    zone_count, zone_rows,
};

pub use map_comments::{
    CommentDetail, comment_count, comment_list, delete_comment, duplicate_comment, move_comment,
    place_comment, read_comment, refile_comment_to_layer, rename_comment, set_comment_tooltip,
};
