//! Refresh of the `leaderboard_totals` materialized view.

use sqlx::postgres::PgPool;

/// Refresh the `leaderboard_totals` materialized view. Call after match telemetry
/// ingest (debounced). Falls back to a non-concurrent refresh if the concurrent one
/// fails (e.g. the view has not been populated yet).
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
