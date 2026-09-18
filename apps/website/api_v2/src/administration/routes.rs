//! The `/api/v1` route table for administration.
//!
//! Paths are written relative to the `/api/v1` nest applied by `core::http_router`. Every route
//! here is `AdminUser`, enforced per-handler by the extractor each takes, so the tier travels
//! with the handler rather than with the registration. These paths sit under `/admin/*` because
//! they name resources only an admin may READ at all.

use axum::Router;
use axum::routing::{get, post};

use crate::core::application_state::AppState;

use super::handlers;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/admin/audit-logs",
            get(handlers::audit_logs::list_audit_logs),
        )
        .route(
            "/admin/audit-logs/stream",
            get(handlers::audit_logs::stream_audit_logs),
        )
        .route(
            "/admin/audit-logs/export.csv",
            get(handlers::audit_logs::export_audit_logs_csv),
        )
        .route("/admin/users", get(handlers::personnel_roster::list_users))
        .route(
            "/admin/users/{discordId}",
            axum::routing::patch(handlers::role_management::update_user),
        )
        .route(
            "/admin/users/{discordId}/ban",
            post(handlers::disciplinary::ban_user).delete(handlers::disciplinary::unban_user),
        )
        .route(
            "/admin/users/{discordId}/warnings",
            post(handlers::disciplinary::issue_warning),
        )
        .route(
            "/admin/roles/sync",
            post(handlers::role_management::resync_roles),
        )
}
