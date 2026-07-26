//! API server entrypoint — Rust port of `cmd/api/main.go`.
//!
//! Boot order (mirrors the Go `main`): load config → open pool (with backoff) →
//! run migrations → build the router + middleware ([`website_api::app::router`])
//! → serve on `:PORT` with graceful shutdown (SIGINT/SIGTERM). The `/api/v1` route
//! tree + refresh-token purge task are wired in as later phases land.

use std::net::SocketAddr;

use tracing_subscriber::EnvFilter;
use website_api::config::Config;
use website_api::state::AppState;
use website_api::{app, db, handlers, realtime, services};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let cfg = Config::load()?;
    let pool = db::connect(&cfg.database_url).await?;
    // `SKIP_MIGRATE` is used only by the G5 differential harness, where the Go server
    // owns migration of the shared database (avoids a dual-migration clash).
    if std::env::var("SKIP_MIGRATE").is_err() {
        db::migrate(&pool).await?;
        tracing::info!(env = %cfg.env, "migrations applied");
    }

    let port = cfg.port.clone();
    let state = AppState::new(pool, cfg);
    // Background refresh-token purge (immediate sweep, then every 6h).
    let _purge = services::start_refresh_token_purge(state.pool.clone());
    // Event lifecycle convergence (T-225): starts operations at their start time and
    // completes them past their end horizon, so the stored `events.status` agrees with the
    // status the handlers derive. Safe to be late or absent — the registration window and
    // every read derive from `now()` at request time, never from this task's output.
    let _lifecycle = handlers::events::start_event_lifecycle(state.pool.clone());
    // T-261: scheduled `leaderboard_totals` MV refresh — immediate + interval (env
    // `LEADERBOARD_REFRESH_INTERVAL_SECS`, default 15m). Safety net when telemetry/me
    // ingest is quiet; those callers still refresh in-request.
    let lb_interval = db::leaderboard_refresh_interval();
    tracing::info!(
        secs = lb_interval.as_secs(),
        "leaderboard MV scheduled refresh armed"
    );
    let _leaderboard = db::start_leaderboard_refresh(state.pool.clone(), lb_interval);
    // T-272: scheduled `server_statuses` → SSE republish — immediate + interval (env
    // `SERVER_STATUS_PUBLISH_INTERVAL_SECS`, default 10s). Closes the SSE loop when no
    // game-server bridge calls ingest; ingest still publishes in-request.
    let ss_interval = realtime::server_status_publish_interval();
    tracing::info!(
        secs = ss_interval.as_secs(),
        "server-status SSE republish armed"
    );
    let _server_status =
        realtime::start_server_status_publisher(state.pool.clone(), state.hub.clone(), ss_interval);
    let app = app::router(state);

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
