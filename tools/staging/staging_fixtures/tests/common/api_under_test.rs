//! The API under test: the production services and the route tables the suites call, behind the
//! API binary's middleware chain.
//!
//! **Role:** builds the application state with the services the `api-server` binary runs, and the
//! router that serves the identity and access, operations and missions route tables under
//! `/api/v1`.
//! **Position:** the `staging_fixtures` suites that check what the tool seeded through the API (a
//! seeded member signs in and registers for a slot of a fixture event) call it; the tool crate
//! never depends on the `api_server` application, so the suites compose the API from the api
//! crates the application itself composes it from.
//! **Signals & state:** none of its own; the state owns the pool and the services it builds.
//! **Invariants:** the services are the ones `crates/api/api_server/src/composition.rs` builds
//! (the database session authority over the same configuration, the Discord and webhook clients,
//! the equipment datasets); the route tables nest under `/api/v1` and pass through the layers of
//! `crates/api/api_server/src/router.rs` in the same order: the per-address rate limit, the JSON
//! body limit, cross-origin, panic capture, the request observer, logging and the request id.

use std::path::PathBuf;
use std::sync::Arc;

use api_caller_identity::session_authorization::DatabaseSessionAuthority;
use api_configuration::configuration::Config;
use api_discord::discord_client::DiscordService;
use api_discord::discord_webhook::WebhookService;
use api_equipment_datasets::EquipmentDatasets;
use api_http_layer::middleware;
use api_http_layer::observability::request_observer::observe;
use api_state::AppState;
use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::middleware::{from_fn, from_fn_with_state};
use sqlx::PgPool;
use tower_http::catch_panic::CatchPanicLayer;

/// The application state of an open `pool` and a loaded `cfg`, with the services the
/// `api-server` binary runs.
pub(crate) fn application_state(pool: PgPool, cfg: Config) -> AppState {
    let cfg = Arc::new(cfg);
    let session_authority = Arc::new(DatabaseSessionAuthority {
        pool: pool.clone(),
        config: cfg.clone(),
    });
    let discord = DiscordService::new(
        cfg.discord_client_id.clone(),
        cfg.discord_client_secret.clone(),
        cfg.discord_redirect_url.clone(),
        cfg.discord_guild_id.clone(),
    );
    let webhook = WebhookService::new(cfg.discord_webhook_url.clone());
    let equipment_data = EquipmentDatasets::new(
        &cfg.equipment_data_dir,
        cfg.equipment_export_source_dir.as_ref().map(PathBuf::from),
    );
    AppState::new(
        pool,
        cfg,
        session_authority,
        Arc::new(discord),
        Arc::new(webhook),
        Arc::new(equipment_data),
    )
}

/// The router over `state`: the identity and access, operations and missions route tables under
/// `/api/v1`, behind the API binary's middleware chain.
pub(crate) fn router(state: AppState) -> Router {
    let dev = state.cfg.is_development();
    let version_limit = state.cfg.mission_version_body_limit() as usize;
    let routes = Router::new()
        .merge(api_identity_and_access::routes(dev))
        .merge(api_operations::routes())
        .merge(api_missions::routes(version_limit));
    Router::new()
        .nest("/api/v1", routes)
        .layer(from_fn_with_state(state.clone(), middleware::rate_limit))
        .layer(DefaultBodyLimit::max(middleware::MAX_JSON_BODY))
        .layer(from_fn_with_state(state.clone(), middleware::cors))
        .layer(CatchPanicLayer::new())
        .layer(from_fn_with_state(state.metrics_registry.clone(), observe))
        .layer(from_fn(middleware::logging))
        .layer(from_fn(middleware::request_id))
        .with_state(state)
}
