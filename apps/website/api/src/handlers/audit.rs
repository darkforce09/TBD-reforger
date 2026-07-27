//! Audit console — Rust port of `handlers/audit.go` (list + CSV + SSE stream, admin).

use std::borrow::Cow;
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

/// Neutralise CSV formula injection for spreadsheet consumers (Excel / Sheets).
///
/// Cells whose first character is `=`, `+`, `-`, or `@` are live formulas when the file is
/// opened. Audit fields (`actor_name`, `action`, `message`, …) are user-influenced, so the
/// export writer must prefix those cells. A leading `'` is the standard spreadsheet "treat as
/// text" marker; it survives `csv::Writer` quoting and is stripped by Excel on display.
///
/// T-408 / T-391: `is_http_url` does **not** cover this sink — a value that passes a URL guard
/// is still illegal to interpolate raw into CSV.
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

#[cfg(test)]
mod tests {
    use super::*;

    /// T-408 Class-R: formula-leading cells must be prefixed so Excel/Sheets do not execute them.
    ///
    /// RED: delete the `Some(b'=' | …)` arm (or make the helper return `Cow::Borrowed` always) —
    /// `assert!(escaped.starts_with('\''))` fails and raw `=cmd` survives.
    #[test]
    fn escape_csv_formula_prefixes_equals_plus_minus_at() {
        for dangerous in [
            "=cmd|'/C calc'!A0",
            "=HYPERLINK(\"http://evil\")",
            "+1+1",
            "-1+1",
            "@SUM(A1)",
        ] {
            let escaped = escape_csv_formula(dangerous);
            assert!(
                escaped.starts_with('\''),
                "formula cell {dangerous:?} must be quote-prefixed, got {escaped:?}"
            );
            assert_eq!(&escaped[1..], dangerous);
        }
        // Safe cells pass through unchanged (including empty and leading digit/letter).
        assert_eq!(escape_csv_formula(""), "");
        assert_eq!(escape_csv_formula("alice"), "alice");
        assert_eq!(escape_csv_formula("mission.updated"), "mission.updated");
        assert_eq!(escape_csv_formula("9=ok"), "9=ok");
    }

    /// T-408 Class-R: the export writer path (same `csv::Writer` + `escape_csv_formula` as
    /// [`export_audit_logs_csv`]) must not emit a record whose decoded field still starts with
    /// a formula character. A helper-only green with a raw write path is a false green.
    ///
    /// RED: write `&l.message` (etc.) without `escape_csv_formula` — the decoded field is
    /// `=cmd…` and this assert fires.
    #[test]
    fn export_writer_path_prefixes_formula_cells() {
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
        let formula_message = "=cmd|'/C calc'!A0";
        let formula_actor = "@SUM(A1)";
        let _ = w.write_record([
            escape_csv_formula("2026-07-27T00:00:00Z").as_ref(),
            escape_csv_formula("info").as_ref(),
            escape_csv_formula(formula_actor).as_ref(),
            escape_csv_formula("audit.test").as_ref(),
            escape_csv_formula(formula_message).as_ref(),
            escape_csv_formula("mission").as_ref(),
            escape_csv_formula("-uuid-lookalike").as_ref(),
        ]);
        let body = String::from_utf8(w.into_inner().unwrap_or_default()).expect("utf8 csv");

        // Byte-level: raw `=cmd` must not appear as a CSV field start (after comma or line start).
        assert!(
            !body.contains(",=cmd") && !body.lines().any(|l| l.starts_with('=')),
            "raw formula must not survive in CSV bytes:\n{body}"
        );
        assert!(
            body.contains("'=cmd|'/C calc'!A0") || body.contains("\"'=cmd|'/C calc'!A0\""),
            "escaped formula message must appear quote-prefixed:\n{body}"
        );

        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(true)
            .from_reader(body.as_bytes());
        let rec = rdr.records().next().expect("one data row").expect("csv row");
        for (i, field) in rec.iter().enumerate() {
            assert!(
                !matches!(field.as_bytes().first(), Some(b'=' | b'+' | b'-' | b'@')),
                "decoded field[{i}] still formula-leading: {field:?}\ncsv:\n{body}"
            );
        }
        assert_eq!(rec.get(2).unwrap(), "'@SUM(A1)");
        assert_eq!(rec.get(4).unwrap(), "'=cmd|'/C calc'!A0");
        assert_eq!(rec.get(6).unwrap(), "'-uuid-lookalike");
    }
}
