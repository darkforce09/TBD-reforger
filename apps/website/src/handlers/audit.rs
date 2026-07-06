//! Audit console — Rust port of `handlers/audit.go` (list + CSV + SSE stream, admin).

use std::convert::Infallible;
use std::time::Duration;

use async_stream::stream;
use axum::extract::{Query, State};
use axum::http::{HeaderName, header};
use axum::response::sse::{Event, Sse};
use axum::response::{IntoResponse, Json, Response};
use serde::Deserialize;
use serde_json::{Value, json};
use sqlx::QueryBuilder;

use crate::error::ApiError;
use crate::handlers::PageParams;
use crate::middleware::AdminUser;
use crate::models::AuditLog;
use crate::state::AppState;

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
        let _ = w.write_record([
            &l.created_at
                .to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            l.severity.as_str(),
            &l.actor_name,
            &l.action,
            &l.message,
            &l.target_type,
            &l.target_id,
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

/// `GET /api/v1/admin/audit-logs/stream` — terminal-style live feed (SSE poll @ 2s).
///
/// @route GET /api/v1/admin/audit-logs/stream
pub async fn stream_audit_logs(State(state): State<AppState>, _a: AdminUser) -> Response {
    let pool = state.pool.clone();
    let body = stream! {
        // Start from the current tail so the client only sees new events.
        let mut last_id: i64 = sqlx::query_scalar("SELECT COALESCE(max(id), 0) FROM audit_logs")
            .fetch_one(&pool).await.unwrap_or(0);
        let mut ticker = tokio::time::interval(Duration::from_secs(2));
        ticker.tick().await; // consume the immediate first tick
        loop {
            ticker.tick().await;
            let rows: Vec<AuditLog> = sqlx::query_as(
                "SELECT id, severity, actor_id, COALESCE(actor_name, '') AS actor_name, action, message, COALESCE(target_type, '') AS target_type, COALESCE(target_id, '') AS target_id, metadata, COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at FROM audit_logs WHERE id > $1 ORDER BY id ASC LIMIT 100",
            ).bind(last_id).fetch_all(&pool).await.unwrap_or_default();
            for r in &rows {
                if let Ok(js) = serde_json::to_string(r) {
                    yield Ok::<Event, Infallible>(Event::default().data(js));
                }
                last_id = r.id;
            }
        }
    };
    (
        [(HeaderName::from_static("x-accel-buffering"), "no")],
        Sse::new(body),
    )
        .into_response()
}
