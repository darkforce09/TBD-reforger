//! The game-runtime roster wire: which player sits in which compiled slot, and every compiled
//! slot a runtime may ask to deploy a player into.

use api_identifiers::{
    ArmaPlayerId, EventId, EventMissionId, MissionId, MissionSlotUid, OrbatSlotId,
};
use serde::{Serialize, Serializer};

/// One seated player (game wire: camelCase, see [`EventRoster`]).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RosterAssignment {
    /// The seated player's Arma identity (`armaId`).
    pub arma_id: ArmaPlayerId,
    /// Compiled mission slot uid the player sits in (`slotUid`).
    pub slot_uid: MissionSlotUid,
    /// The ORBAT slot bound to that compiled slot (`orbatSlotId`).
    pub orbat_slot_id: OrbatSlotId,
    /// The event mission the seat belongs to (`eventMissionId`).
    pub event_mission_id: EventMissionId,
}

/// One compiled slot the runtime may ask to deploy a player into.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RosterSlot {
    /// The event mission the slot belongs to (`eventMissionId`).
    pub event_mission_id: EventMissionId,
    /// Compiled mission slot uid (`slotUid`).
    pub slot_uid: MissionSlotUid,
    /// The ORBAT slot bound to that compiled slot (`orbatSlotId`).
    pub orbat_slot_id: OrbatSlotId,
}

/// Roster wire version 2. The keys are camelCase and NOT the snake_case API contract:
/// Enfusion's `JsonLoadContext` binds JSON keys to class fields by name and silently ignores any
/// key the class does not declare. `missionId` is the catalog mission of the deployment the server
/// runs, empty when it runs no mission of this event. Every slot carries the `orbatSlotId` and
/// `eventMissionId` a deployment request names.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventRoster {
    /// Roster wire version; always `2`.
    pub version: u32,
    /// The event the roster belongs to (`eventId`).
    pub event_id: EventId,
    /// Catalog mission the server runs (`missionId`); `None` when it runs none of this event.
    #[serde(serialize_with = "mission_or_empty")]
    pub mission_id: Option<MissionId>,
    /// Every seated player with a linked Arma identity in the running deployment.
    pub assignments: Vec<RosterAssignment>,
    /// Every compiled slot the running deployment binds; empty when none runs.
    pub slots: Vec<RosterSlot>,
}

/// Writes the roster's mission as its id text, or `""` when the server runs no mission of the
/// event: the runtime's `JsonLoadContext` reads the key as a plain string.
fn mission_or_empty<S: Serializer>(
    mission: &Option<MissionId>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    match mission {
        Some(mission) => serializer.collect_str(mission),
        None => serializer.serialize_str(""),
    }
}

#[cfg(test)]
#[path = "tests/game_runtime_roster.rs"]
mod tests;
