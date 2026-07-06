//! Database layer — Rust port of `internal/db` (pool + migration runner + MV refresh).
//!
//! The migration pipeline is a single frozen `migrations/0001_initial_schema.sql`
//! (the Go GORM-AutoMigrate + raw-SQL schema, proven byte-equal by gate G2). sqlx
//! embeds it at compile time via `migrate!`; future schema changes add new files.

use std::time::Duration;

use sqlx::postgres::{PgPool, PgPoolOptions};

/// Startup connection retry budget (mirrors `db.Open`: 10 attempts, linear backoff).
const CONNECT_ATTEMPTS: u32 = 10;

/// Connect to Postgres, tuning the pool and retrying the initial connection with
/// linear backoff (Postgres can briefly refuse connections just after reporting ready).
///
/// Mirrors `db.Open`: MaxOpen 25, ConnMaxLifetime 30m, ConnMaxIdleTime 5m.
pub async fn connect(database_url: &str) -> Result<PgPool, sqlx::Error> {
    let opts = PgPoolOptions::new()
        .max_connections(25)
        .idle_timeout(Duration::from_secs(5 * 60))
        .max_lifetime(Duration::from_secs(30 * 60))
        .acquire_timeout(Duration::from_secs(30));

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
pub fn connect_lazy(database_url: &str) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(25)
        .connect_lazy(database_url)
}

/// Run all pending migrations (embedded from `./migrations` at compile time).
///
/// Mirrors `db.Migrate` — the pre/AutoMigrate/post pipeline is collapsed into the
/// single frozen `0001_initial_schema.sql`.
pub async fn migrate(pool: &PgPool) -> Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!("./migrations").run(pool).await
}

/// Refresh the `leaderboard_totals` materialized view. Call after match telemetry
/// ingest (debounced). Falls back to a non-concurrent refresh if the concurrent one
/// fails (e.g. the view has not been populated yet). Mirrors `db.RefreshLeaderboard`.
pub async fn refresh_leaderboard(pool: &PgPool) -> Result<(), sqlx::Error> {
    if sqlx::query("REFRESH MATERIALIZED VIEW CONCURRENTLY leaderboard_totals")
        .execute(pool)
        .await
        .is_err()
    {
        sqlx::query("REFRESH MATERIALIZED VIEW leaderboard_totals")
            .execute(pool)
            .await?;
    }
    Ok(())
}
