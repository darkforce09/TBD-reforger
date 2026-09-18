//! API server entrypoint.
//!
//! Boot order: load config → open pool (with backoff) → run migrations → spawn the background
//! workers → build the router + middleware ([`website_api::core::http_router::router`]) → serve
//! on `:PORT` with graceful shutdown (SIGINT/SIGTERM).

use std::net::SocketAddr;

use tracing_subscriber::EnvFilter;
use website_api::background_workers::{leaderboard_refresher, server_status_publisher};
use website_api::core::application_state::AppState;
use website_api::core::configuration::Config;
use website_api::core::{database, http_router};
use website_api::{handlers, services};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let cfg = Config::load()?;
    let pool = database::connect(&cfg.database_url).await?;
    // `SKIP_MIGRATE` lets a harness that owns migration of a shared database keep this
    // process from racing it.
    if std::env::var("SKIP_MIGRATE").is_err() {
        database::migrate(&pool).await?;
        tracing::info!(env = %cfg.env, "migrations applied");
    }

    let port = cfg.port.clone();
    let state = AppState::new(pool, cfg);
    // Background refresh-token purge (immediate sweep, then every 6h).
    let _purge = services::start_refresh_token_purge(state.pool.clone());
    // Event lifecycle convergence: starts operations at their start time and
    // completes them past their end horizon, so the stored `events.status` agrees with the
    // status the handlers derive. Safe to be late or absent — the registration window and
    // every read derive from `now()` at request time, never from this task's output.
    let _lifecycle = handlers::events::start_event_lifecycle(state.pool.clone());
    // Scheduled `leaderboard_totals` MV refresh — immediate + interval (env
    // `LEADERBOARD_REFRESH_INTERVAL_SECS`, default 15m). Safety net when telemetry/me
    // ingest is quiet; those callers still refresh in-request.
    let lb_interval = leaderboard_refresher::leaderboard_refresh_interval();
    tracing::info!(
        secs = lb_interval.as_secs(),
        "leaderboard MV scheduled refresh armed"
    );
    let _leaderboard =
        leaderboard_refresher::start_leaderboard_refresh(state.pool.clone(), lb_interval);
    // Scheduled `server_statuses` → SSE republish — immediate + interval (env
    // `SERVER_STATUS_PUBLISH_INTERVAL_SECS`, default 10s). Closes the SSE loop when no
    // game-server bridge calls ingest; ingest still publishes in-request.
    let ss_interval = server_status_publisher::server_status_publish_interval();
    tracing::info!(
        secs = ss_interval.as_secs(),
        "server-status SSE republish armed"
    );
    let _server_status = server_status_publisher::start_server_status_publisher(
        state.pool.clone(),
        state.hub.clone(),
        ss_interval,
    );
    // Scheduled Discord → web role resync — immediate + interval (env
    // `ROLE_RESYNC_INTERVAL_SECS`, default 24h / nightly). Safety net when admins
    // remap `discord_roles` without curling POST /admin/roles/sync; OAuth login
    // still syncs in-request.
    let role_interval = services::role_sync::role_resync_interval();
    tracing::info!(secs = role_interval.as_secs(), "discord role resync armed");
    let _role_resync = services::role_sync::start_role_resync(state.pool.clone(), role_interval);
    // Garbage-collect the durable rate limiter's bucket table — one row per client IP seen on
    // the auth/ingest surface, and nothing else removes them. Deleting an idle bucket can never
    // grant quota (it refills to full in ten seconds; the TTL is an hour), so this is
    // reclamation only — see `services::ratelimit_gc`.
    let _rl_prune = services::start_rate_limit_prune(
        state.pool.clone(),
        services::RATE_LIMIT_BUCKET_TTL,
        services::RATE_LIMIT_PRUNE_INTERVAL,
    );
    let app = http_router::router(state);

    let addr = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("listening on {addr}");
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;
    Ok(())
}

/// Resolve on SIGINT or SIGTERM so `axum::serve` drains in-flight requests.
async fn shutdown_signal() {
    use tokio::signal;

    let ctrl_c = async {
        signal::ctrl_c().await.expect("install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("install SIGTERM handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
    tracing::info!("shutting down");
}
