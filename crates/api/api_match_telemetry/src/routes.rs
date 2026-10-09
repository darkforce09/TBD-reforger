//! The `/api/v1` route table for match telemetry.
//!
//! Paths are written relative to the `/api/v1` nest the API's router
//! (`crates/api/api_server/src/router.rs`) applies. The heartbeat and the three ingest routes take a
//! `mod_runtime` machine credential (`MachineCaller`); the event read takes a signed-in user
//! (`AuthUser`). Each is enforced per-handler by the extractor it takes, so the tier travels with
//! the handler rather than with the registration.

use axum::Router;
use axum::routing::{get, post};

use super::handlers;
use api_state::AppState;

/// The match telemetry route table the API's router merges under `/api/v1`: the session-fenced
/// heartbeat, the three ingests and the event read.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/game-runtime/sessions/{sessionId}/heartbeats",
            post(handlers::server_heartbeat::ingest_server_status),
        )
        .route(
            "/ingest/matches",
            post(handlers::match_registration::ingest_match_registration),
        )
        .route(
            "/ingest/match-results",
            post(handlers::match_results::ingest_match_results),
        )
        .route(
            "/ingest/match-events",
            post(handlers::match_event_batches::ingest_match_events),
        )
        .route(
            "/matches/{matchId}/events",
            get(handlers::match_event_reads::list_match_events),
        )
}
