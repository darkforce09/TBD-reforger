//! The `/api/v1` route table for match telemetry.
//!
//! Paths are written relative to the `/api/v1` nest applied by `core::http_router`. The heartbeat
//! takes a `mod_runtime` machine credential (`MachineCaller`) and match results take
//! `ServiceAuth` (the game-server `X-Service-Token`), each enforced per-handler by the extractor
//! it takes, so the tier travels with the handler rather than with the registration.

use axum::Router;
use axum::routing::post;

use super::handlers;
use crate::core::application_state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/game-runtime/sessions/{sessionId}/heartbeats",
            post(handlers::server_heartbeat::ingest_server_status),
        )
        .route(
            "/ingest/match-results",
            post(handlers::match_results::ingest_match_results),
        )
}
