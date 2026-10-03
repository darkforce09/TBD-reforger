//! Registration of a server-scoped source match before any report about it.
//!
//! **Role:** creates the `pending` match a game runtime names by its source match id, or answers
//! the existing one for a repeat of the same registration.
//! **Position:** called by `handlers::match_registration` with a validated
//! [`MatchRegistration`] and the machine caller.
//! **Signals & state:** one transaction per call.
//! **Invariants:** the runtime session must belong to the caller's server (it may have ended:
//! queued registrations arrive after restarts); `(server_id, source_match_id)` is unique, so
//! concurrent registrations create one match; the same digest again is inert, another digest is
//! the 409 `REGISTRATION_CONFLICT`; a new match starts `pending` at revision 0.

use api_identifiers::{MatchId, ServerId};
use serde_json::json;

use super::ingest_parsing::{foreign_key_error, parse_terrain_opt};
use crate::models::match_registration::{MatchRegistration, MatchRegistrationAnswer};
use crate::models::telemetry_refusal::telemetry_conflict;
use api_caller_identity::machine_caller::MachineCaller;
use api_foundation::error_handling::api_error::ApiError;
use api_state::AppState;

/// Refusal code of a different registration for a registered source.
pub const REGISTRATION_CONFLICT: &str = "REGISTRATION_CONFLICT";

/// Register `registration` for the caller's server.
pub async fn register_match(
    state: &AppState,
    caller: &MachineCaller,
    registration: &MatchRegistration,
) -> Result<MatchRegistrationAnswer, ApiError> {
    let mut tx = state.pool.begin().await?;
    let session_server: Option<ServerId> =
        sqlx::query_scalar("SELECT server_id FROM server_runtime_sessions WHERE id = $1")
            .bind(registration.runtime_session_id)
            .fetch_optional(&mut *tx)
            .await?;
    let Some(session_server) = session_server else {
        return Err(ApiError::bad_request(
            "runtime_session_id names no runtime session",
        ));
    };
    caller.require_server(session_server)?;

    let inserted: Option<MatchId> = sqlx::query_scalar(
        "INSERT INTO matches (source_match_id, server_id, registered_runtime_session_id,
                              registration_sha256, event_id, mission_id, terrain, started_at,
                              outcome, winning_faction, aar_replay_url, created_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'pending', '', '', clock_timestamp())
         ON CONFLICT (server_id, source_match_id) WHERE server_id IS NOT NULL DO NOTHING
         RETURNING id",
    )
    .bind(&registration.source_match_id)
    .bind(caller.server_id)
    .bind(registration.runtime_session_id)
    .bind(&registration.registration_sha256)
    .bind(registration.event_id)
    .bind(registration.mission_id)
    .bind(parse_terrain_opt(&registration.terrain))
    .bind(registration.started_at)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|error| foreign_key_error(&error).unwrap_or_else(|| error.into()))?;
    if let Some(match_id) = inserted {
        tx.commit().await?;
        return Ok(MatchRegistrationAnswer {
            match_id,
            registered: true,
        });
    }

    let (match_id, stored_sha256): (MatchId, String) = sqlx::query_as(
        "SELECT id, COALESCE(registration_sha256, '') FROM matches
         WHERE server_id = $1 AND source_match_id = $2",
    )
    .bind(caller.server_id)
    .bind(&registration.source_match_id)
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    if stored_sha256 != registration.registration_sha256 {
        return Err(telemetry_conflict(
            REGISTRATION_CONFLICT,
            "this source match is registered with another registration",
            json!({ "match_id": match_id }),
        ));
    }
    Ok(MatchRegistrationAnswer {
        match_id,
        registered: false,
    })
}
