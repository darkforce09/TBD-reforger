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
        .route("/registry", get(crate::handlers::registry::list_registry))
        .route(
            "/registry/compat",
            get(crate::handlers::registry::list_registry_compat),
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
            get(crate::handlers::missions::list_missions)
                .post(crate::handlers::missions::create_mission),
        )
        .route(
            "/missions/{id}",
            get(crate::handlers::missions::get_mission)
                .patch(crate::handlers::missions::update_mission)
                .delete(crate::handlers::missions::delete_mission),
        )
        .route(
            "/missions/{id}/submit",
            post(crate::handlers::missions::submit_mission),
        )
        .route(
            "/missions/{id}/versions",
            // The version POST carries the compiled editor payload (hundreds of MB) —
            // override the global 1 MB body cap for this route only (Go: per-route BodyLimit).
            post(crate::handlers::missions::create_version)
                .layer(DefaultBodyLimit::max(version_limit)),
        )
        .route(
            "/missions/{id}/versions/{vid}",
            get(crate::handlers::missions::get_version),
        )
        // Re-point current_version_id at a prior mission_versions row (rollback tip).
        .route(
            "/missions/{id}/versions/{vid}/set-current",
            post(crate::handlers::missions::set_current_version),
        )
        .route(
            "/missions/{id}/armory",
            get(crate::handlers::missions::get_armory).put(crate::handlers::missions::set_armory),
        )
        .route(
            "/missions/{id}/bookmark",
            post(crate::handlers::missions::bookmark_mission)
                .delete(crate::handlers::missions::remove_bookmark),
        )
        .route(
            "/missions/{id}/export",
            get(crate::handlers::missions::export_mission),
        )
        .route(
            "/missions/{id}/compiled",
            get(crate::handlers::missions::get_compiled_mission),
        )
        // Inject a mission into a live session.
        .route(
            "/missions/{id}/inject",
            post(crate::handlers::field_tools::inject_mission),
        )
        // Game-server mission read (service-token). Deliberately NOT the member-tier `/missions`
        // handler: that one is scoped to the CALLING USER (owner/bookmark filters) and a service
        // token has no "me" — see the handler docs.
        .route(
            "/ingest/missions",
            get(crate::handlers::missions::ingest_list_missions),
        )
        // Corpus-wide default-override instrumentation. Lives under `/admin/*` because it is
        // an aggregate only an admin reads (not per-mission content); the handler is in `missions`
        // because it queries `mission_versions`. Tier via the per-handler `AdminUser` extractor.
        .route(
            "/admin/mission-default-overrides",
            get(crate::handlers::missions::mission_default_overrides),
        )
        // Approvals.
        .route(
            "/approvals",
            get(crate::handlers::approvals::list_approvals),
        )
        .route(
            "/approvals/{id}/approve",
            post(crate::handlers::approvals::approve_mission),
        )
        .route(
            "/approvals/{id}/reject",
            post(crate::handlers::approvals::reject_mission),
        )
}
