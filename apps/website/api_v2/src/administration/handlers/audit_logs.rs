//! The audit console: the filtered list, the CSV export, and the live SSE feed.
//!
//! The stream is pushed by Postgres NOTIFY ([`crate::administration::services::audit_notifier`]);
//! its 2 s poll survives only as the fallback while that listener is down.

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
use tokio::sync::broadcast;

use crate::administration::models::audit_log::AuditLog;
use crate::administration::services::audit_notifier::{AuditNotify, AuditSignal};
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

/// Fallback cadence for the live feed. The ticker reaches the database only while the
/// `audit_log` listener ([`crate::administration::services::audit_notifier`]) is down.
pub const AUDIT_POLL_FALLBACK: Duration = Duration::from_secs(2);

/// Rows per catch-up SELECT; a longer burst is drained page by page before the stream waits again.
const CATCH_UP_PAGE: i64 = 100;

const NEW_ROWS_SQL: &str = "SELECT id, severity, actor_id, COALESCE(actor_name, '') AS actor_name, action, message, COALESCE(target_type, '') AS target_type, COALESCE(target_id, '') AS target_id, metadata, COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at FROM audit_logs WHERE id > $1 ORDER BY id ASC LIMIT $2";

/// Every `audit_logs` row committed after this call, in id order.
///
/// Pushed: the 0025 trigger raises `pg_notify('audit_log', id)` per insert, `notify` fans it out,
/// and each [`AuditSignal::Row`] / [`AuditSignal::Resync`] triggers one `id > last_id` fetch — a
/// burst is coalesced into one query. Polled: the `poll_every` ticker runs the same fetch, but only
/// while [`AuditNotify::is_listening`] is false, so a client on a healthy listener costs the
/// database nothing between rows. The subscription and the tail snapshot are taken before this
/// returns, so a row committed after the call is never missed.
pub async fn audit_row_stream(
    pool: PgPool,
    notify: AuditNotify,
    poll_every: Duration,
) -> impl Stream<Item = AuditLog> + Send {
    // Subscribe BEFORE reading the tail: a row that commits in between is announced and fetched
    // rather than lost behind the snapshot.
    let mut rx = notify.subscribe();
    let mut last_id: i64 = sqlx::query_scalar("SELECT COALESCE(max(id), 0) FROM audit_logs")
        .fetch_one(&pool)
        .await
        .unwrap_or(0);
    stream! {
        let mut ticker = tokio::time::interval(poll_every);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        ticker.tick().await; // consume the immediate first tick
        let mut closed = false;
        loop {
            let fetch = tokio::select! {
                sig = rx.recv(), if !closed => match sig {
                    Ok(AuditSignal::Row(_) | AuditSignal::Resync) => true,
                    Ok(AuditSignal::Down) => false,
                    Err(broadcast::error::RecvError::Lagged(_)) => true,
                    Err(broadcast::error::RecvError::Closed) => {
                        closed = true;
                        false
                    }
                },
                _ = ticker.tick() => !notify.is_listening(),
            };
            if !fetch {
                continue;
            }
            // A burst announces one row at a time: drain what is already queued, query once.
            while matches!(
                rx.try_recv(),
                Ok(_) | Err(broadcast::error::TryRecvError::Lagged(_))
            ) {}
            loop {
                let rows: Vec<AuditLog> = match sqlx::query_as(NEW_ROWS_SQL)
                    .bind(last_id)
                    .bind(CATCH_UP_PAGE)
                    .fetch_all(&pool)
                    .await
                {
                    Ok(rows) => rows,
                    Err(e) => {
                        tracing::warn!(error = %e, "audit stream fetch failed; will retry");
                        Vec::new()
                    }
                };
                let page_full = rows.len() as i64 == CATCH_UP_PAGE;
                for r in rows {
                    last_id = r.id;
                    yield r;
                }
                if !page_full {
                    break;
                }
            }
        }
    }
}

/// `GET /api/v1/admin/audit-logs/stream` — terminal-style live feed. Pushed by Postgres NOTIFY;
/// polls every [`AUDIT_POLL_FALLBACK`] only while the listener is down.
///
/// @route GET /api/v1/admin/audit-logs/stream
pub async fn stream_audit_logs(State(state): State<AppState>, _a: AdminUser) -> Response {
    let notify = AuditNotify::for_pool(&state.pool);
    let rows = audit_row_stream(state.pool.clone(), notify, AUDIT_POLL_FALLBACK).await;
    let body = stream! {
        for await row in rows {
            if let Ok(js) = serde_json::to_string(&row) {
                yield Ok::<Event, Infallible>(Event::default().data(js));
            }
        }
    };
    (
        [(HeaderName::from_static("x-accel-buffering"), "no")],
        // The feed is silent between rows now; the comment ping keeps idle proxies from cutting it.
        Sse::new(body).keep_alive(KeepAlive::default()),
    )
        .into_response()
}

#[cfg(test)]
#[path = "tests/audit_logs.rs"]
mod tests;
