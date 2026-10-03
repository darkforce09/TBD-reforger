//! Serialized refresh of the materialized leaderboard and its contributing transaction.
//!
//! **Role:** refreshes the `leaderboard_totals` materialized view under one advisory lock.
//! **Position:** called last inside the match results and identity link transactions, and on a
//! schedule by the leaderboard refresher worker.
//! **Signals & state:** the transaction-scoped advisory lock `tbd.leaderboard_totals.refresh`.
//! **Invariants:** a refresher takes the lock before its snapshot, so no refresh publishes a
//! snapshot older than one already committed.

use sqlx::{PgConnection, PgPool};

/// Maintenance refreshes use the same publisher lock as business transactions.
pub async fn refresh_leaderboard(pool: &PgPool) -> sqlx::Result<()> {
    let mut tx = pool.begin().await?;
    refresh_leaderboard_on_connection(&mut tx).await?;
    tx.commit().await
}

/// Acquire the final lock in a business transaction before taking the aggregate snapshot.
/// The next refresher cannot capture an older snapshot and publish over this transaction's result.
pub async fn refresh_leaderboard_on_connection(connection: &mut PgConnection) -> sqlx::Result<()> {
    sqlx::query(
        "SELECT pg_advisory_xact_lock(hashtextextended('tbd.leaderboard_totals.refresh', 0))",
    )
    .execute(&mut *connection)
    .await?;
    sqlx::query("REFRESH MATERIALIZED VIEW leaderboard_totals")
        .execute(connection)
        .await?;
    Ok(())
}
