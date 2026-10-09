//! The `/api/v1` route table for the command center.
//!
//! **Role:** registers the dashboard, leaderboard and statistics card routes.
//! **Position:** the API's router (`crates/api/api_server/src/router.rs`) merges [`routes()`] under
//! its `/api/v1` nest, so the paths here are relative to that nest.
//! **Signals & state:** none; the table is built once at boot.
//! **Invariants:** every route here is `AuthUser`, enforced per handler by the extractor each
//! takes, so the tier travels with the handler rather than with the registration.

use axum::Router;
use axum::routing::get;

use super::handlers;
use api_state::AppState;

/// The command center's three `AuthUser` routes, relative to `/api/v1`.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/dashboard", get(handlers::live_dashboard::get_dashboard))
        .route(
            "/leaderboards",
            get(handlers::leaderboards::get_leaderboards),
        )
        .route(
            "/users/{discordId}/stats",
            get(handlers::user_stats_card::get_user_stats),
        )
}
