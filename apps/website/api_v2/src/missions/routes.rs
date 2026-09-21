//! The `/api/v1` route table for missions.
//!
//! Paths are written relative to the `/api/v1` nest applied by `core::http_router`. Auth tiers
//! are enforced per-handler by the extractor each takes (`AuthUser`, `MissionMakerUser`,
//! `AdminUser`, `ServiceAuth`), so they travel with the handler rather than with the registration.

use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::routing::{get, post};

use super::handlers;
use crate::core::application_state::AppState;

/// `version_limit` is the body cap applied to the mission-version POST alone, in bytes.
pub fn routes(version_limit: usize) -> Router<AppState> {
    Router::new()
        .route("/registry", get(handlers::registry_items::list_registry))
        .route(
            "/registry/compat",
            get(handlers::registry_compat_graph::list_registry_compat),
        )
        // `/factions` writes are `MissionMakerUser` because a faction is authored content, unlike
        // the infrastructure resources whose writes are `AdminUser`.
        .route(
            "/factions",
            get(handlers::faction_library::list_factions)
                .post(handlers::faction_library::create_faction),
        )
        .route(
            "/factions/{id}",
            get(handlers::faction_library::get_faction)
                .put(handlers::faction_library::update_faction)
                .delete(handlers::faction_library::delete_faction),
        )
        // Mission library + editor.
        .route(
            "/missions",
            get(handlers::mission_library::list_missions)
                .post(handlers::mission_lifecycle::create_mission),
        )
        .route(
            "/missions/{id}",
            get(handlers::mission_library::get_mission)
                .patch(handlers::mission_lifecycle::update_mission)
                .delete(handlers::mission_lifecycle::delete_mission),
        )
        .route(
            "/missions/{id}/submit",
            post(handlers::mission_lifecycle::submit_mission),
        )
        .route(
            "/missions/{id}/versions",
            // The version POST carries the compiled editor payload (hundreds of MB) —
            // override the global 1 MB body cap for this route only.
            post(handlers::mission_versions::create_version)
                .layer(DefaultBodyLimit::max(version_limit)),
        )
        .route(
            "/missions/{id}/versions/{vid}",
            get(handlers::mission_versions::get_version),
        )
        // Re-point current_version_id at a prior mission_versions row (rollback tip).
        .route(
            "/missions/{id}/versions/{vid}/set-current",
            post(handlers::mission_versions::set_current_version),
        )
        .route(
            "/missions/{id}/armory",
            get(handlers::mission_armory::get_armory).put(handlers::mission_armory::set_armory),
        )
        .route(
            "/missions/{id}/bookmark",
            post(handlers::mission_library::bookmark_mission)
                .delete(handlers::mission_library::remove_bookmark),
        )
        .route(
            "/missions/{id}/export",
            get(handlers::mission_export::export_mission),
        )
        .route(
            "/missions/{id}/compiled",
            get(handlers::mission_export::get_compiled_mission),
        )
        // Inject a mission into a live session.
        .route(
            "/missions/{id}/inject",
            post(handlers::game_server_injection::inject_mission),
        )
        // Game-server mission read (service-token). Deliberately NOT the member-tier `/missions`
        // handler: that one is scoped to the CALLING USER (owner/bookmark filters) and a service
        // token has no "me" — see the handler docs.
        .route(
            "/ingest/missions",
            get(handlers::game_server_injection::ingest_list_missions),
        )
        // Corpus-wide default-override instrumentation. Lives under `/admin/*` because it is
        // an aggregate only an admin reads (not per-mission content); the handler is in `missions`
        // because it queries `mission_versions`. Tier via the per-handler `AdminUser` extractor.
        .route(
            "/admin/mission-default-overrides",
            get(handlers::mission_default_overrides::mission_default_overrides),
        )
        // Approvals.
        .route("/approvals", get(handlers::approvals_queue::list_approvals))
        .route(
            "/approvals/{id}/approve",
            post(handlers::approvals_queue::approve_mission),
        )
        .route(
            "/approvals/{id}/reject",
            post(handlers::approvals_queue::reject_mission),
        )
}
