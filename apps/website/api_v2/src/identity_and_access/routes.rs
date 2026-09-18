//! The `/api/v1` route table for identity and access.
//!
//! Paths are written relative to the `/api/v1` nest applied by `core::http_router`. Auth tiers
//! are enforced per-handler by the extractor each takes (`AuthUser`, `ServiceAuth`), so they
//! travel with the handler rather than with the registration.

use axum::Router;
use axum::routing::{get, post};

use crate::core::application_state::AppState;

use super::handlers;

/// `dev` gates the development-only login shortcut, which is registered only when the
/// configuration reports a development environment.
pub fn routes(dev: bool) -> Router<AppState> {
    let mut r = Router::new()
        .route(
            "/auth/discord/login",
            get(handlers::discord_oauth::discord_login),
        )
        .route(
            "/auth/discord/callback",
            get(handlers::discord_oauth::discord_callback),
        )
        .route("/auth/refresh", post(handlers::session_tokens::refresh))
        .route("/auth/logout", post(handlers::session_tokens::logout))
        .route(
            "/me",
            get(handlers::member_profile::get_me).patch(handlers::member_profile::update_me),
        )
        .route(
            "/me/link",
            post(handlers::arma_link_codes::create_link_code)
                .delete(handlers::arma_link_codes::unlink),
        )
        .route(
            "/me/link/status",
            get(handlers::arma_link_codes::link_status),
        )
        .route(
            "/ingest/link-confirm",
            post(handlers::arma_link_confirmation::ingest_link_confirm),
        );
    if dev {
        // Development-only login shortcut (also re-guards on env in-handler).
        r = r.route("/auth/dev-login", get(handlers::developer_login::dev_login));
    }
    r
}
