//! The `/api/v1` route table for the command center.
//!
//! Paths are written relative to the `/api/v1` nest applied by `core::http_router`. Every route
//! here is `AuthUser`, enforced per-handler by the extractor each takes, so the tier travels with
//! the handler rather than with the registration.

use axum::Router;
use axum::routing::get;

use crate::core::application_state::AppState;
use crate::handlers;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/dashboard", get(handlers::dashboard::get_dashboard))
        .route(
            "/leaderboards",
            get(handlers::leaderboards::get_leaderboards),
        )
        .route(
            "/users/{discordId}/stats",
            get(handlers::leaderboards::get_user_stats),
        )
}
