//! The audit console: the filtered list, the CSV export, and the live SSE feed.
//!
//! The stream is pushed by Postgres NOTIFY ([`crate::administration::services::audit_notifier`]);
//! a periodic retry recovers failed reads independently of notification health.

use std::borrow::Cow;
use std::convert::Infallible;
use std::time::Duration;

use async_stream::stream;
use axum::extract::{Query, State};
use axum::http::{HeaderName, header};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Json, Response};
use futures::Stream;
use serde::Deserialize;
use serde_json::{Value, json};
use sqlx::{PgPool, QueryBuilder};

use crate::administration::models::audit_log::AuditLog;
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

/// Compatibility stream of audit rows; HTTP clients use publication IDs for replay.
pub async fn audit_row_stream(
    pool: PgPool,
    notify: AuditNotify,
    poll_every: Duration,
) -> impl Stream<Item = AuditLog> + Send {
    let rows = crate::administration::services::audit_delivery::audit_delivery_stream(
        pool, notify, poll_every, None,
    )
    .await
    .expect("initialize durable audit stream");
    stream! { for await delivery in rows { yield delivery.row; } }
}

/// GET /api/v1/admin/audit-logs/stream — durable publication IDs support reconnect replay.
/// @route GET /api/v1/admin/audit-logs/stream
pub async fn stream_audit_logs(
    State(state): State<AppState>,
    _admin: AdminUser,
    headers: axum::http::HeaderMap,
) -> Result<Response, ApiError> {
    let cursor = headers
        .get("last-event-id")
        .map(|value| {
            value
                .to_str()
                .ok()
                .and_then(|value| value.parse::<i64>().ok())
                .filter(|value| *value >= 0)
                .ok_or_else(|| ApiError::bad_request("invalid Last-Event-ID"))
        })
        .transpose()?;
    let notify = AuditNotify::for_pool(&state.pool);
    let deliveries = crate::administration::services::audit_delivery::audit_delivery_stream(
        state.pool.clone(),
        notify,
        AUDIT_POLL_FALLBACK,
        cursor,
    )
    .await
    .map_err(|error| match error {
        sqlx::Error::Protocol(_) => {
            ApiError::conflict("audit history reset required; reload the audit list")
        }
        error => ApiError::from(error),
    })?;
    let body = stream! {
        for await delivery in deliveries {
            match serde_json::to_string(&delivery.row) {
                Ok(json) => yield Ok::<Event, Infallible>(Event::default().id(delivery.sequence.to_string()).data(json)),
                Err(error) => {
                    tracing::error!(%error, "audit delivery serialization failed");
                    yield Ok(Event::default().event("reset").data("reload audit history"));
                    break;
                }
            }
        }
    };
    Ok((
        [(HeaderName::from_static("x-accel-buffering"), "no")],
        Sse::new(
            crate::core::middleware::authorized_event_stream::authorize_event_stream(
                body, state, _admin.0, "admin",
            ),
        )
        .keep_alive(KeepAlive::default()),
    )
        .into_response())
}

#[cfg(test)]
#[path = "tests/audit_logs.rs"]
mod tests;
