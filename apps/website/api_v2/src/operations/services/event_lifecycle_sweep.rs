//! The convergence pass that makes the stored `events.status` column agree with the derived
//! status in [`super::event_status_rules`].
//!
//! Nothing user-visible depends on this having run: reads and the registration guard derive
//! their answer from the current statement time inside Postgres. What the pass buys is a stored
//! column an operator can trust in `psql` and an audit row naming each automatic move.

use sqlx::PgPool;
use uuid::Uuid;

use super::event_lifecycle_transition::advance_locked_event;
use crate::operations::models::EventStatus;
use crate::operations::services::event_status_rules::{
    EFFECTIVE_STATUS_SQL, EVENT_END_HORIZON_SQL, sql,
};

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
///     their answer from statement time, so a sweep that is a minute — or a day —
///     behind cannot let anyone register for a started operation. The worst outcome is a
///     stale `status` column in `psql` and a late audit row.
///   * TWO API INSTANCES both sweep. `pg_try_advisory_xact_lock` means only one does the
///     work in any given moment and the other returns immediately rather than queueing, so
///     a long pass cannot pile up runners. Even without the lock the writes are safe: both
///     decisions are re-read after event locks, so a double run updates zero rows the second
///     time and cannot double-audit.
///   * CLOCK SKEW is not a factor. No comparison uses the API process clock; statement time,
///     `start_time` and the end horizon are all evaluated by Postgres.
///   * All candidate events are locked in UUID order with `FOR NO KEY UPDATE`, which admits
///     foreign-key `KEY SHARE` locks. Schedule writers share this event lock before changing
///     event or mission times. Decisions use a fresh statement snapshot after lock acquisition.
///   * The transitions and required audits share ONE transaction and are ordered, so a long-past `scheduled`
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

    // Both transition phases use this one UUID-ordered lock set. Taking separate phase locks
    // could lock a later UUID before an earlier live event and invert a schedule writer's order.
    // This query discovers candidates only: a concurrent schedule edit can commit while it waits.
    let locked: Vec<Uuid> = sqlx::query_scalar(sql(format!(
        "SELECT e.id FROM events e WHERE e.deleted_at IS NULL AND \
         ((e.status IN ('scheduled', 'open', 'locked') AND statement_timestamp() >= e.start_time) \
          OR (e.status = 'live' AND statement_timestamp() >= {EVENT_END_HORIZON_SQL})) \
         ORDER BY e.id ASC FOR NO KEY UPDATE OF e"
    )))
    .fetch_all(&mut *tx)
    .await?;

    // Read after every parent lock is held. Schedule writers use the same transition service
    // before changing dates, so a completed operation cannot reopen depending on sweep timing.
    let observations: Vec<(Uuid, EventStatus, EventStatus)> = sqlx::query_as(sql(format!(
        "SELECT e.id, e.status, {} AS observed FROM events e WHERE e.id = ANY($1) AND e.deleted_at IS NULL ORDER BY e.id", &*EFFECTIVE_STATUS_SQL)))
        .bind(&locked).fetch_all(&mut *tx).await?;
    let mut started = Vec::new();
    let mut completed = Vec::new();
    for (id, stored, observed) in observations {
        let (became_live, became_complete) =
            advance_locked_event(&mut tx, id, stored, observed).await?;
        if became_live {
            started.push(id);
        }
        if became_complete {
            completed.push(id);
        }
    }
    tx.commit().await?;
    Ok((started, completed))
}
