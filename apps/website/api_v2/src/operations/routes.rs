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
            get(handlers::member_service_record::get_my_deployments),
        )
        .route(
            "/me/leave-requests",
            get(handlers::leave_requests::list_my_leave)
                .post(handlers::leave_requests::submit_leave),
        )
        // Admin: LOA review console.
        .route(
            "/admin/leave-requests",
            get(handlers::leave_requests::list_all_leave),
        )
        .route(
            "/admin/leave-requests/{id}",
            axum::routing::patch(handlers::leave_requests::review_leave),
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
            get(handlers::orbat_view::get_orbat),
        )
        .route(
            "/event-missions/{emid}/register",
            post(handlers::slot_registration::register_for_event_mission)
                .delete(handlers::slot_registration::withdraw_from_event_mission),
        )
        .route(
            "/event-missions/{emid}/slots/{slotId}/assign",
            axum::routing::put(handlers::slot_assignment::assign_slot)
                .delete(handlers::slot_assignment::clear_slot),
        )
        .route(
            "/event-missions/{emid}/squads/reserve",
            post(handlers::slot_assignment::reserve_squad),
        )
        .route(
            "/event-missions/{emid}/squads/release",
            post(handlers::slot_assignment::release_squad),
        )
        .route("/members", get(handlers::orbat_view::search_members))
        // Game-server roster read (service-token). Deliberately NOT the member-tier
        // `/event-missions/{emid}/orbat` handler: that one is scoped to the CALLING USER (the
        // caller's own registration state) and a service token has no "me" — see the handler docs.
        .route(
            "/ingest/events/{id}/roster",
            get(handlers::roster_ingest::ingest_event_roster),
        )
        // Field tools — mortar ballistics and saved fire missions.
        .route(
            "/fire-missions/solve",
            post(handlers::fire_missions::solve_fire),
        )
        .route("/fire-missions", post(handlers::fire_missions::save_fire))
        .route(
            "/events/{id}/fire-missions",
            get(handlers::fire_missions::list_event_fire_missions),
        )
}
