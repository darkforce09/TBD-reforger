//! A match's detailed events as `GET /api/v1/matches/{matchId}/events` pages them.
//!
//! **Role:** the page envelope, one stored event, and the payload of each of the seven event
//! kinds as a typed variant selected by `kind`.
//! **Position:** deserialised straight from the backend's JSON; re-serialised unchanged by the
//! round-trip tests.
//! **Signals & state:** none — these are plain data.
//! **Invariants:** items arrive in ascending `sequence` and are kept in that order;
//! `next_after_sequence` is null on the last page and the key is always present. `kind` and
//! `payload` travel as sibling keys, so the event's kind and its payload shape cannot disagree
//! once decoded. `distance_m` keeps the number exactly as sent (whole metres stay integers), so a
//! page re-serialises to the bytes it was read from.
//! @contract match-telemetry.schema.json#/definitions/MatchEvent

use serde::{Deserialize, Serialize};
use serde_json::Number;

/// One page of a match's events.
/// @contract match-telemetry.schema.json#
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MatchEventPageDto {
    pub items: Vec<MatchEventDto>,
    /// The `after_sequence` that reads the next page; null on the last page.
    pub next_after_sequence: Option<i64>,
}

/// One stored event.
/// @contract match-telemetry.schema.json#/definitions/MatchEvent
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MatchEventDto {
    /// Unique within the match; assigned by the game runtime.
    pub event_id: String,
    /// Capture order within the match, from 1.
    pub sequence: i64,
    /// Milliseconds since the mission started.
    pub mission_time_ms: i64,
    /// RFC 3339 UTC.
    pub occurred_at: String,
    #[serde(flatten)]
    pub detail: MatchEventDetail,
}

/// The event's kind with its payload.
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "payload")]
pub enum MatchEventDetail {
    /// @contract match-telemetry.schema.json#/definitions/CombatKillPayload
    #[serde(rename = "combat.kill")]
    CombatKill(CombatKillPayloadDto),
    /// @contract match-telemetry.schema.json#/definitions/CombatDeathPayload
    #[serde(rename = "combat.death")]
    CombatDeath(CombatDeathPayloadDto),
    /// @contract match-telemetry.schema.json#/definitions/MedicalIncapacitatedPayload
    #[serde(rename = "medical.incapacitated")]
    MedicalIncapacitated(MedicalSubjectPayloadDto),
    /// @contract match-telemetry.schema.json#/definitions/MedicalRevivedPayload
    #[serde(rename = "medical.revived")]
    MedicalRevived(MedicalSubjectPayloadDto),
    /// @contract match-telemetry.schema.json#/definitions/VehicleDestroyedPayload
    #[serde(rename = "vehicle.destroyed")]
    VehicleDestroyed(VehicleDestroyedPayloadDto),
    /// @contract match-telemetry.schema.json#/definitions/VehicleEnteredPayload
    #[serde(rename = "vehicle.entered")]
    VehicleEntered(VehicleCrewPayloadDto),
    /// @contract match-telemetry.schema.json#/definitions/VehicleExitedPayload
    #[serde(rename = "vehicle.exited")]
    VehicleExited(VehicleCrewPayloadDto),
}

/// A kill: the killer, the victim when known, and how it happened.
/// @contract match-telemetry.schema.json#/definitions/CombatKillPayload
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CombatKillPayloadDto {
    pub killer_arma_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub victim_arma_id: Option<String>,
    pub victim_is_player: bool,
    pub team_kill: bool,
    /// Metres, at least 0, kept as the number the backend sent.
    pub distance_m: Number,
    /// The weapon prefab, when the game runtime resolved one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weapon: Option<String>,
}

/// A death with no player killer.
/// @contract match-telemetry.schema.json#/definitions/CombatDeathPayload
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CombatDeathPayloadDto {
    pub victim_arma_id: String,
    pub cause: DeathCause,
}

/// What caused a death without a player killer.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeathCause {
    Ai,
    Environment,
    /// Self-inflicted.
    #[serde(rename = "self")]
    SelfInflicted,
    Unknown,
}

/// The subject of a medical event (incapacitated or revived).
/// @contract match-telemetry.schema.json#/definitions/MedicalIncapacitatedPayload
/// @contract match-telemetry.schema.json#/definitions/MedicalRevivedPayload
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MedicalSubjectPayloadDto {
    pub subject_arma_id: String,
}

/// A destroyed vehicle and, when known, who destroyed it.
/// @contract match-telemetry.schema.json#/definitions/VehicleDestroyedPayload
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VehicleDestroyedPayloadDto {
    pub vehicle_prefab: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instigator_arma_id: Option<String>,
}

/// A player entering or leaving a vehicle compartment.
/// @contract match-telemetry.schema.json#/definitions/VehicleEnteredPayload
/// @contract match-telemetry.schema.json#/definitions/VehicleExitedPayload
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VehicleCrewPayloadDto {
    pub arma_id: String,
    pub vehicle_prefab: String,
    /// `pilot`, `turret`, `cargo` or `other` from the TBD mod; any 1–64 byte name is accepted.
    pub compartment: String,
}
