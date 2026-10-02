//! The game-runtime roster wire: which player sits in which compiled slot, and every compiled
//! slot a runtime may ask to deploy a player into.

use serde::Serialize;
use uuid::Uuid;

/// One seated player (game wire: camelCase, see [`EventRoster`]).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RosterAssignment {
    pub arma_id: String,
    pub slot_uid: String,
    pub orbat_slot_id: Uuid,
    pub event_mission_id: Uuid,
}

/// One compiled slot the runtime may ask to deploy a player into.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RosterSlot {
    pub event_mission_id: Uuid,
    pub slot_uid: String,
    pub orbat_slot_id: Uuid,
}

/// Roster wire version 2. The keys are camelCase and NOT the snake_case API contract:
/// Enfusion's `JsonLoadContext` binds JSON keys to class fields by name and silently ignores any
/// key the class does not declare. `missionId` is the catalog mission of the deployment the server
/// runs, empty when it runs no mission of this event. Every slot carries the `orbatSlotId` and
/// `eventMissionId` a deployment request names.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventRoster {
    pub version: u32,
    pub event_id: Uuid,
    pub mission_id: String,
    pub assignments: Vec<RosterAssignment>,
    pub slots: Vec<RosterSlot>,
}
