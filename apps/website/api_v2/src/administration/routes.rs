//! The `/api/v1` route table for administration.
//!
//! Paths are written relative to the `/api/v1` nest applied by `core::http_router`. Every route
//! here is `AdminUser`, enforced per-handler by the extractor each takes, so the tier travels
//! with the handler rather than with the registration. These paths sit under `/admin/*` because
//! they name resources only an admin may READ at all.

use axum::Router;
use axum::routing::{get, post};

use crate::core::application_state::AppState;
use crate::handlers;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/admin/audit-logs", get(handlers::audit::list_audit_logs))
        .route(
            "/admin/audit-logs/stream",
            get(handlers::audit::stream_audit_logs),
        )
        .route(
            "/admin/audit-logs/export.csv",
            get(handlers::audit::export_audit_logs_csv),
        )
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
}
