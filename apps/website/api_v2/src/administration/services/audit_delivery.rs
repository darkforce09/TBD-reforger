//! The replayable audit delivery stream behind `GET /api/v1/admin/audit-logs/stream`.
//!
//! **Role:** turns the durable publication sequence into one ordered stream per client: a `ready`
//! item, then every published audit row after the client's cursor, with a `reset` wherever the
//! requested history cannot be replayed.
//! **Position:** reads `audit_publication_state`, `audit_publications` and `audit_logs`; publishes
//! pending rows through [`super::audit_publication::publish_audit_batch`]; woken by
//! [`super::audit_notifier::AuditNotify`] and by its own timer; consumed by
//! [`crate::administration::handlers::audit_logs`], which writes each item as one SSE event.
//! **Signals & state:** each stream owns its cursor (the last delivered publication sequence) and
//! one broadcast receiver; nothing is shared between streams.
//! **Invariants:**
//! - Rows are delivered in ascending publication sequence, never in audit id order.
//! - `ready` is the first item; its `resume_after` is the requested cursor, or the tail.
//! - A cursor above the tail resets with `cursor_ahead`, a cursor below the retained floor with
//!   `history_unavailable`, at open and on every wake; after a reset the cursor is the tail.
//! - Every page is read in one repeatable-read snapshot together with the floor it is checked
//!   against, so a sequence removed between the check and the read cannot be skipped silently.
//! - A publish failure is logged and the read still runs; a read failure is logged, keeps the
//!   cursor, and is retried on the next timer tick whether or not the listener is healthy.

use std::time::Duration;

use async_stream::stream;
use futures::Stream;
use sqlx::PgPool;
use tokio::sync::broadcast;

use super::audit_notifier::AuditNotify;
use super::audit_publication::publish_audit_batch;
use crate::administration::models::audit_log::AuditLog;
use crate::administration::models::audit_stream::{
    AuditStreamReady, AuditStreamReset, AuditStreamResetReason,
};

/// One published audit row and the publication sequence it was delivered under.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AuditDelivery {
    /// The publication sequence: the SSE id and the replay cursor.
    pub sequence: i64,
    #[sqlx(flatten)]
    pub row: AuditLog,
}

/// What the delivery stream yields, in the order a client receives it.
#[derive(Debug, Clone)]
pub enum AuditStreamItem {
    /// The opening item: where the stream starts and the retained floor at open.
    Ready(AuditStreamReady),
    /// One published audit row.
    Delivery(AuditDelivery),
    /// The requested history cannot be replayed; the stream continues after the tail.
    Reset(AuditStreamReset),
}

/// The published tail and the retained floor, read together from `audit_publication_state`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::FromRow)]
pub struct PublicationBounds {
    /// `last_sequence`: the newest publication sequence.
    pub tail: i64,
    /// `retained_after_sequence`: every sequence at or below it may be missing.
    pub retained_after: i64,
}

/// Rows read per page; a full page is followed by another read before the stream waits again.
const PAGE_SIZE: i64 = 100;

/// Pending audit rows published per wake.
const PUBLISH_BATCH: i64 = 1000;

const BOUNDS: &str = "SELECT last_sequence AS tail, retained_after_sequence AS retained_after \
     FROM audit_publication_state WHERE singleton = true";

const ROWS: &str = "SELECT p.sequence, a.id, a.severity, a.actor_id, \
     COALESCE(a.actor_name, '') AS actor_name, a.action, a.message, \
     COALESCE(a.target_type, '') AS target_type, COALESCE(a.target_id, '') AS target_id, \
     a.metadata, COALESCE(a.created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at \
     FROM audit_publications p JOIN audit_logs a ON a.id = p.audit_id \
     WHERE p.sequence > $1 ORDER BY p.sequence ASC LIMIT $2";

/// Why `cursor` cannot be replayed against `bounds`, or `None` when every sequence after it up to
/// the tail is retained.
///
/// A cursor equal to the floor is replayable: only sequences at or below the floor may be missing.
pub fn reset_reason(cursor: i64, bounds: PublicationBounds) -> Option<AuditStreamResetReason> {
    if cursor > bounds.tail {
        Some(AuditStreamResetReason::CursorAhead)
    } else if cursor < bounds.retained_after {
        Some(AuditStreamResetReason::HistoryUnavailable)
    } else {
        None
    }
}

/// The reset that moves a stream to the tail of `bounds` for `reason`.
pub fn reset_to_tail(
    reason: AuditStreamResetReason,
    bounds: PublicationBounds,
) -> AuditStreamReset {
    AuditStreamReset {
        reason,
        resume_after: bounds.tail,
        retained_after: bounds.retained_after,
    }
}

/// Where a stream opened with `requested` starts: its `ready` item, the reset that follows it when
/// the requested history cannot be replayed, and the cursor the first read continues after.
///
/// Without a requested cursor the stream starts at the tail.
pub fn opening(
    requested: Option<i64>,
    bounds: PublicationBounds,
) -> (AuditStreamReady, Option<AuditStreamReset>, i64) {
    let resume_after = requested.unwrap_or(bounds.tail);
    let ready = AuditStreamReady {
        resume_after,
        retained_after: bounds.retained_after,
    };
    match reset_reason(resume_after, bounds) {
        Some(reason) => (ready, Some(reset_to_tail(reason, bounds)), bounds.tail),
        None => (ready, None, resume_after),
    }
}

/// The bounds and, when the cursor is replayable, the page after it, read in one snapshot.
struct PublicationWindow {
    bounds: PublicationBounds,
    deliveries: Vec<AuditDelivery>,
}

/// Reads the bounds and the page after `cursor` in one read-only repeatable-read transaction, so
/// the page is exactly what the floor it was checked against promises. A cursor the bounds refuse
/// reads no rows.
async fn read_window(pool: &PgPool, cursor: i64) -> Result<PublicationWindow, sqlx::Error> {
    let mut snapshot = pool.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ, READ ONLY")
        .execute(&mut *snapshot)
        .await?;
    let bounds: PublicationBounds = sqlx::query_as(BOUNDS).fetch_one(&mut *snapshot).await?;
    let deliveries = if reset_reason(cursor, bounds).is_some() {
        Vec::new()
    } else {
        sqlx::query_as(ROWS)
            .bind(cursor)
            .bind(PAGE_SIZE)
            .fetch_all(&mut *snapshot)
            .await?
    };
    snapshot.commit().await?;
    Ok(PublicationWindow { bounds, deliveries })
}

/// Publishes pending audit rows; a failure is logged and never stops the read that follows,
/// because rows published by the worker or another stream are still there to deliver.
async fn publish_pending(pool: &PgPool) {
    if let Err(error) = publish_audit_batch(pool, PUBLISH_BATCH).await {
        tracing::warn!(%error, "audit publication failed; reading what is already published");
    }
}

/// The delivery stream for one client, starting after `resume_after` (the tail when `None`).
///
/// The broadcast subscription is taken before the opening bounds are read, so no row published
/// after the snapshot goes unannounced. Each wake (a notifier signal or a timer tick) publishes
/// pending rows, then reads pages until one comes back short. The timer fires every `poll_every`
/// whatever the listener's health, because a healthy notification channel does not prove a
/// successful read.
///
/// # Errors
/// Reading the opening bounds fails the call; nothing has been streamed yet.
pub async fn audit_delivery_stream(
    pool: PgPool,
    notify: AuditNotify,
    poll_every: Duration,
    resume_after: Option<i64>,
) -> Result<impl Stream<Item = AuditStreamItem> + Send, sqlx::Error> {
    let mut receiver = notify.subscribe();
    publish_pending(&pool).await;
    let bounds: PublicationBounds = sqlx::query_as(BOUNDS).fetch_one(&pool).await?;
    let (ready, opening_reset, mut cursor) = opening(resume_after, bounds);
    Ok(stream! {
        yield AuditStreamItem::Ready(ready);
        if let Some(reset) = opening_reset {
            yield AuditStreamItem::Reset(reset);
        }
        let mut interval = tokio::time::interval(poll_every.max(Duration::from_millis(1)));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        let mut listener_closed = false;
        loop {
            tokio::select! {
                _ = interval.tick() => {},
                signal = receiver.recv(), if !listener_closed => {
                    if let Err(broadcast::error::RecvError::Closed) = signal {
                        listener_closed = true;
                    }
                },
            }
            if pool.is_closed() {
                break;
            }
            // Every signal means "read again"; one read covers all that are already queued.
            while receiver.try_recv().is_ok() {}
            publish_pending(&pool).await;
            loop {
                let window = match read_window(&pool, cursor).await {
                    Ok(window) => window,
                    Err(error) => {
                        tracing::warn!(%error, cursor, "audit replay read failed; retrying on the next wake");
                        break;
                    }
                };
                if let Some(reason) = reset_reason(cursor, window.bounds) {
                    cursor = window.bounds.tail;
                    yield AuditStreamItem::Reset(reset_to_tail(reason, window.bounds));
                    continue;
                }
                let full = window.deliveries.len() as i64 == PAGE_SIZE;
                for delivery in window.deliveries {
                    cursor = delivery.sequence;
                    yield AuditStreamItem::Delivery(delivery);
                }
                if !full {
                    break;
                }
            }
        }
    })
}

#[cfg(test)]
#[path = "tests/audit_delivery.rs"]
mod tests;
