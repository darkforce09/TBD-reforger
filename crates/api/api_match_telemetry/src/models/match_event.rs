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

use api_identifiers::{ArmaPlayerId, MatchEventId, MatchId, SourceMatchId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::match_registration::source_match_id;
use super::telemetry_refusal::{check_object_keys, field_named_by, invalid_entry};
use api_foundation::error_handling::api_error::ApiError;
use api_foundation::wire_format::content_digest::canonical_sha256;

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
    /// The player who killed.
    pub killer_arma_id: ArmaPlayerId,
    /// The victim, when it has an Arma identity.
    pub victim_arma_id: Option<ArmaPlayerId>,
    /// Whether the victim is a player rather than AI.
    pub victim_is_player: bool,
    /// Whether killer and victim are on the same side.
    pub team_kill: bool,
    /// Distance between killer and victim, in metres.
    pub distance_m: f64,
    /// The weapon the kill was made with, when the runtime names one.
    pub weapon: Option<String>,
}

/// `combat.death`: a player died without a player killer.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CombatDeath {
    /// The player who died.
    pub victim_arma_id: ArmaPlayerId,
    /// What killed the player.
    pub cause: DeathCause,
}

/// What killed a player when no player did.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeathCause {
    /// An AI unit (`ai`).
    Ai,
    /// The environment: a fall, a collision, drowning (`environment`).
    Environment,
    /// The player's own action (`self`).
    #[serde(rename = "self")]
    SelfInflicted,
    /// A cause the runtime could not classify (`unknown`).
    Unknown,
}

/// `medical.incapacitated` and `medical.revived`: a player's life state changed.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MedicalSubject {
    /// The player whose life state changed.
    pub subject_arma_id: ArmaPlayerId,
}

/// `vehicle.destroyed`: a vehicle reached its destroyed damage state.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VehicleDestroyed {
    /// The prefab of the destroyed vehicle.
    pub vehicle_prefab: String,
    /// The player who destroyed it, when one did.
    pub instigator_arma_id: Option<ArmaPlayerId>,
}

/// `vehicle.entered` and `vehicle.exited`: a player took or left a vehicle compartment.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VehicleCompartment {
    /// The player who took or left the compartment.
    pub arma_id: ArmaPlayerId,
    /// The prefab of the vehicle.
    pub vehicle_prefab: String,
    /// The compartment taken or left.
    pub compartment: String,
}

/// An event's kind and its typed payload.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", content = "payload")]
pub enum EventBody {
    /// `combat.kill`.
    #[serde(rename = "combat.kill")]
    CombatKill(CombatKill),
    /// `combat.death`.
    #[serde(rename = "combat.death")]
    CombatDeath(CombatDeath),
    /// `medical.incapacitated`.
    #[serde(rename = "medical.incapacitated")]
    MedicalIncapacitated(MedicalSubject),
    /// `medical.revived`.
    #[serde(rename = "medical.revived")]
    MedicalRevived(MedicalSubject),
    /// `vehicle.destroyed`.
    #[serde(rename = "vehicle.destroyed")]
    VehicleDestroyed(VehicleDestroyed),
    /// `vehicle.entered`.
    #[serde(rename = "vehicle.entered")]
    VehicleEntered(VehicleCompartment),
    /// `vehicle.exited`.
    #[serde(rename = "vehicle.exited")]
    VehicleExited(VehicleCompartment),
}

#[derive(Debug, Deserialize)]
struct EventEnvelope {
    event_id: MatchEventId,
    sequence: i64,
    mission_time_ms: i64,
    occurred_at: DateTime<Utc>,
    #[serde(flatten)]
    body: EventBody,
}

/// One validated event, ready to store.
#[derive(Debug, Clone)]
pub struct IncomingEvent {
    /// The runtime's id of the event, unique within the match.
    pub event_id: MatchEventId,
    /// The event's position in the match's event order.
    pub sequence: i64,
    /// The stored kind string.
    pub kind: &'static str,
    /// Mission time of the event, in milliseconds.
    pub mission_time_ms: i64,
    /// When the event happened.
    pub occurred_at: DateTime<Utc>,
    /// The player who acted (the killer, the instigator, the occupant), when there is one.
    pub actor_arma_id: Option<ArmaPlayerId>,
    /// The player acted on (the victim, the subject), when there is one.
    pub subject_arma_id: Option<ArmaPlayerId>,
    /// The submitted payload, stored as `jsonb`.
    pub payload: Value,
    /// Canonical SHA-256 of the payload, which tells a repeated event from a conflicting one.
    pub payload_sha256: String,
}

/// A validated batch of one match's events.
#[derive(Debug, Clone)]
pub struct MatchEventBatch {
    /// The runtime's id of the match the events belong to.
    pub source_match_id: SourceMatchId,
    /// The validated events, in submitted order.
    pub events: Vec<IncomingEvent>,
}

/// The answer to an event batch.
/// @contract match-telemetry.schema.json#/definitions/MatchEventBatchAnswer
#[derive(Debug, Serialize)]
pub struct MatchEventBatchAnswer {
    /// The registered match the batch was stored against.
    pub match_id: MatchId,
    /// Events this batch inserted.
    pub accepted: i64,
    /// Events this batch repeated, already stored.
    pub duplicates: i64,
    /// Events the match holds after this batch.
    pub event_count: i64,
    /// The highest stored `sequence`, `None` while the match holds no event.
    pub last_sequence: Option<i64>,
}

fn event_refusal(
    message: impl Into<String>,
    index: Option<usize>,
    field: Option<&str>,
) -> ApiError {
    invalid_entry(INVALID_EVENT, message, index, field)
}

fn identity(value: &ArmaPlayerId, field: &str, index: usize) -> Result<ArmaPlayerId, ApiError> {
    let trimmed = value.as_str().trim();
    if trimmed.is_empty() || trimmed.len() > 128 || trimmed != value.as_str() {
        return Err(event_refusal(
            format!("{field} must be 1 to 128 bytes without surrounding space"),
            Some(index),
            Some(field),
        ));
    }
    Ok(value.clone())
}

fn optional_identity(
    value: &Option<ArmaPlayerId>,
    field: &str,
    index: usize,
) -> Result<Option<ArmaPlayerId>, ApiError> {
    value
        .as_ref()
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
    fn participants(
        &self,
        index: usize,
    ) -> Result<(Option<ArmaPlayerId>, Option<ArmaPlayerId>), ApiError> {
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
    check_object_keys(value, &EVENT_KEYS, &EVENT_KEYS).map_err(|error| {
        event_refusal(error.to_string(), Some(index), Some(error.offending_key()))
    })?;
    let envelope: EventEnvelope = serde_json::from_value(value.clone()).map_err(|error| {
        let field = field_named_by(&error);
        event_refusal(
            format!("invalid event: {error}"),
            Some(index),
            field.as_deref(),
        )
    })?;
    if !valid_event_id(envelope.event_id.as_str()) {
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
    .map_err(|error| event_refusal(error.to_string(), None, Some(error.offending_key())))?;
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
            source_match_id(raw).map_err(|error| {
                event_refusal(error.to_string(), None, Some(error.offending_key()))
            })
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
