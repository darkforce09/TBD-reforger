//! The `/api/v1` route table for server infrastructure.
//!
//! Paths are written relative to the `/api/v1` nest applied by `core::http_router`. Auth tiers
//! are enforced per-handler by the extractor each takes (`AuthUser`, `AdminUser`), so they travel
//! with the handler rather than with the registration.
//!
//! The write tier here is `AdminUser` and NOT `MissionMakerUser`: a `servers` row is
//! infrastructure, not mission content. The row carries the `inet` + port that RCON dials and
//! that the game-server ingest path keys off.
//!
//! The writes live at `/servers`, not `/admin/servers`: that is what the `@route` tags claim, and
//! it is what the crate does for every other admin write on a resource the whole authenticated
//! site can read. `/admin/*` is reserved for resources only an admin may READ at all.

use axum::Router;
use axum::routing::{get, post};

use crate::core::application_state::AppState;

use super::handlers;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/servers",
            get(handlers::server_intel::list_servers)
                .post(handlers::server_registry::create_server),
        )
        .route(
            "/servers/{id}",
            axum::routing::patch(handlers::server_registry::update_server)
                .delete(handlers::server_registry::deactivate_server),
        )
        .route(
            "/servers/{id}/status",
            get(handlers::server_intel::get_server_status),
        )
        .route(
            "/servers/{id}/status/stream",
            get(handlers::server_status_stream::stream_server_status),
        )
        .route(
            "/admin/servers/{id}/rcon",
            post(handlers::rcon_console::send_rcon),
        )
}
