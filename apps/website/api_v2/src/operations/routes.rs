//! The `/api/v1` route table for operations.
//!
//! Paths are written relative to the `/api/v1` nest applied by `core::http_router`. Auth tiers
//! are enforced per-handler by the extractor each takes (`AuthUser`, `LeaderUser`, `AdminUser`,
//! `ServiceAuth`), so they travel with the handler rather than with the registration.

use axum::Router;
use axum::routing::{get, post};

use crate::core::application_state::AppState;
use crate::handlers;

pub fn routes() -> Router<AppState> {
    Router::new()
        // The caller's own deployment history and leave requests.
        .route(
            "/me/deployments",
            get(handlers::deployments::get_my_deployments),
        )
        .route(
            "/me/leave-requests",
            get(handlers::deployments::list_my_leave).post(handlers::deployments::submit_leave),
        )
        // Admin: LOA review console.
        .route(
            "/admin/leave-requests",
            get(handlers::deployments::list_all_leave),
        )
        .route(
            "/admin/leave-requests/{id}",
            axum::routing::patch(handlers::deployments::review_leave),
        )
        // Events (campaign) + ORBAT + registration.
        .route(
            "/events",
            get(handlers::events::list_events).post(handlers::events::create_event),
        )
        .route(
            "/events/{id}",
            get(handlers::events::get_event)
                .patch(handlers::events::update_event)
                .delete(handlers::events::delete_event),
        )
        .route(
            "/events/{id}/missions",
            post(handlers::events::add_event_mission),
        )
        .route(
            "/events/{id}/missions/{emid}",
            axum::routing::delete(handlers::events::remove_event_mission),
        )
        .route(
            "/event-missions/{emid}/orbat",
            get(handlers::events::get_orbat),
        )
        .route(
            "/event-missions/{emid}/register",
            post(handlers::events::register_for_event_mission)
                .delete(handlers::events::withdraw_from_event_mission),
        )
        .route(
            "/event-missions/{emid}/slots/{slotId}/assign",
            axum::routing::put(handlers::events::assign_slot).delete(handlers::events::clear_slot),
        )
        .route(
            "/event-missions/{emid}/squads/reserve",
            post(handlers::events::reserve_squad),
        )
        .route(
            "/event-missions/{emid}/squads/release",
            post(handlers::events::release_squad),
        )
        .route("/members", get(handlers::events::search_members))
        // Game-server roster read (service-token). Deliberately NOT the member-tier
        // `/event-missions/{emid}/orbat` handler: that one is scoped to the CALLING USER (the
        // caller's own registration state) and a service token has no "me" — see the handler docs.
        .route(
            "/ingest/events/{id}/roster",
            get(handlers::events::ingest_event_roster),
        )
        // Field tools — mortar ballistics and saved fire missions.
        .route(
            "/fire-missions/solve",
            post(handlers::field_tools::solve_fire),
        )
        .route("/fire-missions", post(handlers::field_tools::save_fire))
        .route(
            "/events/{id}/fire-missions",
            get(handlers::field_tools::list_event_fire_missions),
        )
}
