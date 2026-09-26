//! Match identity: a match is registered by one server for one `source_match_id` before any
//! report about it, the registration is idempotent and immutable, and every ingest route
//! authenticates the `mod_runtime` credential of the reporting server.
//!
//! Each case checks the HTTP answer and the persisted `matches` rows. Requires
//! `TEST_DATABASE_URL`.

mod common;
mod telemetry_support;

use axum::http::StatusCode;
use serde_json::{Value, json};
use sqlx::PgPool;
use telemetry_support::match_reports::ReportingServer;
use telemetry_support::report_fixtures::{boot_with_state, counters, line, match_state, report};
use telemetry_support::{call, heartbeat};
use uuid::Uuid;

fn refusal(body: &Value) -> &str {
    body["details"]["code"].as_str().unwrap_or_default()
}

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
    .unwrap()
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
async fn match_identity_same_source_on_two_servers_is_two_matches() {
    let (app, pool, _state) = boot_with_state().await;
    let alpha = ReportingServer::open(&app, &pool, &source("identity-alpha")).await;
    let bravo = ReportingServer::open(&app, &pool, &source("identity-bravo")).await;
    let src = source("shared");

    let (status_a, a) = alpha.register_match_with(&app, &src, json!({})).await;
    let (status_b, b) = bravo.register_match_with(&app, &src, json!({})).await;
    assert_eq!(
        (status_a, status_b),
        (StatusCode::CREATED, StatusCode::CREATED),
        "{a} {b}"
    );
    assert_ne!(a["match_id"], b["match_id"]);

    let rows = registered_rows(&pool, &src).await;
    assert_eq!(rows.len(), 2);
    let mut servers: Vec<Uuid> = rows.iter().filter_map(|row| row.1).collect();
    servers.sort();
    let mut expected = vec![alpha.server_id, bravo.server_id];
    expected.sort();
    assert_eq!(servers, expected);
}

#[tokio::test]
async fn match_identity_changed_registration_conflicts() {
    let (app, pool, _state) = boot_with_state().await;
    let server = ReportingServer::open(&app, &pool, &source("identity-conflict")).await;
    let src = source("conflict");
    let (status, first) = server.register_match_with(&app, &src, json!({})).await;
    assert_eq!(status, StatusCode::CREATED, "{first}");
    let before = registered_rows(&pool, &src).await;

    let (status, body) = server
        .register_match_with(&app, &src, json!({ "terrain": "arland" }))
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(refusal(&body), "REGISTRATION_CONFLICT", "{body}");
    assert_eq!(body["details"]["match_id"], first["match_id"]);

    assert_eq!(
        registered_rows(&pool, &src).await,
        before,
        "the registration never changes"
    );
    let terrain: Option<String> =
        sqlx::query_scalar("SELECT terrain::text FROM matches WHERE source_match_id = $1")
            .bind(&src)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(terrain, None);
}

#[tokio::test]
async fn match_identity_results_before_registration_are_refused() {
    let (app, pool, _state) = boot_with_state().await;
    let server = ReportingServer::open(&app, &pool, &source("identity-early-results")).await;
    let src = source("early-results");
    let arma = common::unique_arma("early-results-arma");

    let (status, body) = server
        .post_results(
            &app,
            1,
            &report(
                &src,
                "success",
                vec![line(&arma, "life-1", Some(counters(3, 1)))],
            ),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(refusal(&body), "MATCH_NOT_REGISTERED", "{body}");

    assert!(
        registered_rows(&pool, &src).await.is_empty(),
        "no match row is created"
    );
    let lines: i64 =
        sqlx::query_scalar("SELECT count(*) FROM match_player_stats WHERE arma_id = $1")
            .bind(&arma)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(lines, 0);
}

#[tokio::test]
async fn match_identity_events_before_registration_are_refused() {
    let (app, pool, _state) = boot_with_state().await;
    let server = ReportingServer::open(&app, &pool, &source("identity-early-events")).await;
    let src = source("early-events");
    let arma = common::unique_arma("early-events-arma");
    let event = json!({
        "event_id": "early-1", "sequence": 1, "kind": "medical.revived", "mission_time_ms": 10,
        "occurred_at": "2026-01-01T00:00:10Z", "payload": { "subject_arma_id": arma },
    });

    let (status, body) = server.post_events(&app, &src, json!([event])).await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(refusal(&body), "MATCH_NOT_REGISTERED", "{body}");

    assert!(registered_rows(&pool, &src).await.is_empty());
    let totals: i64 =
        sqlx::query_scalar("SELECT count(*) FROM match_event_totals WHERE arma_id = $1")
            .bind(&arma)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(totals, 0);
}

#[tokio::test]
async fn match_identity_host_agent_credential_is_forbidden() {
    let (app, pool, _state) = boot_with_state().await;
    let server = ReportingServer::open(&app, &pool, &source("identity-host-agent")).await;
    let credential = Uuid::new_v4();
    let secret = format!(
        "tbdm_{}_{}",
        credential.simple(),
        website_api::core::authentication_primitives::random_token(32)
    );
    sqlx::query(
        "INSERT INTO server_machine_credentials (id, server_id, executor_kind, secret_sha256, label, created_by)
         VALUES ($1, $2, 'host_agent', $3, 'Host agent', $4)",
    )
    .bind(credential)
    .bind(server.server_id)
    .bind(website_api::core::authentication_primitives::hash_token(&secret))
    .bind(common::DEV_LOGIN_USER)
    .execute(&pool)
    .await
    .unwrap();
    let src = source("host-agent");
    let registration = json!({
        "source_match_id": src, "runtime_session_id": server.session.id,
        "started_at": "2026-01-01T00:00:00Z",
    });
    let mut results = report(&src, "success", vec![]);
    results["revision"] = json!(1);
    let events = json!({ "source_match_id": src, "events": [] });

    for (uri, body) in [
        ("/api/v1/ingest/matches", registration),
        ("/api/v1/ingest/match-results", results),
        ("/api/v1/ingest/match-events", events),
    ] {
        let (status, answer) = call(
            &app,
            "POST",
            uri,
            Some(&secret),
            None,
            Some(&body.to_string()),
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{uri}: {answer}");
    }
    assert!(registered_rows(&pool, &src).await.is_empty());
}

#[tokio::test]
async fn match_identity_ingest_without_a_credential_is_unauthorized() {
    let (app, pool, _state) = boot_with_state().await;
    let server = ReportingServer::open(&app, &pool, &source("identity-anonymous")).await;
    let src = source("anonymous");
    let registration = json!({
        "source_match_id": src, "runtime_session_id": server.session.id,
        "started_at": "2026-01-01T00:00:00Z",
    });
    let (status, body) = call(
        &app,
        "POST",
        "/api/v1/ingest/matches",
        None,
        None,
        Some(&registration.to_string()),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "{body}");
    assert!(registered_rows(&pool, &src).await.is_empty());
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

#[tokio::test]
async fn match_identity_registration_after_the_session_ended_is_accepted() {
    let (app, pool, _state) = boot_with_state().await;
    let server = ReportingServer::open(&app, &pool, &source("identity-ended")).await;
    let (status, ended) = call(
        &app,
        "POST",
        &format!("/api/v1/game-runtime/sessions/{}/end", server.session.id),
        Some(&server.session.secret),
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{ended}");
    let ended_at: Option<chrono::DateTime<chrono::Utc>> =
        sqlx::query_scalar("SELECT ended_at FROM server_runtime_sessions WHERE id = $1")
            .bind(server.session.id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(ended_at.is_some(), "the session has ended");

    let src = source("after-end");
    let (status, body) = server.register_match_with(&app, &src, json!({})).await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    let rows = registered_rows(&pool, &src).await;
    assert_eq!(rows.len(), 1);
    assert_eq!(
        rows[0].2,
        Some(server.session.id),
        "the ended session is the provenance"
    );
}

#[tokio::test]
async fn match_identity_heartbeat_naming_another_servers_match_is_refused() {
    let (app, pool, _state) = boot_with_state().await;
    let alpha = ReportingServer::open(&app, &pool, &source("identity-beat-alpha")).await;
    let bravo = ReportingServer::open(&app, &pool, &source("identity-beat-bravo")).await;
    let own = alpha.register_match(&app, &source("own-match")).await;
    let foreign = bravo.register_match(&app, &source("foreign-match")).await;

    let (status, body) =
        heartbeat(&app, &alpha.session, 1, json!({ "current_match_id": own })).await;
    assert!(status.is_success(), "own match: {status} {body}");
    let stored: Option<Uuid> =
        sqlx::query_scalar("SELECT current_match_id FROM server_statuses WHERE server_id = $1")
            .bind(alpha.server_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(stored, Some(own));

    let (status, body) = heartbeat(
        &app,
        &alpha.session,
        2,
        json!({ "current_match_id": foreign }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    let stored: Option<Uuid> =
        sqlx::query_scalar("SELECT current_match_id FROM server_statuses WHERE server_id = $1")
            .bind(alpha.server_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(stored, Some(own), "the refused heartbeat changes nothing");
}

#[tokio::test]
async fn match_identity_body_naming_server_id_is_refused() {
    let (app, pool, _state) = boot_with_state().await;
    let server = ReportingServer::open(&app, &pool, &source("identity-claims")).await;
    let other = ReportingServer::open(&app, &pool, &source("identity-claimed")).await;
    let src = source("claims-server");

    let (status, body) = server
        .register_match_with(&app, &src, json!({ "server_id": other.server_id }))
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "registration: {body}");
    assert!(registered_rows(&pool, &src).await.is_empty());

    let match_id = server.register_match(&app, &src).await;
    let mut results = report(&src, "success", vec![]);
    results["server_id"] = json!(other.server_id);
    let (status, body) = server.post_results(&app, 1, &results).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "results: {body}");
    let (status, body) = server
        .post(
            &app,
            "/api/v1/ingest/match-events",
            &json!({ "source_match_id": src, "server_id": other.server_id, "events": [{
                "event_id": "claim-1", "sequence": 1, "kind": "medical.revived",
                "mission_time_ms": 1, "occurred_at": "2026-01-01T00:00:01Z",
                "payload": { "subject_arma_id": "claims-arma" },
            }] }),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "events: {body}");
    let (status, body) = heartbeat(
        &app,
        &server.session,
        1,
        json!({ "server_id": other.server_id }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "heartbeat: {body}");

    assert_eq!(
        match_state(&pool, match_id).await,
        (0, None, "pending".into(), None, 0)
    );
    let rows = registered_rows(&pool, &src).await;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].1, Some(server.server_id));
}

#[tokio::test]
async fn match_identity_source_match_id_is_one_to_128_bytes_after_trimming() {
    let (app, pool, _state) = boot_with_state().await;
    let server = ReportingServer::open(&app, &pool, &source("identity-length")).await;
    let too_long = format!("{}{}", source("long"), "x".repeat(128));

    let (status, body) = server.register_match_with(&app, &too_long, json!({})).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert!(registered_rows(&pool, &too_long).await.is_empty());
    let (status, body) = server.register_match_with(&app, "   ", json!({})).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");

    let exact = format!("{:x<128}", source("edge"));
    let (status, body) = server.register_match_with(&app, &exact, json!({})).await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    assert_eq!(registered_rows(&pool, &exact).await.len(), 1);
}

#[tokio::test]
async fn match_identity_matches_recorded_before_registration_accept_no_reports() {
    let (app, pool, _state) = boot_with_state().await;
    let server = ReportingServer::open(&app, &pool, &source("identity-legacy")).await;
    let src = source("legacy");
    let legacy: Uuid = sqlx::query_scalar(
        "INSERT INTO matches (source_match_id, started_at, outcome, finalized_at)
         VALUES ($1, now(), 'success', now()) RETURNING id",
    )
    .bind(&src)
    .fetch_one(&pool)
    .await
    .unwrap();

    let (status, body) = server
        .post_results(&app, 1, &report(&src, "failure", vec![]))
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(refusal(&body), "MATCH_NOT_REGISTERED", "{body}");
    let (revision, _, outcome, _, _) = match_state(&pool, legacy).await;
    assert_eq!(
        (revision, outcome.as_str()),
        (0, "success"),
        "the legacy match is unchanged"
    );
    let server_id: Option<Uuid> = sqlx::query_scalar("SELECT server_id FROM matches WHERE id = $1")
        .bind(legacy)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(server_id, None);
}

#[tokio::test]
async fn match_identity_registration_columns_never_change() {
    let (app, pool, _state) = boot_with_state().await;
    let alpha = ReportingServer::open(&app, &pool, &source("identity-guard-alpha")).await;
    let bravo = ReportingServer::open(&app, &pool, &source("identity-guard-bravo")).await;
    let match_id = alpha.register_match(&app, &source("guarded")).await;

    for statement in [
        "UPDATE matches SET server_id = $2 WHERE id = $1",
        "UPDATE matches SET registered_runtime_session_id = (SELECT id FROM server_runtime_sessions WHERE server_id = $2) WHERE id = $1",
        "UPDATE matches SET registration_sha256 = repeat('0', 64) WHERE id = $1 AND $2::uuid IS NOT NULL",
    ] {
        let refused = sqlx::query(statement)
            .bind(match_id)
            .bind(bravo.server_id)
            .execute(&pool)
            .await;
        assert!(
            refused.is_err(),
            "`{statement}` must be refused by match_revision_guard"
        );
    }
    let server_id: Option<Uuid> = sqlx::query_scalar("SELECT server_id FROM matches WHERE id = $1")
        .bind(match_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(server_id, Some(alpha.server_id));
}
