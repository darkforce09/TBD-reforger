//! Role: library.
//! Position: the `apply_faction::library` module of `mission_operations`; hosted commands drive it.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use mission_document::ids::{FactionId, SquadId};
use orbat_slot_ids::SlotUid;

use super::MissionDocCore;
use super::Value;
use map_coordinates::terrain_frames::{ANCHOR, ARLAND_CENTRE};

/// Canonical slot spacing x value.
pub(super) const SLOT_SPACING_X: f64 = 15.0;

/// Apply anchor for terrain using the supplied domain data.
pub(super) fn apply_anchor_for_terrain(terrain: &str) -> (f64, f64) {
    match terrain {
        "arland" => (ARLAND_CENTRE[0], ARLAND_CENTRE[1]),

        _ => (ANCHOR[0], ANCHOR[1]),
    }
}

/// Apply anchor xy using the supplied domain data.
pub(super) fn apply_anchor_xy(doc: &MissionDocCore) -> (f64, f64) {
    let terrain = serde_json::from_str::<Value>(&doc.small_maps_json())
        .ok()
        .and_then(|root| {
            root.get("meta")
                .and_then(|m| m.get("terrain"))
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .unwrap_or_else(|| "everon".to_string());
    apply_anchor_for_terrain(&terrain)
}

/// Canonical valid sides value.
pub(super) const VALID_SIDES: &[&str] = &["BLUFOR", "OPFOR", "INDFOR"];

/// One role row from `faction-library.schema.json` (serde-owned; no FE types).
#[derive(Debug, Clone)]
pub struct FactionLibraryRole {
    /// Role.
    pub role: String,

    /// Tag.
    pub tag: Option<String>,

    /// Character.
    pub character: String,

    /// Loadout.
    pub loadout: Option<Value>,
}

/// One vehicle row from the library pool.
#[derive(Debug, Clone)]
pub struct FactionLibraryVehicle {
    /// Vehicle.
    pub vehicle: String,

    /// Label.
    pub label: Option<String>,
}

/// Library payload for [`apply_faction_library`](crate::apply_faction::apply_faction_library) (name + roles + vehicles).
#[derive(Debug, Clone)]
pub struct FactionLibraryInput {
    /// Name.
    pub name: String,

    /// Roles.
    pub roles: Vec<FactionLibraryRole>,

    /// Vehicles.
    pub vehicles: Vec<FactionLibraryVehicle>,
}

/// Result of a successful Apply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplyFactionResult {
    /// Faction id.
    pub faction_id: FactionId,

    /// Squad id.
    pub squad_id: SquadId,

    /// Leader slot id.
    pub leader_slot_id: SlotUid,

    /// Roles applied.
    pub roles_applied: usize,

    /// Vehicles applied.
    pub vehicles_applied: usize,
}

/// Domain representation of authored squad.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthoredSquad {
    /// The squad's id.
    pub id: SquadId,

    /// The label the operator sees in the ORBAT tree (falls back to [`Self::id`] when unnamed).
    pub name: String,

    /// Why it reads as authored, phrased for the operator: `"3 slots"`, `"renamed"`, ….
    pub why: String,

    /// Slots it holds — the bodies that would change hands.
    pub slots: usize,
}
