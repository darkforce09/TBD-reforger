//! The `/api/v1` route table for missions.
//!
//! Paths are written relative to the `/api/v1` nest applied by `core::http_router`. Auth tiers
//! are enforced per-handler by the extractor each takes (`AuthUser`, `MissionMakerUser`,
//! `AdminUser`, `ServiceAuth`), so they travel with the handler rather than with the registration.

use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::routing::{get, post};

use crate::core::application_state::AppState;
use crate::handlers;

/// `version_limit` is the body cap applied to the mission-version POST alone, in bytes.
pub fn routes(version_limit: usize) -> Router<AppState> {
    Router::new()
        .route("/registry", get(handlers::registry::list_registry))
        .route(
            "/registry/compat",
            get(handlers::registry::list_registry_compat),
        )
        // `/factions` writes are `MissionMakerUser` because a faction is authored content, unlike
        // the infrastructure resources whose writes are `AdminUser`.
        .route(
            "/factions",
            get(handlers::factions::list_factions).post(handlers::factions::create_faction),
        )
        .route(
            "/factions/{id}",
            get(handlers::factions::get_faction)
                .put(handlers::factions::update_faction)
                .delete(handlers::factions::delete_faction),
        )
        // Mission library + editor.
        .route(
            "/missions",
            get(handlers::missions::list_missions).post(handlers::missions::create_mission),
        )
        .route(
            "/missions/{id}",
            get(handlers::missions::get_mission)
                .patch(handlers::missions::update_mission)
                .delete(handlers::missions::delete_mission),
        )
        .route(
            "/missions/{id}/submit",
            post(handlers::missions::submit_mission),
        )
        .route(
            "/missions/{id}/versions",
            // The version POST carries the compiled editor payload (hundreds of MB) —
            // override the global 1 MB body cap for this route only (Go: per-route BodyLimit).
            post(handlers::missions::create_version).layer(DefaultBodyLimit::max(version_limit)),
        )
        .route(
            "/missions/{id}/versions/{vid}",
            get(handlers::missions::get_version),
        )
        // Re-point current_version_id at a prior mission_versions row (rollback tip).
        .route(
            "/missions/{id}/versions/{vid}/set-current",
            post(handlers::missions::set_current_version),
        )
        .route(
            "/missions/{id}/armory",
            get(handlers::missions::get_armory).put(handlers::missions::set_armory),
        )
        .route(
            "/missions/{id}/bookmark",
            post(handlers::missions::bookmark_mission).delete(handlers::missions::remove_bookmark),
        )
        .route(
            "/missions/{id}/export",
            get(handlers::missions::export_mission),
        )
        .route(
            "/missions/{id}/compiled",
            get(handlers::missions::get_compiled_mission),
        )
        // Inject a mission into a live session.
        .route(
            "/missions/{id}/inject",
            post(handlers::field_tools::inject_mission),
        )
        // Game-server mission read (service-token). Deliberately NOT the member-tier `/missions`
        // handler: that one is scoped to the CALLING USER (owner/bookmark filters) and a service
        // token has no "me" — see the handler docs.
        .route(
            "/ingest/missions",
            get(handlers::missions::ingest_list_missions),
        )
        // Corpus-wide default-override instrumentation. Lives under `/admin/*` because it is
        // an aggregate only an admin reads (not per-mission content); the handler is in `missions`
        // because it queries `mission_versions`. Tier via the per-handler `AdminUser` extractor.
        .route(
            "/admin/mission-default-overrides",
            get(handlers::missions::mission_default_overrides),
        )
        // Approvals.
        .route("/approvals", get(handlers::approvals::list_approvals))
        .route(
            "/approvals/{id}/approve",
            post(handlers::approvals::approve_mission),
        )
        .route(
            "/approvals/{id}/reject",
            post(handlers::approvals::reject_mission),
        )
}
