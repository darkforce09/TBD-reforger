//! Postgres connection lifecycle: opening a tuned pool with a startup retry budget, and running
//! the embedded migration pipeline.
//!
//! The migration pipeline is a single frozen `migrations/0001_initial_schema.sql`; sqlx embeds
//! it at compile time via `migrate!`, and future schema changes add new files beside it.

pub mod connection_pool;
pub mod leaderboard_refresh;

use std::time::Duration;

use sqlx::postgres::{PgPool, PgPoolOptions};

use connection_pool::{DbPoolConfig, pool_options};

/// Startup connection retry budget: 10 attempts, linear backoff.
const CONNECT_ATTEMPTS: u32 = 10;

/// Connect to Postgres with the pool tuned from `TBD_DB_POOL_*` ([`DbPoolConfig::from_env`]),
/// retrying the initial connection with linear backoff (Postgres can briefly refuse
/// connections just after reporting ready).
///
/// A variable that does not parse fails HERE, before any connection attempt, as
/// [`sqlx::Error::Configuration`] wrapping the [`ConfigError`] that names it — so the API
/// binary's `connect(&cfg.database_url)?` stops startup with that message, and every caller
/// that opens a pool without loading [`Config`] (`import-registry`, the integration suites)
/// gets the same guard.
///
/// [`ConfigError`]: crate::core::configuration::ConfigError
/// [`Config`]: crate::core::configuration::Config
pub async fn connect(database_url: &str) -> Result<PgPool, sqlx::Error> {
    connect_with_options(database_url, env_pool_options()?).await
}

/// The [`PgPoolOptions`] [`connect`] builds: [`DbPoolConfig::from_env`] through
/// [`pool_options`], a malformed variable surfaced as [`sqlx::Error::Configuration`].
fn env_pool_options() -> Result<PgPoolOptions, sqlx::Error> {
    DbPoolConfig::from_env()
        .map(|cfg| pool_options(&cfg))
        .map_err(|e| sqlx::Error::Configuration(e.into()))
}

async fn connect_with_options(
    database_url: &str,
    opts: PgPoolOptions,
) -> Result<PgPool, sqlx::Error> {
    let mut last_err: Option<sqlx::Error> = None;
    for attempt in 1..=CONNECT_ATTEMPTS {
        match opts.clone().connect(database_url).await {
            Ok(pool) => return Ok(pool),
            Err(e) => {
                last_err = Some(e);
                tokio::time::sleep(Duration::from_millis(u64::from(attempt) * 250)).await;
            }
        }
    }
    Err(last_err.expect("loop runs at least once"))
}

/// Build a pool that connects lazily (on first use). Used by tests/harnesses that
/// exercise code paths not reaching the DB, without requiring a live server.
///
/// Deliberately NOT env-tuned: a harness pool must neither change shape nor fail to build
/// because the developer's shell happens to export a `TBD_DB_POOL_*` value.
pub fn connect_lazy(database_url: &str) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(DbPoolConfig::default().max_connections)
        .connect_lazy(database_url)
}

/// Run all pending migrations (embedded from `./migrations` at compile time).
pub async fn migrate(pool: &PgPool) -> Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!("./migrations").run(pool).await
}

#[cfg(test)]
#[path = "tests/connection.rs"]
mod tests;
