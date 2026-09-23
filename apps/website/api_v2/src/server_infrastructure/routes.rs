//! The `/api/v1` route table for server infrastructure.
//!
//! Paths are written relative to the `/api/v1` nest applied by `core::http_router`. Auth tiers
//! are enforced per-handler by the extractor each takes (`AuthUser`, `AdminUser`, and the
//! machine-credential `MachineCaller` on `/game-runtime/sessions` and `/fleet-executor/commands`),
//! so they travel with the handler rather than with the registration.
//!
//! The write tier here is `AdminUser` and NOT `MissionMakerUser`: a `servers` row is
//! infrastructure, not mission content, and its commands control the server process.
//!
//! The writes live at `/servers`, not `/admin/servers`: that is what the `@route` tags claim, and
//! it is what the crate does for every other admin write on a resource the whole authenticated
//! site can read. `/admin/*` is reserved for resources only an admin may READ at all.

use axum::Router;
use axum::routing::{delete, get, post, put};

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
            "/servers/{id}/credentials",
            get(handlers::machine_credentials::list_server_credentials)
                .post(handlers::machine_credentials::issue_server_credential),
        )
        .route(
            "/servers/{id}/credentials/{credentialId}",
            delete(handlers::machine_credentials::revoke_server_credential),
        )
        .route(
            "/servers/{id}/commands",
            get(handlers::fleet_commands::list_server_commands)
                .post(handlers::fleet_commands::request_server_command),
        )
        .route(
            "/servers/{id}/commands/{commandId}",
            get(handlers::fleet_commands::get_server_command),
        )
        .route(
            "/servers/{id}/commands/{commandId}/cancel",
            post(handlers::fleet_commands::cancel_server_command),
        )
        .route(
            "/fleet-executor/commands/claim",
            post(handlers::fleet_executor::claim_fleet_command),
        )
        .route(
            "/fleet-executor/commands/{commandId}/executing",
            post(handlers::fleet_executor::start_fleet_command),
        )
        .route(
            "/fleet-executor/commands/{commandId}/result",
            post(handlers::fleet_executor::finish_fleet_command),
        )
        .route(
            "/game-runtime/sessions",
            post(handlers::game_runtime_sessions::start_server_runtime_session),
        )
        .route(
            "/game-runtime/sessions/{sessionId}/end",
            post(handlers::game_runtime_sessions::end_server_runtime_session),
        )
        .route(
            "/fleet/scenarios",
            get(handlers::fleet_scenarios::list_fleet_scenarios),
        )
        .route(
            "/fleet/scenarios/{terrainKey}",
            put(handlers::fleet_scenarios::put_fleet_scenario)
                .delete(handlers::fleet_scenarios::delete_fleet_scenario),
        )
}
