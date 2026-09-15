//! Role: Module boundary for doc.
//! Position: `doc` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

/// Native arrays, row projections, and undo clocks.
pub mod crdt;

/// Headless authored-document commands.
pub mod operations;

/// Resolve graphics row handles and apply document selection policy.
pub mod picking;
mod store;

/// Faction library inputs, results, and placement operation.
pub use operations::apply_faction::{
    APPLY_ANCHOR_X, APPLY_ANCHOR_Y, ApplyFactionError, ApplyFactionResult, FactionLibraryInput,
    FactionLibraryRole, FactionLibraryVehicle, apply_faction_library,
};

/// Character placement under a faction side.
pub use operations::place_orbat::{PlaceOrbatError, place_character_under_side};

/// Slot projection and stance encoding.
pub use crdt::soa::{NONE_IDX, STANCE_CROUCH, STANCE_PRONE, STANCE_STAND, SlotSoa};

/// Mission state, connection validation, and formation commands.
pub use store::{
    ConnectionFinding, ConnectionKind, ConnectionRow, EntityTransformPatch, MissionDocCore,
    SquadMembership, formation_offsets, validate_connection_rows,
};

/// Undo timing and host clock installation.
pub use crdt::undo_groups::{GESTURE_WINDOW_MS, MAX_UNDO_GROUPS, ManualClock, install_wasm_now};

#[cfg(test)]
#[path = "tests/reexports.rs"]
mod reexport_pins;
