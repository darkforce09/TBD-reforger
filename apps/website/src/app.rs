//! HTTP application assembly — the router + global middleware chain. Shared by the
//! `api` binary and the test/differential harnesses so they exercise one router.

use axum::Router;
use axum::extract::{DefaultBodyLimit, State};
use axum::http::StatusCode;
use axum::middleware::{from_fn, from_fn_with_state};
use axum::response::Json;
use axum::routing::{get, post};
use serde_json::json;
use sqlx::PgPool;
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::services::ServeDir;

use crate::state::AppState;
use crate::{handlers, middleware};

/// The `/api/v1` route tree. Auth tiers are enforced per-handler by the extractor
/// each takes (`AuthUser`, the role-gated newtypes, `ServiceAuth`). Grows per phase.
fn api_routes(dev: bool, version_limit: usize) -> Router<AppState> {
    let mut r = Router::new()
        .route("/auth/discord/login", get(handlers::oauth::discord_login))
        .route(
            "/auth/discord/callback",
            get(handlers::oauth::discord_callback),
        )
        .route("/auth/refresh", post(handlers::auth::refresh))
        .route("/auth/logout", post(handlers::auth::logout))
        .route(
            "/me",
            get(handlers::me::get_me).patch(handlers::me::update_me),
        )
        .route(
            "/me/link",
            post(handlers::me::create_link_code).delete(handlers::me::unlink),
        )
        .route("/me/link/status", get(handlers::me::link_status))
        .route(
            "/ingest/link-confirm",
            post(handlers::me::ingest_link_confirm),
        )
        // Content reads (member tier via each handler's AuthUser extractor).
        .route(
            "/announcements",
            get(handlers::announcements::list_announcements),
        )
        .route(
            "/announcements/{id}",
            get(handlers::announcements::get_announcement),
        )
        .route("/wiki", get(handlers::wiki::list_wiki))
        .route(
            "/wiki/{slug}",
            get(handlers::wiki::get_wiki_page).put(handlers::wiki::upsert_wiki_page),
        )
        .route("/vehicle-database", get(handlers::wiki::list_vehicles))
        .route("/modpacks", get(handlers::modpacks::list_modpacks))
        .route(
            "/modpacks/current",
            get(handlers::modpacks::get_current_modpack),
        )
        .route("/servers", get(handlers::servers::list_servers))
        .route(
            "/servers/{id}/status",
            get(handlers::servers::get_server_status),
        )
        .route(
            "/servers/{id}/status/stream",
            get(handlers::leaderboards::stream_server_status),
        )
        .route("/registry", get(handlers::registry::list_registry))
        .route("/dashboard", get(handlers::dashboard::get_dashboard))
        .route(
            "/leaderboards",
            get(handlers::leaderboards::get_leaderboards),
        )
        .route(
            "/users/{discordId}/stats",
            get(handlers::leaderboards::get_user_stats),
        )
        .route(
            "/me/deployments",
            get(handlers::deployments::get_my_deployments),
        )
        .route(
            "/me/leave-requests",
            get(handlers::deployments::list_my_leave).post(handlers::deployments::submit_leave),
        )
        // Admin: LOA review + audit console.
        .route(
            "/admin/leave-requests",
            get(handlers::deployments::list_all_leave),
        )
        .route(
            "/admin/leave-requests/{id}",
            axum::routing::patch(handlers::deployments::review_leave),
        )
        .route("/admin/audit-logs", get(handlers::audit::list_audit_logs))
        .route(
            "/admin/audit-logs/stream",
            get(handlers::audit::stream_audit_logs),
        )
        .route(
            "/admin/audit-logs/export.csv",
            get(handlers::audit::export_audit_logs_csv),
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
        // Game-server telemetry ingest (service-token).
        .route(
            "/ingest/server-status",
            post(handlers::telemetry::ingest_server_status),
        )
        .route(
            "/ingest/match-results",
            post(handlers::telemetry::ingest_match_results),
        )
        // Admin — personnel + server control.
        .route("/admin/users", get(handlers::admin::list_users))
        .route(
            "/admin/users/{discordId}",
            axum::routing::patch(handlers::admin::update_user),
        )
        .route(
            "/admin/users/{discordId}/ban",
            post(handlers::admin::ban_user).delete(handlers::admin::unban_user),
        )
        .route(
            "/admin/users/{discordId}/warnings",
            post(handlers::admin::issue_warning),
        )
        .route("/admin/roles/sync", post(handlers::admin::resync_roles))
        .route("/admin/servers/{id}/rcon", post(handlers::admin::send_rcon))
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
        // Field tools — mortar + inject.
        .route(
            "/fire-missions/solve",
            post(handlers::field_tools::solve_fire),
        )
        .route("/fire-missions", post(handlers::field_tools::save_fire))
        .route(
            "/events/{id}/fire-missions",
            get(handlers::field_tools::list_event_fire_missions),
        )
        .route(
            "/missions/{id}/inject",
            post(handlers::field_tools::inject_mission),
        )
        // CMS — announcements + uploads.
        .route(
            "/cms/announcements",
            post(handlers::cms::create_announcement),
        )
        .route(
            "/cms/announcements/{id}",
            axum::routing::patch(handlers::cms::update_announcement)
                .delete(handlers::cms::delete_announcement),
        )
        .route(
            "/cms/announcements/{id}/push-discord",
            post(handlers::cms::push_announcement_discord),
        )
        .route(
            "/cms/uploads",
            post(handlers::cms::upload_image)
                .layer(DefaultBodyLimit::max(middleware::MAX_MULTIPART_BODY)),
        );
    if dev {
        // Development-only login shortcut (also re-guards on env in-handler).
        r = r.route("/auth/dev-login", get(handlers::dev::dev_login));
    }
    r
}

/// Build the application: `/healthz`, `/api/v1/*`, static `/uploads`, and the global
/// middleware chain (outermost first: request-id → logging → recovery → CORS →
/// body-limit → rate-limit).
pub fn router(state: AppState) -> Router {
    let dev = state.cfg.is_development();
    let version_limit = state.cfg.mission_version_body_limit() as usize;
    Router::new()
        .route("/healthz", get(healthz))
        .nest("/api/v1", api_routes(dev, version_limit))
        .nest_service("/uploads", ServeDir::new("uploads"))
        .layer(from_fn_with_state(state.clone(), middleware::rate_limit))
        .layer(DefaultBodyLimit::max(middleware::MAX_JSON_BODY))
        .layer(from_fn_with_state(state.clone(), middleware::cors))
        .layer(CatchPanicLayer::new())
        .layer(from_fn(middleware::logging))
        .layer(from_fn(middleware::request_id))
        .with_state(state)
}

/// Liveness probe — pings the DB (mirrors the Go `/healthz`).
async fn healthz(State(pool): State<PgPool>) -> (StatusCode, Json<serde_json::Value>) {
    match sqlx::query("SELECT 1").execute(&pool).await {
        Ok(_) => (StatusCode::OK, Json(json!({ "status": "ok" }))),
        Err(_) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "status": "unavailable" })),
        ),
    }
}
