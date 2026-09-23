//! Replayable audit delivery uses publication sequence, never audit allocation order.

use super::{
    audit_notifier::{AuditNotify, AuditSignal},
    audit_publication::publish_audit_batch,
};
use crate::administration::models::audit_log::AuditLog;
use async_stream::stream;
use futures::Stream;
use sqlx::PgPool;
use std::time::Duration;
use tokio::sync::broadcast;

#[derive(Debug, sqlx::FromRow)]
pub struct AuditDelivery {
    pub sequence: i64,
    #[sqlx(flatten)]
    pub row: AuditLog,
}

const PAGE: i64 = 100;
const ROWS: &str = "SELECT p.sequence, a.id, a.severity, a.actor_id, COALESCE(a.actor_name, '') AS actor_name, a.action, a.message, COALESCE(a.target_type, '') AS target_type, COALESCE(a.target_id, '') AS target_id, a.metadata, COALESCE(a.created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at FROM audit_publications p JOIN audit_logs a ON a.id = p.audit_id WHERE p.sequence > $1 ORDER BY p.sequence ASC LIMIT $2";

/// A missing cursor starts at the current published tail. A supplied cursor replays later rows.
/// Subscription precedes the tail snapshot. Every fetch retries on the timer even if LISTEN
/// remains connected, because notification health does not imply a successful database read.
pub async fn audit_delivery_stream(
    pool: PgPool,
    notify: AuditNotify,
    poll_every: Duration,
    resume_after: Option<i64>,
) -> Result<impl Stream<Item = AuditDelivery> + Send, sqlx::Error> {
    let mut receiver = notify.subscribe();
    publish_audit_batch(&pool, 1000).await?;
    let tail: i64 = sqlx::query_scalar(
        "SELECT last_sequence FROM audit_publication_state WHERE singleton = true",
    )
    .fetch_one(&pool)
    .await?;
    if resume_after.is_some_and(|cursor| cursor < 0 || cursor > tail) {
        return Err(sqlx::Error::Protocol(
            "audit replay cursor is outside retained publication history".into(),
        ));
    }
    let mut cursor = resume_after.unwrap_or(tail);
    Ok(stream! {
        let mut interval = tokio::time::interval(poll_every.max(Duration::from_millis(1)));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        let mut closed = false;
        loop {
            tokio::select! {
                _ = interval.tick() => {},
                signal = receiver.recv(), if !closed => match signal {
                    Ok(AuditSignal::Row(_) | AuditSignal::Resync | AuditSignal::Down) => {},
                    Err(broadcast::error::RecvError::Lagged(_)) => {},
                    Err(broadcast::error::RecvError::Closed) => { closed = true; },
                },
            }
            if pool.is_closed() { break; }
            while receiver.try_recv().is_ok() {}
            if let Err(error) = publish_audit_batch(&pool, 1000).await {
                tracing::warn!(%error, "audit publication failed; retrying on next wakeup");
                continue;
            }
            loop {
                let rows: Vec<AuditDelivery> = match sqlx::query_as(ROWS).bind(cursor).bind(PAGE).fetch_all(&pool).await {
                    Ok(rows) => rows,
                    Err(error) => {
                        tracing::warn!(%error, "audit replay read failed; retrying on next wakeup");
                        break;
                    }
                };
                let full = rows.len() as i64 == PAGE;
                for delivery in rows {
                    cursor = delivery.sequence;
                    yield delivery;
                }
                if !full { break; }
            }
        }
    })
}
