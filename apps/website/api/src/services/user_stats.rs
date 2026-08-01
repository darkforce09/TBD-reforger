//! Denormalized user statistics — `users.total_deployments` + `users.attendance_rate`,
//! and the best-effort `leaderboard_totals` refresh that always travels with them (T-336).
//!
//! # Why this is a service and not a handler internal
//!
//! [`recompute_user_stats`] is the **sole writer** of those two columns. It shipped at T-326
//! as `pub(super) fn` inside `handlers/telemetry.rs` because `handlers/me.rs` needed it at
//! identity-link time and `pub(super)` was the minimal unblock. T-326 explicitly refused to
//! re-derive the SQL in `me.rs` — two definitions of "a deployment" drifting apart is the same
//! silent-wrong-number failure the backfill was filed to fix — and that refusal is the whole
//! argument for this file: a function two handlers depend on is a service, not a handler
//! internal, and `handlers/telemetry.rs` is not a place other handlers should be reaching into.
//!
//! **Nothing about the behaviour changed in the move.** The three statements, their bind order,
//! the `count(DISTINCT match_id)` / `state::text = 'attended'` / `start_time <= now()` predicates
//! and the zero-denominator rule are byte-for-byte what T-326 shipped. `t336_user_stats_service`
//! pins the numbers from outside the crate, which is only possible now that this is `pub`.
//!
//! # The best-effort pair
//!
//! Three call sites (`telemetry::ingest_match_results`, `me::unlink`, `me::ingest_link_confirm`)
//! had the identical `if … .await.is_err() { write_audit(Warn, …) }` block around
//! [`crate::db::refresh_leaderboard`], differing only in the message and the audit target.
//! T-336 asked for that pattern to come along "if it also has two callers by then"; it had
//! three. [`refresh_leaderboard_best_effort`] and [`recompute_user_stats_best_effort`] are that
//! block, once.
//!
//! Both are deliberately **infallible**. These refresh derived numbers *after* their caller's
//! transaction has committed: the rows are already correct, the next ingest recomputes them
//! anyway, and failing the request here would turn a cosmetic staleness into a 500 that makes a
//! successful link look like a failed one. What must never happen is that the failure is
//! *silent* — hence the `Warn` audit row, which is the observable this file guarantees.

use sqlx::PgPool;

use crate::db::refresh_leaderboard;
use crate::error::ApiError;
use crate::models::AuditSeverity;
use crate::services::write_audit;

/// Recompute a user's denormalized deployment + attendance metrics.
///
/// The identity-link confirm backfills `match_player_stats.discord_id` for matches played
/// before the link existed, and unlink releases them again — both change exactly the two counts
/// this function derives, so both have to call it or `users.total_deployments` reports a number
/// the rows contradict. Measured before it was reachable: a player with three claimed pre-link
/// matches still read `total_deployments = 0`, and for anyone who links *after* their last op
/// nothing else ever recomputes it.
///
/// Takes `&PgPool` rather than a transaction on purpose: it reads committed state, so callers
/// must run it *after* their commit, never inside it.
pub async fn recompute_user_stats(pool: &PgPool, discord_id: &str) -> Result<(), ApiError> {
    let deployments: i64 = sqlx::query_scalar(
        "SELECT count(DISTINCT match_id) FROM match_player_stats WHERE discord_id = $1",
    )
    .bind(discord_id)
    .fetch_one(pool)
    .await?;
    let attended: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM event_registrations WHERE discord_id = $1 AND state::text = 'attended'",
    )
    .bind(discord_id)
    .fetch_one(pool)
    .await?;
    let past_registered: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM event_registrations \
         JOIN event_missions ON event_missions.id = event_registrations.event_mission_id \
         WHERE event_registrations.discord_id = $1 AND event_missions.start_time <= now()",
    )
    .bind(discord_id)
    .fetch_one(pool)
    .await?;
    let rate = if past_registered > 0 {
        attended as f64 / past_registered as f64 * 100.0
    } else {
        0.0
    };
    sqlx::query(
        "UPDATE users SET total_deployments = $1, attendance_rate = $2::float8::numeric WHERE discord_id = $3",
    )
    .bind(deployments)
    .bind(rate)
    .bind(discord_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// [`recompute_user_stats`], with a `Warn` audit row instead of an error.
///
/// `message` is the caller's "…after X" sentence; the audit target is always the user, because
/// that is the row whose numbers went stale.
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

/// [`crate::db::refresh_leaderboard`], with a `Warn` audit row instead of an error.
///
/// The target is the caller's, not the user's: a refresh failure after match ingest is about the
/// match, and after an identity link it is about the user. Both are wanted in the audit console,
/// so neither is hard-coded here.
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
