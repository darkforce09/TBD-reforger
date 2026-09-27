//! The audit console: the filtered list, the CSV export, and the live SSE feed.
//!
//! **Role:** the administrator's three audit routes: the keyset-paged history, the CSV export and
//! the replayable live stream.
//! **Position:** reads `audit_logs` directly for the list and the export; the stream consumes
//! [`crate::administration::services::audit_delivery::audit_delivery_stream`], which is woken by
//! [`crate::administration::services::audit_notifier`] and polls on [`AUDIT_POLL_FALLBACK`].
//! **Signals & state:** none held here; each stream owns its delivery cursor.
//! **Invariants:** a list row and a stream row serialize the same [`AuditLog`] shape; every stream
//! event's SSE id is a publication sequence, never an audit id; the stream's first event is
//! `ready`, and a cursor it cannot replay becomes `reset` to the tail, never an error status; a
//! `Last-Event-ID` that is not a non-negative integer answers 400.

use std::borrow::Cow;
use std::convert::Infallible;
use std::time::Duration;

use async_stream::stream;
use axum::extract::{Query, State};
use axum::http::{HeaderMap, HeaderName, header};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Json, Response};
use futures::Stream;
use serde::Deserialize;
use serde_json::{Value, json};
use sqlx::{PgPool, QueryBuilder};

use crate::administration::models::audit_log::AuditLog;
use crate::administration::services::audit_delivery::{AuditStreamItem, audit_delivery_stream};
use crate::administration::services::audit_notifier::AuditNotify;
use crate::core::application_state::AppState;
use crate::core::error_handling::api_error::ApiError;
use crate::core::http::pagination::PageParams;
use crate::core::middleware::AdminUser;

/// Neutralise CSV formula injection for spreadsheet consumers (Excel / Sheets).
///
/// Cells whose first character is `=`, `+`, `-`, or `@` are live formulas when the file is
/// opened. Audit fields (`actor_name`, `action`, `message`, …) are user-influenced, so the
/// export writer must prefix those cells. A leading `'` is the standard spreadsheet "treat as
/// text" marker; it survives `csv::Writer` quoting and is stripped by Excel on display.
///
/// A URL guard such as `is_http_url` does **not** cover this sink — a value that passes one is
/// still illegal to interpolate raw into CSV.
fn escape_csv_formula(cell: &str) -> Cow<'_, str> {
    match cell.as_bytes().first() {
        Some(b'=' | b'+' | b'-' | b'@') => Cow::Owned(format!("'{cell}")),
        _ => Cow::Borrowed(cell),
    }
}

#[derive(Debug, Deserialize)]
pub struct AuditFilter {
    severity: Option<String>,
    q: Option<String>,
    before: Option<i64>,
    limit: Option<i64>,
}

fn valid_severity(s: &str) -> Option<&str> {
    matches!(s, "info" | "warn" | "crit").then_some(s)
}

/// Apply `?severity=` and `?q=` filters to a running audit query builder.
fn apply_filters(qb: &mut QueryBuilder<sqlx::Postgres>, f: &AuditFilter) {
    if let Some(sev) = f.severity.as_deref().and_then(valid_severity) {
        qb.push(" AND severity::text = ").push_bind(sev.to_string());
    }
    if let Some(search) = f.q.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        qb.push(" AND message ILIKE ")
            .push_bind(format!("%{search}%"));
    }
}

/// `GET /api/v1/admin/audit-logs` — newest-first, keyset pagination via `?before=`.
///
/// @route GET /api/v1/admin/audit-logs
pub async fn list_audit_logs(
    State(state): State<AppState>,
    _a: AdminUser,
    Query(f): Query<AuditFilter>,
) -> Result<Json<Value>, ApiError> {
    let (limit, _) = PageParams {
        limit: f.limit,
        offset: None,
    }
    .bounds();

    let mut qb = QueryBuilder::new(
        "SELECT id, severity, actor_id, COALESCE(actor_name, '') AS actor_name, action, message, COALESCE(target_type, '') AS target_type, COALESCE(target_id, '') AS target_id, metadata, COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at FROM audit_logs WHERE true",
    );
    apply_filters(&mut qb, &f);
    if let Some(before) = f.before {
        qb.push(" AND id < ").push_bind(before);
    }
    qb.push(" ORDER BY id DESC LIMIT ").push_bind(limit);

    let logs: Vec<AuditLog> = qb
        .build_query_as()
        .fetch_all(&state.pool)
        .await
        .map_err(ApiError::from)?;
    let next_cursor: Option<i64> =
        (logs.len() as i64 == limit && limit > 0).then(|| logs[logs.len() - 1].id);
    Ok(Json(json!({ "data": logs, "next_cursor": next_cursor })))
}

/// `GET /api/v1/admin/audit-logs/export.csv` — filtered CSV download.
///
/// @route GET /api/v1/admin/audit-logs/export.csv
pub async fn export_audit_logs_csv(
    State(state): State<AppState>,
    _a: AdminUser,
    Query(f): Query<AuditFilter>,
) -> Result<Response, ApiError> {
    let mut qb = QueryBuilder::new(
        "SELECT id, severity, actor_id, COALESCE(actor_name, '') AS actor_name, action, message, COALESCE(target_type, '') AS target_type, COALESCE(target_id, '') AS target_id, metadata, COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at FROM audit_logs WHERE true",
    );
    apply_filters(&mut qb, &f);
    qb.push(" ORDER BY id DESC LIMIT 10000");
    let logs: Vec<AuditLog> = qb
        .build_query_as()
        .fetch_all(&state.pool)
        .await
        .map_err(ApiError::from)?;

    let mut w = csv::Writer::from_writer(Vec::new());
    let _ = w.write_record([
        "timestamp",
        "severity",
        "actor",
        "action",
        "message",
        "target_type",
        "target_id",
    ]);
    for l in &logs {
        let ts = l
            .created_at
            .to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        let _ = w.write_record([
            escape_csv_formula(&ts).as_ref(),
            escape_csv_formula(l.severity.as_str()).as_ref(),
            escape_csv_formula(&l.actor_name).as_ref(),
            escape_csv_formula(&l.action).as_ref(),
            escape_csv_formula(&l.message).as_ref(),
            escape_csv_formula(&l.target_type).as_ref(),
            escape_csv_formula(&l.target_id).as_ref(),
        ]);
    }
    let body = w.into_inner().unwrap_or_default();

    Ok((
        [
            (header::CONTENT_TYPE, "text/csv".to_string()),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"audit-logs.csv\"".to_string(),
            ),
        ],
        body,
    )
        .into_response())
}

/// Retry cadence for durable audit publication and replay reads.
pub const AUDIT_POLL_FALLBACK: Duration = Duration::from_secs(2);

/// The audit rows of a delivery stream opened at the tail, without its control items.
///
/// The listener suite reads rows through this; HTTP clients use [`stream_audit_logs`], whose
/// `ready` and `reset` events this stream leaves out.
///
/// # Panics
/// When the opening publication bounds cannot be read.
pub async fn audit_row_stream(
    pool: PgPool,
    notify: AuditNotify,
    poll_every: Duration,
) -> impl Stream<Item = AuditLog> + Send {
    let items = audit_delivery_stream(pool, notify, poll_every, None)
        .await
        .expect("open the audit delivery stream");
    stream! {
        for await item in items {
            if let AuditStreamItem::Delivery(delivery) = item {
                yield delivery.row;
            }
        }
    }
}

/// The `Last-Event-ID` cursor: absent, or a non-negative publication sequence. Anything else is a
/// malformed header and answers 400.
fn requested_cursor(headers: &HeaderMap) -> Result<Option<i64>, ApiError> {
    headers
        .get("last-event-id")
        .map(|value| {
            value
                .to_str()
                .ok()
                .and_then(|value| value.parse::<i64>().ok())
                .filter(|value| *value >= 0)
                .ok_or_else(|| ApiError::bad_request("invalid Last-Event-ID"))
        })
        .transpose()
}

/// One delivery stream item as its SSE event: `event: ready` and `event: reset` carry their
/// control data, and an audit row is an unnamed event with the list route's row JSON. Every
/// event's id is the publication sequence a reconnect resumes after.
fn stream_event(item: &AuditStreamItem) -> Result<Event, serde_json::Error> {
    Ok(match item {
        AuditStreamItem::Ready(ready) => Event::default()
            .event("ready")
            .id(ready.resume_after.to_string())
            .data(serde_json::to_string(ready)?),
        AuditStreamItem::Delivery(delivery) => Event::default()
            .id(delivery.sequence.to_string())
            .data(serde_json::to_string(&delivery.row)?),
        AuditStreamItem::Reset(reset) => Event::default()
            .event("reset")
            .id(reset.resume_after.to_string())
            .data(serde_json::to_string(reset)?),
    })
}

/// `GET /api/v1/admin/audit-logs/stream` — the live audit feed, replayable by publication
/// sequence.
///
/// The stream opens with `event: ready`, sends each published row as an unnamed event, and sends
/// `event: reset` when the cursor cannot be replayed, continuing from the tail. A malformed
/// `Last-Event-ID` answers 400 before the stream starts.
///
/// @route GET /api/v1/admin/audit-logs/stream
pub async fn stream_audit_logs(
    State(state): State<AppState>,
    admin: AdminUser,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let cursor = requested_cursor(&headers)?;
    let notify = AuditNotify::for_pool(&state.pool);
    let items =
        audit_delivery_stream(state.pool.clone(), notify, AUDIT_POLL_FALLBACK, cursor).await?;
    let body = stream! {
        for await item in items {
            match stream_event(&item) {
                Ok(event) => yield Ok::<Event, Infallible>(event),
                Err(error) => {
                    // The client reconnects from the last id it received; nothing is skipped.
                    tracing::error!(%error, "audit stream event serialization failed; closing the stream");
                    break;
                }
            }
        }
    };
    Ok((
        [(HeaderName::from_static("x-accel-buffering"), "no")],
        Sse::new(
            crate::core::middleware::authorized_event_stream::authorize_event_stream(
                body, state, admin.0, "admin",
            ),
        )
        .keep_alive(KeepAlive::default()),
    )
        .into_response())
}

#[cfg(test)]
#[path = "tests/audit_logs.rs"]
mod tests;
