//! The `/api/v1` route table for operations.
//!
//! Paths are written relative to the `/api/v1` nest applied by `core::http_router`. Auth tiers
//! are enforced per-handler by the extractor each takes (`AuthUser`, `LeaderUser`, `AdminUser`,
//! `ServiceAuth`), so they travel with the handler rather than with the registration.

use axum::Router;
use axum::routing::{get, post};

use super::handlers;
use crate::core::application_state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        // The caller's own deployment history and leave requests.
        .route(
            "/me/deployments",
            get(crate::handlers::deployments::get_my_deployments),
        )
        .route(
            "/me/leave-requests",
            get(crate::handlers::deployments::list_my_leave)
                .post(crate::handlers::deployments::submit_leave),
        )
        // Admin: LOA review console.
        .route(
            "/admin/leave-requests",
            get(crate::handlers::deployments::list_all_leave),
        )
        .route(
            "/admin/leave-requests/{id}",
            axum::routing::patch(crate::handlers::deployments::review_leave),
        )
        // Events (campaign) + ORBAT + registration.
        .route(
            "/events",
            get(handlers::event_listing::list_events)
                .post(handlers::event_create_update::create_event),
        )
        .route(
            "/events/{id}",
            get(handlers::event_listing::get_event)
                .patch(handlers::event_create_update::update_event)
                .delete(handlers::event_create_update::delete_event),
        )
        .route(
            "/events/{id}/missions",
            post(handlers::event_mission_attachment::add_event_mission),
        )
        .route(
            "/events/{id}/missions/{emid}",
            axum::routing::delete(handlers::event_mission_attachment::remove_event_mission),
        )
        .route(
            "/event-missions/{emid}/orbat",
            get(crate::handlers::events::get_orbat),
        )
        .route(
            "/event-missions/{emid}/register",
            post(crate::handlers::events::register_for_event_mission)
                .delete(crate::handlers::events::withdraw_from_event_mission),
        )
        .route(
            "/event-missions/{emid}/slots/{slotId}/assign",
            axum::routing::put(crate::handlers::events::assign_slot)
                .delete(crate::handlers::events::clear_slot),
        )
        .route(
            "/event-missions/{emid}/squads/reserve",
            post(crate::handlers::events::reserve_squad),
        )
        .route(
            "/event-missions/{emid}/squads/release",
            post(crate::handlers::events::release_squad),
        )
        .route("/members", get(crate::handlers::events::search_members))
        // Game-server roster read (service-token). Deliberately NOT the member-tier
        // `/event-missions/{emid}/orbat` handler: that one is scoped to the CALLING USER (the
        // caller's own registration state) and a service token has no "me" — see the handler docs.
        .route(
            "/ingest/events/{id}/roster",
            get(crate::handlers::events::ingest_event_roster),
        )
        // Field tools — mortar ballistics and saved fire missions.
        .route(
            "/fire-missions/solve",
            post(crate::handlers::field_tools::solve_fire),
        )
        .route(
            "/fire-missions",
            post(crate::handlers::field_tools::save_fire),
        )
        .route(
            "/events/{id}/fire-missions",
            get(crate::handlers::field_tools::list_event_fire_missions),
        )
}
