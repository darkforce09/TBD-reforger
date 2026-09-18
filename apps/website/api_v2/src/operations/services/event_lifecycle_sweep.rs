//! The convergence pass that makes the stored `events.status` column agree with the derived
//! status in [`super::event_status_rules`].
//!
//! Nothing user-visible depends on this having run: reads and the registration guard derive
//! their answer from `now()` inside Postgres at request time. What the pass buys is a stored
//! column an operator can trust in `psql` and an audit row naming each automatic move.

use sqlx::PgPool;
use uuid::Uuid;

use crate::administration::models::audit_log::AuditSeverity;
use crate::administration::services::audit_writer::write_audit;
use crate::models::EventStatus;
use crate::operations::services::event_status_rules::{EVENT_END_HORIZON_SQL, sql};

/// Postgres advisory-lock key for the lifecycle sweep. Arbitrary but fixed: every API
/// instance must pick the same number or the lock does nothing.
const LIFECYCLE_LOCK_KEY: i64 = 0x7BD_0225;

/// Run one convergence pass: move started operations to `live`, then finished ones to
/// `completed`, and audit both.
///
/// ══ WHY THIS BACKGROUND TASK IS SAFE ═══════════════════════════════════════════════════
/// It is defensible as a background task only because it is not load-bearing:
///
///   * A SLOW OR STUCK PASS decides nothing. The registration guard and every read derive
///     their answer from `now()` at request time, so a sweep that is a minute — or a day —
///     behind cannot let anyone register for a started operation. The worst outcome is a
///     stale `status` column in `psql` and a late audit row.
///   * TWO API INSTANCES both sweep. `pg_try_advisory_xact_lock` means only one does the
///     work in any given moment and the other returns immediately rather than queueing, so
///     a long pass cannot pile up runners. Even without the lock the writes are safe: both
///     statements are conditional on the state they are leaving (`status IN (…)`), and the
///     rows are taken `FOR UPDATE`, so a double run updates zero rows the second time and
///     cannot double-audit.
///   * CLOCK SKEW is not a factor. No comparison uses the API process clock; `now()`,
///     `start_time` and the end horizon are all evaluated by Postgres.
///   * The two statements share ONE transaction and are ordered, so a long-past `scheduled`
///     event that nobody ever ran is stepped `scheduled → live → completed` in a single
///     pass — both legal edges of [`super::event_status_rules::can_transition`], never the
///     illegal shortcut.
///
/// Returns `(started, completed)` for logging/tests.
pub async fn sweep_once(pool: &PgPool) -> sqlx::Result<(Vec<Uuid>, Vec<Uuid>)> {
    let mut tx = pool.begin().await?;

    // Held for the transaction, so it is released on commit, rollback OR panic.
    let acquired: bool = sqlx::query_scalar("SELECT pg_try_advisory_xact_lock($1)")
        .bind(LIFECYCLE_LOCK_KEY)
        .fetch_one(&mut *tx)
        .await?;
    if !acquired {
        return Ok((Vec::new(), Vec::new()));
    }

    // 1. Start: every pre-start operation whose start time has arrived. Selected first so
    //    the audit row can name the state it actually left.
    let starting: Vec<(Uuid, EventStatus)> = sqlx::query_as(
        "SELECT id, status FROM events \
         WHERE deleted_at IS NULL AND status IN ('scheduled', 'open', 'locked') \
           AND now() >= start_time \
         ORDER BY start_time ASC FOR UPDATE",
    )
    .fetch_all(&mut *tx)
    .await?;
    let started: Vec<Uuid> = starting.iter().map(|(id, _)| *id).collect();
    if !started.is_empty() {
        sqlx::query("UPDATE events SET status = 'live', updated_at = now() WHERE id = ANY($1)")
            .bind(&started)
            .execute(&mut *tx)
            .await?;
    }

    // 2. Complete: every live operation past its end horizon — including the ones just
    //    flipped above, which this statement sees because it is the same transaction.
    let completed: Vec<Uuid> = sqlx::query_scalar(sql(format!(
        "SELECT e.id FROM events e \
         WHERE e.deleted_at IS NULL AND e.status = 'live' AND now() >= {EVENT_END_HORIZON_SQL} \
         ORDER BY e.start_time ASC FOR UPDATE OF e"
    )))
    .fetch_all(&mut *tx)
    .await?;
    if !completed.is_empty() {
        sqlx::query(
            "UPDATE events SET status = 'completed', updated_at = now() WHERE id = ANY($1)",
        )
        .bind(&completed)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    // Audit AFTER commit: `write_audit` is best-effort and must not hold the sweep's locks.
    // Registration closes at `live`, so that row is the one an operator needs when someone
    // asks why they could not sign up.
    for (id, from) in &starting {
        write_audit(
            pool,
            AuditSeverity::Info,
            None,
            "system",
            "event.auto_live",
            &format!(
                "start time reached: {} → live; registration closed",
                from.as_str()
            ),
            "event",
            &id.to_string(),
        )
        .await;
    }
    for id in &completed {
        write_audit(
            pool,
            AuditSeverity::Info,
            None,
            "system",
            "event.auto_completed",
            "end horizon passed: live → completed",
            "event",
            &id.to_string(),
        )
        .await;
    }

    Ok((started, completed))
}
