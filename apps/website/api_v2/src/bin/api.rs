//! API server entrypoint.
//!
//! Boot order: load config → open pool (with backoff) → run migrations → spawn the background
//! workers → build the router + middleware ([`website_api::core::http_router::router`]) → serve
//! on `:PORT` with graceful shutdown (SIGINT/SIGTERM).

use std::net::SocketAddr;

use tracing_subscriber::EnvFilter;
use website_api::background_workers;
use website_api::core::application_state::AppState;
use website_api::core::configuration::Config;
use website_api::core::{database, http_router};

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
    // Every interval task the API runs, armed in one place. The handles stay owned for the
    // life of the process; what each worker does and why a missed tick is survivable is
    // documented on the worker module itself.
    let _workers = background_workers::spawn_all(&state);
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
