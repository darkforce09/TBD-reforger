//! Canonical deployment and attendance aggregates used by ingestion and identity changes.
//! Business transactions recompute before commit; maintenance wrappers report recoverable failures.
use super::leaderboard_view::refresh_leaderboard;
use crate::administration::models::audit_log::AuditSeverity;
use crate::administration::services::audit_writer::write_audit;
use crate::core::error_handling::api_error::ApiError;
use sqlx::{PgConnection, PgPool};

/// One snapshot calculation shared by displayed rates and transactionally maintained caches.
/// `$1` identifies the account; scheduled time is evaluated by PostgreSQL.
pub const ATTENDANCE_RATE_SQL: &str =
    "SELECT COALESCE(ROUND(100.0 * count(*) FILTER (WHERE r.attendance_state::text = 'attended')
    / NULLIF(count(*), 0), 2), 0) FROM event_registrations r
    JOIN event_missions em ON em.id = r.event_mission_id
    WHERE r.discord_id = $1 AND em.start_time <= statement_timestamp() AND r.attendance_state IS NOT NULL";

/// Recompute committed facts while serializing with account changes.
pub async fn recompute_user_stats(pool: &PgPool, discord_id: &str) -> Result<(), ApiError> {
    let mut tx = pool.begin().await?;
    sqlx::query("SELECT discord_id FROM users WHERE discord_id = $1 FOR NO KEY UPDATE")
        .bind(discord_id)
        .fetch_optional(&mut *tx)
        .await?;
    recompute_user_stats_on_connection(&mut tx, discord_id).await?;
    tx.commit().await?;
    Ok(())
}

/// One statement derives both counters from the same snapshot, including this transaction's writes.
/// Attendance rate uses decided observations only; pending, waitlisted, and withdrawn signups
/// without attendance evidence do not become no-shows merely because scheduled time has passed.
pub async fn recompute_user_stats_on_connection(
    connection: &mut PgConnection,
    discord_id: &str,
) -> Result<(), ApiError> {
    let mut query = sqlx::QueryBuilder::<sqlx::Postgres>::new(
        "UPDATE users SET total_deployments = (
        SELECT count(DISTINCT match_id) FROM match_player_stats WHERE discord_id = $1),
        attendance_rate = (",
    );
    query
        .push(ATTENDANCE_RATE_SQL)
        .push(") WHERE discord_id = $1");
    query.build().bind(discord_id).execute(connection).await?;
    Ok(())
}

/// Recompute aggregates, recording a warning if this maintenance operation fails.
pub async fn recompute_user_stats_best_effort(pool: &PgPool, discord_id: &str, message: &str) {
    if recompute_user_stats(pool, discord_id).await.is_err() {
        write_audit(
            pool,
            AuditSeverity::Warn,
            None,
            "system",
            "user.stats_recompute_failed",
            message,
            "user",
            discord_id,
        )
        .await;
    }
}

/// Refresh the leaderboard, recording a warning on the caller's target if maintenance fails.
pub async fn refresh_leaderboard_best_effort(
    pool: &PgPool,
    message: &str,
    target_type: &str,
    target_id: &str,
) {
    if refresh_leaderboard(pool).await.is_err() {
        write_audit(
            pool,
            AuditSeverity::Warn,
            None,
            "system",
            "leaderboard.refresh_failed",
            message,
            target_type,
            target_id,
        )
        .await;
    }
}
