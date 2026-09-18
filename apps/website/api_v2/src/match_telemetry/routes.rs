//! The `/api/v1` route table for match telemetry.
//!
//! Paths are written relative to the `/api/v1` nest applied by `core::http_router`. Both routes
//! are `ServiceAuth` (the game-server `X-Service-Token`), enforced per-handler by the extractor
//! each takes, so the tier travels with the handler rather than with the registration.

use axum::Router;
use axum::routing::post;

use super::handlers;
use crate::core::application_state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/ingest/server-status",
            post(handlers::server_heartbeat::ingest_server_status),
        )
        .route(
            "/ingest/match-results",
            post(handlers::match_results::ingest_match_results),
        )
}
