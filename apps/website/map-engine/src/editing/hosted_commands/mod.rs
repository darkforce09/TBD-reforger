//! Role: Module boundary for the editor commands that run against the installed host.
//! Position: `editing` in the map engine.
//! Signals & state: none of its own; every entry point opens the hosted document, commits, and
//! runs the post-change tail the host installed.
//! Invariants: a caller names WHAT to change and nothing else — no document handle, no selection
//! set, no undo bookkeeping. Each entry point opens exactly one host borrow and drops it before
//! the tail runs, because that tail opens read borrows of the same document. A command that
//! changed nothing runs no tail, and a command over many rows runs exactly one for the whole set.

/// Read and commit a slot's editable attributes, one slot or a whole selection.
pub mod slot_attributes;

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
