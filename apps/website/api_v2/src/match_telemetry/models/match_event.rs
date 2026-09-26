//! Detailed combat, medical and vehicle events of a registered match.
//!
//! **Role:** decodes and validates `POST /api/v1/ingest/match-events` batches, derives each
//! event's actor and subject identities and digest, and shapes the stored events for reads.
//! **Position:** read by `handlers::match_event_batches` and `handlers::match_event_reads`; the
//! validated batch feeds `services::match_event_batches`.
//! **Signals & state:** none; pure decoding.
//! **Invariants:** a batch holds 1–[`MAX_EVENTS_PER_BATCH`] events and is refused whole on the
//! first invalid one, named by index; `event_id` and `sequence` are unique within the batch; the
//! event digest is the SHA-256 of the canonical JSON of the event object as sent.
//! @contract match-telemetry.schema.json#/definitions/MatchEvent

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::match_registration::source_match_id;
use super::telemetry_refusal::{check_object_keys, field_named_by, invalid_entry};
use crate::core::error_handling::api_error::ApiError;
use crate::core::wire_format::content_digest::canonical_sha256;

/// Refusal code of an invalid event.
pub const INVALID_EVENT: &str = "INVALID_EVENT";
/// Refusal code of a batch above [`MAX_EVENTS_PER_BATCH`].
pub const EVENT_BATCH_TOO_LARGE: &str = "EVENT_BATCH_TOO_LARGE";
/// The most events one batch may carry.
pub const MAX_EVENTS_PER_BATCH: usize = 500;

const EVENT_KEYS: [&str; 6] = [
    "event_id",
    "sequence",
    "kind",
    "mission_time_ms",
    "occurred_at",
    "payload",
];

/// `combat.kill`: a player killed someone.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CombatKill {
    pub killer_arma_id: String,
    pub victim_arma_id: Option<String>,
    pub victim_is_player: bool,
    pub team_kill: bool,
    pub distance_m: f64,
    pub weapon: Option<String>,
}

/// `combat.death`: a player died without a player killer.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CombatDeath {
    pub victim_arma_id: String,
    pub cause: DeathCause,
}

/// What killed a player when no player did.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeathCause {
    Ai,
    Environment,
    #[serde(rename = "self")]
    SelfInflicted,
    Unknown,
}

/// `medical.incapacitated` and `medical.revived`: a player's life state changed.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MedicalSubject {
    pub subject_arma_id: String,
}

/// `vehicle.destroyed`: a vehicle reached its destroyed damage state.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VehicleDestroyed {
    pub vehicle_prefab: String,
    pub instigator_arma_id: Option<String>,
}

/// `vehicle.entered` and `vehicle.exited`: a player took or left a vehicle compartment.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VehicleCompartment {
    pub arma_id: String,
    pub vehicle_prefab: String,
    pub compartment: String,
}

/// An event's kind and its typed payload.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", content = "payload")]
pub enum EventBody {
    #[serde(rename = "combat.kill")]
    CombatKill(CombatKill),
    #[serde(rename = "combat.death")]
    CombatDeath(CombatDeath),
    #[serde(rename = "medical.incapacitated")]
    MedicalIncapacitated(MedicalSubject),
    #[serde(rename = "medical.revived")]
    MedicalRevived(MedicalSubject),
    #[serde(rename = "vehicle.destroyed")]
    VehicleDestroyed(VehicleDestroyed),
    #[serde(rename = "vehicle.entered")]
    VehicleEntered(VehicleCompartment),
    #[serde(rename = "vehicle.exited")]
    VehicleExited(VehicleCompartment),
}

#[derive(Debug, Deserialize)]
struct EventEnvelope {
    event_id: String,
    sequence: i64,
    mission_time_ms: i64,
    occurred_at: DateTime<Utc>,
    #[serde(flatten)]
    body: EventBody,
}

/// One validated event, ready to store.
#[derive(Debug, Clone)]
pub struct IncomingEvent {
    pub event_id: String,
    pub sequence: i64,
    pub kind: &'static str,
    pub mission_time_ms: i64,
    pub occurred_at: DateTime<Utc>,
    pub actor_arma_id: Option<String>,
    pub subject_arma_id: Option<String>,
    pub payload: Value,
    pub payload_sha256: String,
}

/// A validated batch of one match's events.
#[derive(Debug, Clone)]
pub struct MatchEventBatch {
    pub source_match_id: String,
    pub events: Vec<IncomingEvent>,
}

/// The answer to an event batch.
/// @contract match-telemetry.schema.json#/definitions/MatchEventBatchAnswer
#[derive(Debug, Serialize)]
pub struct MatchEventBatchAnswer {
    pub match_id: uuid::Uuid,
    pub accepted: i64,
    pub duplicates: i64,
    pub event_count: i64,
    pub last_sequence: Option<i64>,
}

fn event_refusal(
    message: impl Into<String>,
    index: Option<usize>,
    field: Option<&str>,
) -> ApiError {
    invalid_entry(INVALID_EVENT, message, index, field)
}

fn identity(value: &str, field: &str, index: usize) -> Result<String, ApiError> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.len() > 128 || trimmed != value {
        return Err(event_refusal(
            format!("{field} must be 1 to 128 bytes without surrounding space"),
            Some(index),
            Some(field),
        ));
    }
    Ok(value.to_owned())
}

fn optional_identity(
    value: &Option<String>,
    field: &str,
    index: usize,
) -> Result<Option<String>, ApiError> {
    value
        .as_deref()
        .map(|value| identity(value, field, index))
        .transpose()
}

fn bounded_name(value: &str, field: &str, max: usize, index: usize) -> Result<(), ApiError> {
    if value.trim().is_empty() || value.len() > max {
        return Err(event_refusal(
            format!("{field} must contain 1 to {max} bytes"),
            Some(index),
            Some(field),
        ));
    }
    Ok(())
}

impl EventBody {
    /// The stored kind string.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::CombatKill(_) => "combat.kill",
            Self::CombatDeath(_) => "combat.death",
            Self::MedicalIncapacitated(_) => "medical.incapacitated",
            Self::MedicalRevived(_) => "medical.revived",
            Self::VehicleDestroyed(_) => "vehicle.destroyed",
            Self::VehicleEntered(_) => "vehicle.entered",
            Self::VehicleExited(_) => "vehicle.exited",
        }
    }

    /// Validate the payload and return its (actor, subject) identities.
    fn participants(&self, index: usize) -> Result<(Option<String>, Option<String>), ApiError> {
        Ok(match self {
            Self::CombatKill(kill) => {
                if !kill.distance_m.is_finite() || kill.distance_m < 0.0 {
                    return Err(event_refusal(
                        "distance_m must be a finite number of at least 0",
                        Some(index),
                        Some("distance_m"),
                    ));
                }
                if let Some(weapon) = &kill.weapon {
                    bounded_name(weapon, "weapon", 256, index)?;
                }
                (
                    Some(identity(&kill.killer_arma_id, "killer_arma_id", index)?),
                    optional_identity(&kill.victim_arma_id, "victim_arma_id", index)?,
                )
            }
            Self::CombatDeath(death) => (
                None,
                Some(identity(&death.victim_arma_id, "victim_arma_id", index)?),
            ),
            Self::MedicalIncapacitated(medical) | Self::MedicalRevived(medical) => (
                None,
                Some(identity(
                    &medical.subject_arma_id,
                    "subject_arma_id",
                    index,
                )?),
            ),
            Self::VehicleDestroyed(vehicle) => {
                bounded_name(&vehicle.vehicle_prefab, "vehicle_prefab", 256, index)?;
                (
                    optional_identity(&vehicle.instigator_arma_id, "instigator_arma_id", index)?,
                    None,
                )
            }
            Self::VehicleEntered(seat) | Self::VehicleExited(seat) => {
                bounded_name(&seat.vehicle_prefab, "vehicle_prefab", 256, index)?;
                bounded_name(&seat.compartment, "compartment", 64, index)?;
                (Some(identity(&seat.arma_id, "arma_id", index)?), None)
            }
        })
    }
}

fn valid_event_id(event_id: &str) -> bool {
    (1..=64).contains(&event_id.len())
        && event_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._:-".contains(&byte))
}

fn decode_event(value: &Value, index: usize) -> Result<IncomingEvent, ApiError> {
    check_object_keys(value, &EVENT_KEYS, &EVENT_KEYS)
        .map_err(|(field, message)| event_refusal(message, Some(index), Some(&field)))?;
    let envelope: EventEnvelope = serde_json::from_value(value.clone()).map_err(|error| {
        let field = field_named_by(&error);
        event_refusal(
            format!("invalid event: {error}"),
            Some(index),
            field.as_deref(),
        )
    })?;
    if !valid_event_id(&envelope.event_id) {
        return Err(event_refusal(
            "event_id must match ^[A-Za-z0-9._:-]{1,64}$",
            Some(index),
            Some("event_id"),
        ));
    }
    if envelope.sequence < 1 {
        return Err(event_refusal(
            "sequence must be at least 1",
            Some(index),
            Some("sequence"),
        ));
    }
    if envelope.mission_time_ms < 0 {
        return Err(event_refusal(
            "mission_time_ms must not be negative",
            Some(index),
            Some("mission_time_ms"),
        ));
    }
    let (actor_arma_id, subject_arma_id) = envelope.body.participants(index)?;
    Ok(IncomingEvent {
        event_id: envelope.event_id,
        sequence: envelope.sequence,
        kind: envelope.body.kind(),
        mission_time_ms: envelope.mission_time_ms,
        occurred_at: envelope.occurred_at,
        actor_arma_id,
        subject_arma_id,
        payload: value["payload"].clone(),
        payload_sha256: canonical_sha256(value),
    })
}

/// Decode and validate a whole batch, refusing it on the first invalid event.
pub fn decode_event_batch(body: &Value) -> Result<MatchEventBatch, ApiError> {
    check_object_keys(
        body,
        &["source_match_id", "events"],
        &["source_match_id", "events"],
    )
    .map_err(|(field, message)| event_refusal(message, None, Some(&field)))?;
    let source_match_id = body["source_match_id"]
        .as_str()
        .ok_or_else(|| {
            event_refusal(
                "source_match_id must be a string",
                None,
                Some("source_match_id"),
            )
        })
        .and_then(|raw| {
            source_match_id(raw)
                .map_err(|message| event_refusal(message, None, Some("source_match_id")))
        })?;
    let Some(values) = body["events"].as_array() else {
        return Err(event_refusal(
            "events must be an array",
            None,
            Some("events"),
        ));
    };
    if values.len() > MAX_EVENTS_PER_BATCH {
        return Err(invalid_entry(
            EVENT_BATCH_TOO_LARGE,
            format!("a batch carries at most {MAX_EVENTS_PER_BATCH} events"),
            None,
            Some("events"),
        ));
    }
    if values.is_empty() {
        return Err(event_refusal(
            "a batch carries at least one event",
            None,
            Some("events"),
        ));
    }
    let mut ids = std::collections::HashSet::new();
    let mut sequences = std::collections::HashSet::new();
    let mut events = Vec::with_capacity(values.len());
    for (index, value) in values.iter().enumerate() {
        let event = decode_event(value, index)?;
        if !ids.insert(event.event_id.clone()) {
            return Err(event_refusal(
                "event_id repeats within the batch",
                Some(index),
                Some("event_id"),
            ));
        }
        if !sequences.insert(event.sequence) {
            return Err(event_refusal(
                "sequence repeats within the batch",
                Some(index),
                Some("sequence"),
            ));
        }
        events.push(event);
    }
    Ok(MatchEventBatch {
        source_match_id,
        events,
    })
}

#[cfg(test)]
#[path = "tests/match_event.rs"]
mod tests;
