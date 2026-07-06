//! API server entrypoint — Rust port of `cmd/api/main.go`.
//!
//! Boot order (mirrors the Go `main`): load config → open pool (with backoff) →
//! run migrations → build the router + middleware ([`reforger_backend::app::router`])
//! → serve on `:PORT` with graceful shutdown (SIGINT/SIGTERM). The `/api/v1` route
//! tree + refresh-token purge task are wired in as later phases land.

use std::net::SocketAddr;

use reforger_backend::config::Config;
use reforger_backend::state::AppState;
use reforger_backend::{app, db, services};
use tracing_subscriber::EnvFilter;

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
