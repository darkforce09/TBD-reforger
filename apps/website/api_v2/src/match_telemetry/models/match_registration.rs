//! The match registration a game runtime sends before any report about the match.
//!
//! **Role:** decodes and validates `POST /api/v1/ingest/matches` bodies.
//! **Position:** read by `handlers::match_registration`; the validated value feeds
//! `services::match_registration`.
//! **Signals & state:** none; pure decoding.
//! **Invariants:** unknown keys (including `server_id`: the credential names the server) are
//! refused; `source_match_id` is 1–128 bytes after trimming; the digest covers the canonical JSON of
//! the whole body as sent.
//! @contract match-telemetry.schema.json#/definitions/MatchRegistration

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use super::telemetry_refusal::{field_named_by, invalid_entry};
use crate::core::error_handling::api_error::ApiError;
use crate::core::wire_format::content_digest::canonical_sha256;

/// Refusal code of a malformed registration.
pub const INVALID_MATCH_REGISTRATION: &str = "INVALID_MATCH_REGISTRATION";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RegistrationWire {
    source_match_id: String,
    runtime_session_id: Uuid,
    started_at: DateTime<Utc>,
    mission_id: Option<Uuid>,
    event_id: Option<Uuid>,
    terrain: Option<String>,
}

/// A validated registration and the digest of the body it came from.
#[derive(Debug, Clone)]
pub struct MatchRegistration {
    pub source_match_id: String,
    pub runtime_session_id: Uuid,
    pub started_at: DateTime<Utc>,
    pub mission_id: Option<Uuid>,
    pub event_id: Option<Uuid>,
    pub terrain: Option<String>,
    pub registration_sha256: String,
}

/// The answer to a registration.
/// @contract match-telemetry.schema.json#/definitions/MatchRegistrationAnswer
#[derive(Debug, Serialize)]
pub struct MatchRegistrationAnswer {
    pub match_id: Uuid,
    pub registered: bool,
}

/// Decode `body` into a [`MatchRegistration`], refusing it whole on the first problem.
pub fn decode_registration(body: &Value) -> Result<MatchRegistration, ApiError> {
    let wire: RegistrationWire = serde_json::from_value(body.clone()).map_err(|error| {
        let field = field_named_by(&error);
        invalid_entry(
            INVALID_MATCH_REGISTRATION,
            format!("invalid match registration: {error}"),
            None,
            field.as_deref(),
        )
    })?;
    let source_match_id = source_match_id(&wire.source_match_id).map_err(|message| {
        invalid_entry(
            INVALID_MATCH_REGISTRATION,
            message,
            None,
            Some("source_match_id"),
        )
    })?;
    let terrain = match wire.terrain.as_deref().map(str::trim) {
        None => None,
        Some("") => {
            return Err(invalid_entry(
                INVALID_MATCH_REGISTRATION,
                "terrain must not be blank",
                None,
                Some("terrain"),
            ));
        }
        Some(terrain) => Some(terrain.to_owned()),
    };
    Ok(MatchRegistration {
        source_match_id,
        runtime_session_id: wire.runtime_session_id,
        started_at: wire.started_at,
        mission_id: wire.mission_id,
        event_id: wire.event_id,
        terrain,
        registration_sha256: canonical_sha256(body),
    })
}

/// The trimmed source match id, 1–128 bytes, or the message naming why not.
pub fn source_match_id(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.len() > 128 {
        return Err("source_match_id must contain 1 to 128 bytes".into());
    }
    Ok(trimmed.to_owned())
}
