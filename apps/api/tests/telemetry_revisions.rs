//! Results revisions: under the match row lock a revision is applied, answered as an inert
//! duplicate, refused as stale, or refused as a conflicting rewrite of the stored revision.
//! The report digest covers the body without `revision` and ignores key order and whitespace.
//!
//! Each case checks the HTTP answer, the persisted match and player rows, and the derived
//! statistics. Requires `TEST_DATABASE_URL`.

mod common;
mod contract_support;
mod telemetry_support;

use api::match_telemetry::models::generated::match_telemetry::{
    MatchResultsAnswer, MatchResultsRevision,
};
use axum::http::StatusCode;
use serde_json::{Value, json};
use telemetry_support::match_reports::ReportingServer;
use telemetry_support::report_fixtures::{
    boot_with_state, counters, leaderboard_kills, line, match_state, player_rows, report,
    seed_player, served_statistics,
};
use telemetry_support::{admin_token, call};

const SCHEMA: &str = "match-telemetry.schema.json";

fn refusal(body: &Value) -> &str {
    body["details"]["code"].as_str().unwrap_or_default()
}

fn unique(prefix: &str) -> String {
    common::unique_arma(prefix)
}

#[tokio::test]
async fn telemetry_revisions_duplicate_retry_is_inert() {
    let (app, pool, _state) = boot_with_state().await;
    let admin = admin_token(&app).await;
    let server = ReportingServer::open(&app, &pool, &unique("revisions-duplicate")).await;
    let (discord, username, arma) = seed_player(&pool, "dup").await;
    let src = unique("duplicate");
    let match_id = server.register_match(&app, &src).await;
    let body = report(
        &src,
        "success",
        vec![line(&arma, "life-1", Some(counters(4, 1)))],
    );

    let (status, applied) = server.post_results(&app, 1, &body).await;
    assert_eq!(status, StatusCode::OK, "{applied}");
    assert_eq!(applied["applied"], true);
    let stored = match_state(&pool, match_id).await;
    let rows = player_rows(&pool, match_id).await;
    assert_eq!(leaderboard_kills(&pool, &discord).await, (Some(4), 1));

    let (status, retry) = server.post_results(&app, 1, &body).await;
    assert_eq!(status, StatusCode::OK, "{retry}");
    assert_eq!(retry["applied"], false);
    assert_eq!(retry["revision"], 1);
    for key in [
        "match_id",
        "players",
        "linked",
        "unlinked",
        "unlinked_arma_ids",
    ] {
        assert_eq!(
            retry[key], applied[key],
            "a duplicate computes the same {key}"
        );
    }
    assert_eq!(
        match_state(&pool, match_id).await,
        stored,
        "the match row is untouched"
    );
    assert_eq!(player_rows(&pool, match_id).await, rows);
    assert_eq!(leaderboard_kills(&pool, &discord).await, (Some(4), 1));
    assert_eq!(
        served_statistics(&app, &admin, &discord, &username).await,
        (4, 1, Some(4))
    );
}

#[tokio::test]
async fn telemetry_revisions_same_revision_with_a_new_digest_conflicts() {
    let (app, pool, _state) = boot_with_state().await;
    let server = ReportingServer::open(&app, &pool, &unique("revisions-conflict")).await;
    let (discord, _, arma) = seed_player(&pool, "conflict").await;
    let src = unique("conflict");
    let match_id = server.register_match(&app, &src).await;
    let (status, body) = server
        .post_results(
            &app,
            1,
            &report(
                &src,
                "success",
                vec![line(&arma, "life-1", Some(counters(4, 1)))],
            ),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let stored = match_state(&pool, match_id).await;
    let rows = player_rows(&pool, match_id).await;

    let (status, body) = server
        .post_results(
            &app,
            1,
            &report(
                &src,
                "success",
                vec![line(&arma, "life-1", Some(counters(9, 1)))],
            ),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(refusal(&body), "REVISION_CONFLICT", "{body}");
    assert_eq!(body["details"]["revision"], 1);
    assert_eq!(
        body["details"]["report_sha256"].as_str(),
        stored.1.as_deref(),
        "the refusal names the stored digest"
    );
    contract_support::assert_valid(SCHEMA, Some("TelemetryRefusal"), &body["details"]);

    assert_eq!(match_state(&pool, match_id).await, stored);
    assert_eq!(player_rows(&pool, match_id).await, rows);
    assert_eq!(leaderboard_kills(&pool, &discord).await, (Some(4), 1));
}

#[tokio::test]
async fn telemetry_revisions_older_revision_is_stale_and_changes_nothing() {
    let (app, pool, _state) = boot_with_state().await;
    let server = ReportingServer::open(&app, &pool, &unique("revisions-stale")).await;
    let (discord, _, arma) = seed_player(&pool, "stale").await;
    let src = unique("stale");
    let match_id = server.register_match(&app, &src).await;
    let body = |kills| {
        report(
            &src,
            "success",
            vec![line(&arma, "life-1", Some(counters(kills, 0)))],
        )
    };

    let (status, first) = server.post_results(&app, 1, &body(2)).await;
    assert_eq!(
        (status, first["applied"].clone()),
        (StatusCode::OK, json!(true)),
        "{first}"
    );
    let (status, skipped) = server.post_results(&app, 3, &body(6)).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "a revision may skip numbers: {skipped}"
    );
    assert_eq!(
        (skipped["applied"].clone(), skipped["revision"].clone()),
        (json!(true), json!(3))
    );
    let stored = match_state(&pool, match_id).await;
    assert_eq!(stored.0, 3);

    for older in [2, 1] {
        let (status, refused) = server.post_results(&app, older, &body(1)).await;
        assert_eq!(status, StatusCode::CONFLICT, "revision {older}: {refused}");
        assert_eq!(refusal(&refused), "STALE_REVISION", "{refused}");
        assert_eq!(
            refused["details"]["revision"], 3,
            "the refusal names the stored revision"
        );
    }
    assert_eq!(match_state(&pool, match_id).await, stored);
    let rows = player_rows(&pool, match_id).await;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].2, Some(6));
    assert_eq!(leaderboard_kills(&pool, &discord).await, (Some(6), 1));
}

#[tokio::test]
async fn telemetry_revisions_digest_ignores_key_order_and_whitespace() {
    let (app, pool, _state) = boot_with_state().await;
    let server = ReportingServer::open(&app, &pool, &unique("revisions-digest")).await;
    let (discord, _, arma) = seed_player(&pool, "digest").await;
    let src = unique("digest");
    let match_id = server.register_match(&app, &src).await;

    let compact = format!(
        r#"{{"revision":1,"match":{{"source_match_id":"{src}","outcome":"success","winning_faction":"US"}},"players":[{{"arma_id":"{arma}","role_played":"Medic","source_event_id":"life-1","counters":{{"kills":2,"deaths":1,"team_kills":0,"longest_kill_m":150,"vehicles_destroyed":0,"is_command":false}}}}]}}"#
    );
    let reordered = format!(
        "{{\n  \"players\" : [ {{ \"counters\" : {{ \"is_command\" : false, \"vehicles_destroyed\" : 0,\n    \"longest_kill_m\" : 150, \"team_kills\" : 0, \"deaths\" : 1, \"kills\" : 2 }},\n    \"source_event_id\" : \"life-1\", \"role_played\" : \"Medic\", \"arma_id\" : \"{arma}\" }} ],\n  \"match\" : {{ \"winning_faction\" : \"US\", \"outcome\" : \"success\",\n    \"source_match_id\" : \"{src}\" }},\n  \"revision\" : 1\n}}\n"
    );
    let post = |raw: String| {
        let app = app.clone();
        let secret = server.session.secret.clone();
        async move {
            call(
                &app,
                "POST",
                "/api/v1/ingest/match-results",
                Some(&secret),
                None,
                Some(&raw),
            )
            .await
        }
    };

    let (status, first) = post(compact).await;
    assert_eq!(status, StatusCode::OK, "{first}");
    assert_eq!(first["applied"], true);
    let stored = match_state(&pool, match_id).await;
    let digest = stored.1.clone().expect("report digest stored");
    assert!(digest.len() == 64 && digest.bytes().all(|b| b.is_ascii_hexdigit()));

    let (status, again) = post(reordered).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "same report, other key order and whitespace: {again}"
    );
    assert_eq!(again["applied"], false);
    assert_eq!(match_state(&pool, match_id).await, stored);
    assert_eq!(leaderboard_kills(&pool, &discord).await, (Some(2), 1));
}

#[tokio::test]
async fn telemetry_revisions_concurrent_identical_revisions_apply_once() {
    let (app, pool, _state) = boot_with_state().await;
    let server = ReportingServer::open(&app, &pool, &unique("revisions-race")).await;
    let (discord, _, arma) = seed_player(&pool, "race").await;
    let src = unique("race");
    let match_id = server.register_match(&app, &src).await;
    let body = report(
        &src,
        "success",
        vec![
            line(&arma, "life-1", Some(counters(3, 1))),
            line(&arma, "life-2", Some(counters(2, 1))),
        ],
    );

    let (left, right) = tokio::join!(
        server.post_results(&app, 1, &body),
        server.post_results(&app, 1, &body)
    );
    assert_eq!(
        (left.0, right.0),
        (StatusCode::OK, StatusCode::OK),
        "{} {}",
        left.1,
        right.1
    );
    let mut applied = [left.1["applied"].as_bool(), right.1["applied"].as_bool()];
    applied.sort();
    assert_eq!(
        applied,
        [Some(false), Some(true)],
        "exactly one of the two applies"
    );

    let (revision, digest, ..) = match_state(&pool, match_id).await;
    assert_eq!(revision, 1);
    assert!(digest.is_some());
    let rows = player_rows(&pool, match_id).await;
    assert_eq!(rows.len(), 2);
    assert_eq!(leaderboard_kills(&pool, &discord).await, (Some(5), 1));
}

#[tokio::test]
async fn telemetry_revisions_answer_splits_linked_and_unlinked_lines() {
    let (app, pool, _state) = boot_with_state().await;
    let server = ReportingServer::open(&app, &pool, &unique("revisions-split")).await;
    let (discord, _, linked) = seed_player(&pool, "split").await;
    let orphan = unique("split-orphan");
    let src = unique("split");
    let match_id = server.register_match(&app, &src).await;
    let mut body = report(
        &src,
        "success",
        vec![
            line(&linked, "life-1", Some(counters(1, 1))),
            line(&linked, "life-2", Some(counters(2, 0))),
            line(&orphan, "life-1", Some(counters(0, 1))),
            line(&orphan, "life-2", None),
        ],
    );
    body["revision"] = json!(1);
    contract_support::assert_valid(SCHEMA, Some("MatchResultsRevision"), &body);
    contract_support::assert_decodes::<MatchResultsRevision>("MatchResultsRevision", &body);

    for expected_applied in [true, false] {
        let (status, answer) = server
            .post(&app, "/api/v1/ingest/match-results", &body)
            .await;
        assert_eq!(status, StatusCode::OK, "{answer}");
        contract_support::assert_valid(SCHEMA, Some("MatchResultsAnswer"), &answer);
        contract_support::assert_decodes::<MatchResultsAnswer>("MatchResultsAnswer", &answer);
        assert_eq!(answer["applied"], expected_applied);
        assert_eq!(answer["match_id"], json!(match_id));
        assert_eq!(
            (answer["players"].clone(), answer["linked"].clone()),
            (json!(4), json!(2))
        );
        assert_eq!(answer["unlinked"], 2);
        assert_eq!(answer["unlinked_arma_ids"], json!([orphan]));
    }

    let rows = player_rows(&pool, match_id).await;
    assert_eq!(rows.len(), 4);
    for (arma, _, _, _, owner) in &rows {
        let expected = (arma == &linked).then(|| discord.clone());
        assert_eq!(owner, &expected, "{arma} ownership");
    }
}

#[tokio::test]
async fn telemetry_revisions_malformed_bodies_are_refused_before_anything_is_written() {
    let (app, pool, _state) = boot_with_state().await;
    let server = ReportingServer::open(&app, &pool, &unique("revisions-malformed")).await;
    let (_, _, arma) = seed_player(&pool, "malformed").await;
    let src = unique("malformed");
    let match_id = server.register_match(&app, &src).await;
    let valid = report(
        &src,
        "success",
        vec![line(&arma, "life-1", Some(counters(1, 0)))],
    );

    let (status, body) = server.post_results(&app, 0, &valid).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "revision 0: {body}");
    assert_eq!(refusal(&body), "INVALID_MATCH_RESULTS", "{body}");
    assert_eq!(body["details"]["field"], "revision");
    assert!(
        body["details"].get("index").is_none(),
        "a match-level field has no index"
    );

    let (status, body) = server
        .post(&app, "/api/v1/ingest/match-results", &valid)
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "missing revision: {body}");

    let mut flat = valid.clone();
    flat["players"][0]["kills"] = json!(1);
    let (status, body) = server.post_results(&app, 1, &flat).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "flat counter key: {body}");
    assert_eq!(refusal(&body), "INVALID_MATCH_RESULTS", "{body}");
    assert_eq!(body["details"]["index"], 0);

    let mut unknown = valid.clone();
    unknown["mission_name"] = json!("unknown");
    let (status, body) = server.post_results(&app, 1, &unknown).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "unknown key: {body}");

    assert_eq!(
        match_state(&pool, match_id).await,
        (0, None, "pending".into(), None, 0)
    );
    assert!(player_rows(&pool, match_id).await.is_empty());
}
