//! Role: what may be armed for placement under the current authoring mode, and the debug seed.
//! Position: `doc/operations/entity` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; the armed value itself is the host's to keep.
//! Invariants: the gate is read at ARM time, so a leftover arm from a mode the author has since
//! left is dropped rather than committed under the new mode.

use std::cell::Cell;

use super::MissionDocCore;
use super::mint_id;
use super::place_character_under_side;

/// Which collection an armed placement commits into. It carries no payload: the gate below reads
/// only the kind, and the payload belongs to whoever is holding the arm.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArmedPlacementKind {
    /// A character, placed under a side's order of battle.
    Character,

    /// A vehicle.
    Vehicle,

    /// A world object.
    Object,

    /// A saved composition.
    Composition,

    /// A map marker.
    Marker,

    /// A zone or trigger being drawn.
    Zone,
}

/// May `kind` be armed while the authoring surface is (or is not) in objects mode?
///
/// Objects mode accepts world objects and nothing else that is placed on a side; the side modes
/// accept characters and vehicles and refuse objects. Compositions and markers belong to neither
/// mode and are always armable. A zone is never ARMED this way — a draw is begun, not picked up —
/// so arming one is refused outright.
#[must_use]
pub fn placement_is_armable(kind: ArmedPlacementKind, objects_mode: bool) -> bool {
    match kind {
        ArmedPlacementKind::Object => objects_mode,
        ArmedPlacementKind::Character | ArmedPlacementKind::Vehicle => !objects_mode,
        ArmedPlacementKind::Composition | ArmedPlacementKind::Marker => true,
        ArmedPlacementKind::Zone => false,
    }
}

/// Place `count` riflemen under BLUFOR in `layer_id`, each on a freshly minted id, at the world
/// origin. A diagnostic population for a document that needs slots in it, not an authoring path:
/// nothing here reads a payload or a position.
pub fn seed_debug_slots(core: &MissionDocCore, next_id: &Cell<u32>, layer_id: &str, count: u32) {
    for _ in 0..count {
        let id = mint_id(core, next_id);
        let _ = place_character_under_side(
            core, "BLUFOR", &id, layer_id, "Rifleman", None, None, 0.0, 0.0, 0.0, 0.0,
        );
    }
}

#[cfg(test)]
#[path = "tests/arming.rs"]
mod tests;
