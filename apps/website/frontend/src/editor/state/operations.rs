//! T-159.22 — the dock commands: outliner select / active layer, and palette drag-to-place.
//!
//! Peer of `mission_history` / `mission_commands`, and the same shape for the same reason: the doc /
//! engine / selection handles are `!Send` wasm-only `Rc`s that can't cross the
//! `#[cfg(target_arch = "wasm32")]` boundary into the native view shell, so the dock buttons reach
//! them through a `thread_local` [`OpsCtx`] set from `mission_editor::on_load` — exactly how the
//! Undo button reaches the undo stack.
//!
//! **Placement (T-180.1):** each `place_at` calls
//! [`map_engine_core::doc::place_character_under_side`] under [`OpsCtx::active_side`] (default
//! `BLUFOR`), which ensures `faction-{SIDE}`, mints a **new** squad, adds the slot as sole member /
//! leader, and files it under the resolved layer ([`ensure_layer`]). Layer mint stays LOCAL so it is
//! **undoable** — a boot-time layer would break the save/export gate (`smoke_save_export_editor`
//! uses the seed only). The ORBAT tree derives from squads (`build_orbat`). Seed slots still carry a
//! dangling `squadId` with no squad in the map — they list under Unfiled until placed-through.
//!
//! Consequence: the **first** place is multiple undo steps (layer + faction + squad + slot + leader
//! are separate core transactions); every later place under an existing layer/faction is fewer.
//!
//! **Borrow discipline** (the `mission_history` rule): each `pub fn` opens exactly one `OPS_CTX`
//! borrow; doc `borrow_mut`s are scoped so they drop before `mission_history::after_local_edit`
//! opens its read borrows.
#![cfg(target_arch = "wasm32")]

// T-934.7 — `operations.rs` is now a FAÇADE: the module body was split into submodules
// below (same-commit mechanical move; bodies unchanged). Every public item is re-exported so the
// `crate::editor::state::operations::X` paths (and the `editor_ops` aliases) keep working, and the
// whole tree stays wasm32-gated by the `#![cfg]` above. The Class-R/S source guards that pinned
// patterns in this file now `include_str!` the submodule that holds their pattern.
// NOTE (T-934.6): the `use crate::editor::state::history as mission_history;` alias lives in each
// submodule so the `mission_history::…` guard needles stay stable across the move.

pub mod attrs;
pub mod batch;
pub mod cargo;
pub mod compositions;
pub mod context;
pub mod entity;
// T-939.2 — the Attributes modal's batch faction / squad reassign. Its own module rather than more
// of `entity.rs` for the reason `tactical_graphics` gives below — `entity.rs` is contested — and
// because this is the one mutator that must never take `entity::refile_slot`'s core path: refile
// GCs an emptied source squad (row, `faction.squadIds` place, attached vehicles), and a batch
// reassign must not.
pub mod reassign;
// T-936.7 — the `tacticalGraphics[]` mutators (draw / select / vertex drag / delete). Its own
// module rather than more of `entity.rs` because a control measure is not an entity: it has no
// slot id, no layer, no squad and no SoA row, and `entity.rs` is contested by five other tickets.
pub mod tactical_graphics;
pub mod transform;

pub use attrs::*;
pub use batch::*;
pub use cargo::*;
pub use compositions::*;
pub use context::*;
pub use entity::{
    CommentDetail, ConnectionFindingRow, ConnectionListRow, MarkerRow, OrbatManagerSnapshot,
    OrbatSlotDetail, OwnerOption, PlacedSlotChoice, TRIGGER_ACTIVATIONS, TriggerRow,
    VehicleCargoRow, VehicleRow, ZoneRow, add_whole_terrain_zone, arm_connect,
    armed_composition_id, armed_marker_icon, assign_crew_seat, begin_layer_comment_drag,
    begin_layer_drag, begin_layer_slot_drag, begin_place, begin_place_composition,
    begin_place_marker, begin_place_object, begin_place_vehicle, begin_refile, begin_zone_draw,
    begin_zone_reshape, cancel_connect, cancel_layer_drag, cancel_pending, cancel_refile,
    cancel_zone_draw, census_input, center_on_selection, clear_crew_seat, close_zone_polygon,
    comment_count, comment_details, comment_list, complete_connect,
    complete_layer_drop_onto_folder, complete_layer_drop_onto_root, complete_refile_onto_squad,
    connection_exists, connection_findings, connection_list, copy_selection, create_layer,
    crewed_slot_ids, debug_seed_slots, delete_comment, delete_connection, delete_layer,
    delete_trigger, delete_zone, document_entities, duplicate_comment, faction_doc_from_side,
    force_to_formation, has_pending, hide_selection, is_vehicle_id, marker_count, marker_rows,
    move_comment, move_vehicles, orbat_add_slot, orbat_add_squad, orbat_add_vehicle,
    orbat_apply_faction, orbat_manager_snapshot, orbat_remove_slot, orbat_remove_squad,
    orbat_rename_squad, orbat_set_leader, orbat_update_slot_fields, owner_line_world,
    pending_connect, place_at, place_at_alt, place_at_keep, place_comment, placed_owner_options,
    placed_slot_choices, read_comment, refile_comment_to_layer, refile_slot, refile_slot_to_layer,
    regroup_slot_onto, remove_marker, remove_vehicle, rename_comment, rename_layer, reparent_layer,
    select_all_in_view, select_layer_children, select_layer_descendants, select_slot,
    selection_entities, selection_len, set_active_layer, set_comment_tooltip, set_layer_hidden,
    set_layer_locked, set_marker_icon, set_marker_label, set_marker_position, set_selection_ids,
    set_trigger_activation, set_trigger_name, set_trigger_owner, set_trigger_rule,
    set_vehicle_cargo, set_vehicle_heading, set_zone_faction, set_zone_kind, set_zone_label,
    set_zone_rule, show_all_hidden, show_selection, slot_hidden_rows, take_rename_armed,
    toggle_hidden, trigger_count, trigger_rows, vehicle_points, vehicle_rows, zone_count,
    zone_draft, zone_draw_armed, zone_draw_pop_vertex, zone_rows,
};
pub use reassign::*;
// T-936.7 — a GLOB, like `attrs`/`batch`/`cargo`/`compositions`/`context` above and unlike the
// hand-listed `entity`/`transform` below. The named form warns on every item the wasm bin does not
// yet call (`begin_tactical_draw`, `tactical_draft`, `tactical_draw_pop_vertex`, … — the draw
// tool's API, which has no arming affordance until a `panels/` surface grows one), and a
// `#[allow]` on a `pub use` would suppress a real signal rather than the false one.
pub use tactical_graphics::*;
pub use transform::{
    apply_pattern_to_selection, orient_selection, rotate_selection_to_face, space_selection,
};
pub mod slot_ids;
pub use slot_ids::*;
