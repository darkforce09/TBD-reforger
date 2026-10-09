//! Match identity: a match is registered by one server for one `source_match_id` before any report
//! about it, the registration is idempotent, and a runtime session of another server is refused.
//!
//! Each case checks the HTTP answer and the persisted `matches` rows.

use crate::{common, telemetry_support};

use axum::http::StatusCode;
use serde_json::json;
use sqlx::PgPool;
use telemetry_support::match_reports::ReportingServer;
use telemetry_support::report_fixtures::{boot_with_state, match_state};
use uuid::Uuid;

fn source(prefix: &str) -> String {
    common::unique_arma(prefix)
}

/// Every registered match row of `source`: `(id, server_id, registered_runtime_session_id,
/// registration_sha256)`.
async fn registered_rows(
    pool: &PgPool,
    source: &str,
) -> Vec<(Uuid, Option<Uuid>, Option<Uuid>, Option<String>)> {
    sqlx::query_as(
        "SELECT id, server_id, registered_runtime_session_id, registration_sha256 FROM matches
         WHERE source_match_id = $1 ORDER BY id",
    )
    .bind(source)
    .fetch_all(pool)
    .await
    .expect("the read of matches runs")
}

#[tokio::test]
async fn match_identity_registration_is_idempotent_per_server_and_source() {
    let (app, pool, _state) = boot_with_state().await;
    let server = ReportingServer::open(&app, &pool, &source("identity-idem")).await;
    let src = source("idem");

    let (status, first) = server.register_match_with(&app, &src, json!({})).await;
    assert_eq!(status, StatusCode::CREATED, "{first}");
    assert_eq!(first["registered"], true);
    let (status, again) = server.register_match_with(&app, &src, json!({})).await;
    assert_eq!(status, StatusCode::OK, "{again}");
    assert_eq!(again["registered"], false);
    assert_eq!(first["match_id"], again["match_id"]);

    let rows = registered_rows(&pool, &src).await;
    assert_eq!(rows.len(), 1, "one match per (server, source)");
    let (id, server_id, session, digest) = &rows[0];
    assert_eq!(id.to_string(), first["match_id"].as_str().unwrap());
    assert_eq!(*server_id, Some(server.server_id));
    assert_eq!(*session, Some(server.session.id));
    let digest = digest.as_deref().expect("registration digest stored");
    assert!(digest.len() == 64 && digest.bytes().all(|b| b.is_ascii_hexdigit()));
    let (revision, report_sha, outcome, finalized, events) = match_state(&pool, *id).await;
    assert_eq!(
        (revision, report_sha, outcome.as_str(), finalized, events),
        (0, None, "pending", None, 0)
    );
}

#[tokio::test]
async fn match_identity_runtime_session_of_another_server_is_forbidden() {
    let (app, pool, _state) = boot_with_state().await;
    let alpha = ReportingServer::open(&app, &pool, &source("identity-owner")).await;
    let bravo = ReportingServer::open(&app, &pool, &source("identity-borrower")).await;
    let src = source("borrowed-session");

    let (status, body) = alpha
        .post(
            &app,
            "/api/v1/ingest/matches",
            &json!({
                "source_match_id": src, "runtime_session_id": bravo.session.id,
                "started_at": "2026-01-01T00:00:00Z",
            }),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
    assert!(registered_rows(&pool, &src).await.is_empty());
}
