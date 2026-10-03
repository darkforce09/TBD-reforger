//! API server entrypoint.
//!
//! **Role:** the `api` binary: loads the configuration, opens the pool (with backoff), applies
//! the migrations, spawns the background workers, builds the router and middleware
//! ([`api::router::router`]) and serves on `:PORT` until SIGINT or SIGTERM.
//! **Position:** the composition root of the `api` crate; the systemd unit, the release
//! image and `cargo xtask mk rust-api` start it.
//! **Signals & state:** the process's Tokio runtime; the worker handles live until `main`
//! returns; SIGINT or SIGTERM begins
//! [`api_configuration::process_lifecycle::process_shutdown`].
//! **Invariants:** shutdown begins the process-wide shutdown signal at the moment `axum::serve`
//! stops accepting, so every open SSE stream ends with it and the graceful drain waits only for
//! ordinary requests in flight; the process then exits 0.

use std::net::SocketAddr;

use api::router::router;
use api_configuration::configuration::Config;
use api_configuration::process_lifecycle::process_shutdown;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let cfg = Config::load()?;
    let pool = api_database::connect(&cfg.database_url).await?;
    // `SKIP_MIGRATE` lets a harness that owns migration of a shared database keep this
    // process from racing it.
    if std::env::var("SKIP_MIGRATE").is_err() {
        api_database::migrate(&pool).await?;
        tracing::info!(env = %cfg.env, "migrations applied");
    }

    let port = cfg.port.clone();
    let state = api::composition::application_state(pool, cfg);
    // Every interval task the API runs, armed in one place. The handles stay owned for the
    // life of the process; what each worker does and why a missed tick is survivable is
    // documented on the worker module itself.
    let _workers = api_background_workers::spawn_all(&state);
    let app = router(state);

    let addr = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("listening on {addr}");
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(termination_requested())
    .await?;
    tracing::info!("drained; exiting");
    Ok(())
}

/// Resolves on SIGINT or SIGTERM, after beginning the process-wide shutdown: `axum::serve` then
/// stops accepting and drains the requests in flight, and every open SSE stream ends instead of
/// holding the drain open.
async fn termination_requested() {
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
    process_shutdown().begin();
    tracing::info!("shutting down; open event streams are closing");
}
