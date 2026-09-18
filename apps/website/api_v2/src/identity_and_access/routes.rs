//! The `/api/v1` route table for identity and access.
//!
//! Paths are written relative to the `/api/v1` nest applied by `core::http_router`. Auth tiers
//! are enforced per-handler by the extractor each takes (`AuthUser`, `ServiceAuth`), so they
//! travel with the handler rather than with the registration.

use axum::Router;
use axum::routing::{get, post};

use crate::core::application_state::AppState;
use crate::handlers;

/// `dev` gates the development-only login shortcut, which is registered only when the
/// configuration reports a development environment.
pub fn routes(dev: bool) -> Router<AppState> {
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
        );
    if dev {
        // Development-only login shortcut (also re-guards on env in-handler).
        r = r.route("/auth/dev-login", get(handlers::dev::dev_login));
    }
    r
}
