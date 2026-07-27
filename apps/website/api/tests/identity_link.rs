//! Arma identity-link flow — port of the link half of `identity_integration_test.go`.
//! Skips unless `TEST_DATABASE_URL` points at a migrated DB.
//!
//! # Fixture ownership (T-400)
//!
//! Pre-fix this suite authenticated as the shared `dev-login` snowflake
//! ([`common::DEV_LOGIN_USER`]), nulled that row's `arma_id` (errors swallowed by `let _`),
//! then relinked it. `GET /me` computes `arma_linked` from the **database** row
//! (`handlers/me.rs`), and `auth_refresh.rs` asserts `arma_linked == true` on that same
//! shared id — so a concurrent `cargo test -p website-api` interleaved this setup ahead of
//! auth_refresh and failed. Actors here now live in the T-400 private range and are minted
//! via [`common::access_token`] (writes nothing on the shared row).

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use serde_json::Value;
use sqlx::PgPool;
use tower::ServiceExt;
use website_api::config::Config;
use website_api::state::AppState;
use website_api::{app, db};

mod common;

/// Primary actor under test — namespaced so no sibling binary can rewrite its `arma_id`.
const ACTOR: &str = "000000000000400001";
/// Clash partner for the 409 path (same private range).
const USER2: &str = "000000000000400002";
/// T-351 padded-trim actor (same T-400 private range).
const PAD_ACTOR: &str = "000000000000400003";
const ACTOR_ARMA: &str = "identity-link-arma-400001";
/// Canonical (trimmed) Steam id for the T-351 pin.
const PAD_ARMA: &str = "identity-link-arma-padded-400003";
/// Wire form that must NOT land in `users.arma_id` — spaces around the id.
const PAD_ARMA_PADDED: &str = "  identity-link-arma-padded-400003  ";
const SVC: &str = "test-service-token";

async fn setup() -> Option<(Router, AppState, PgPool)> {
    let url = std::env::var("TEST_DATABASE_URL").ok()?;
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");

    // Scoped cleanup only — never touch `common::DEV_LOGIN_USER`. Fail loud on SQL errors
    // (the pre-fix `let _` swallowed unique-index / migrate races and left a poisoned row).
    for q in [
        "DELETE FROM identity_link_codes WHERE discord_id = ANY($1)",
        "DELETE FROM refresh_tokens WHERE discord_id = ANY($1)",
    ] {
        sqlx::query(q)
            .bind(vec![
                ACTOR.to_string(),
                USER2.to_string(),
                PAD_ACTOR.to_string(),
            ])
            .execute(&pool)
            .await
            .unwrap_or_else(|e| panic!("identity_link cleanup `{q}`: {e}"));
    }
    // Release our private arma_id if another row somehow holds it (UNIQUE, non-partial).
    sqlx::query("UPDATE users SET arma_id = NULL WHERE arma_id = ANY($1)")
        .bind(vec![ACTOR_ARMA.to_string(), PAD_ARMA.to_string()])
        .execute(&pool)
        .await
        .unwrap_or_else(|e| panic!("identity_link release arma: {e}"));
    sqlx::query("DELETE FROM match_player_stats WHERE arma_id = $1")
        .bind(PAD_ARMA)
        .execute(&pool)
        .await
        .unwrap_or_else(|e| panic!("identity_link release pad stats: {e}"));
    sqlx::query("DELETE FROM matches WHERE source_match_id = ANY($1)")
        .bind(vec![
            "m-t351-pad-trim".to_string(),
            "m-t351-pad-trim-2".to_string(),
        ])
        .execute(&pool)
        .await
        .unwrap_or_else(|e| panic!("identity_link release pad match: {e}"));

    common::seed_user(
        &pool,
        ACTOR,
        "Identity Link Actor",
        "identity-link-seed-400001",
        "admin",
    )
    .await;
    // Start the link flow unlinked: seed_user wrote a placeholder arma_id (UNIQUE-safe);
    // clear it for THIS actor only.
    sqlx::query(
        "UPDATE users SET arma_id = NULL, arma_character = '', updated_at = now() \
         WHERE discord_id = $1",
    )
    .bind(ACTOR)
    .execute(&pool)
    .await
    .unwrap_or_else(|e| panic!("identity_link unlink actor: {e}"));

    let state = AppState::new(pool.clone(), Config::for_tests(url, "identity-secret"));
    let app = app::router(state.clone());
    Some((app, state, pool))
}

async fn call(
    app: &Router,
    method: &str,
    uri: &str,
    headers: &[(&str, &str)],
    body: Option<&str>,
) -> (StatusCode, Value) {
    let mut b = Request::builder().method(method).uri(uri);
    for (k, v) in headers {
        b = b.header(*k, *v);
    }
    let req = b
        .body(body.map_or(Body::empty(), |s| Body::from(s.to_string())))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

/// Class-R: this suite must never authenticate as / mutate the shared dev-login snowflake.
#[test]
fn t400_actor_is_not_shared_dev_login_user() {
    assert_ne!(
        ACTOR,
        common::DEV_LOGIN_USER,
        "identity_link must not use the shared DEV_LOGIN_USER — that races auth_refresh \
         arma_linked (T-400)"
    );
    assert_ne!(USER2, common::DEV_LOGIN_USER);
    assert_ne!(PAD_ACTOR, common::DEV_LOGIN_USER);
    let src = include_str!("identity_link.rs");
    // Ban the pre-fix shared-row alias without matching this assertion's own prose.
    assert!(
        !src.lines()
            .any(|l| l.trim_start().starts_with("const DEV_ID")),
        "identity_link must not reintroduce a shared-snowflake const alias"
    );
    // Pre-fix swallowed cleanup nulled the shared row via bind $3. Needle is split so this
    // assert's own source does not contain the forbidden SQL as one literal.
    let forbidden = concat!("UPDATE users SET arma_id = NULL WHERE ", "discord_id = $3");
    assert!(
        !src.contains(forbidden),
        "must not null arma_id via the old shared-row cleanup bind"
    );
}

#[tokio::test]
async fn arma_link_flow() {
    let Some((app, state, pool)) = setup().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };

    // Private actor JWT — does not rewrite `DEV_LOGIN_USER`.
    let access = common::access_token(&state, "identity_link", ACTOR, "admin", false);
    let bearer = format!("Bearer {access}");
    let auth = [(header::AUTHORIZATION.as_str(), bearer.as_str())];
    let json_svc = [
        (header::CONTENT_TYPE.as_str(), "application/json"),
        ("x-service-token", SVC),
    ];

    // Start unlinked.
    let (st, _) = call(&app, "DELETE", "/api/v1/me/link", &auth, None).await;
    assert_eq!(st, StatusCode::OK);
    let (st, body) = call(&app, "GET", "/api/v1/me/link/status", &auth, None).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(body["linked"], false);
    assert_eq!(body["pending_code"], false);

    // Create a code → 201, pending.
    let (st, body) = call(&app, "POST", "/api/v1/me/link", &auth, None).await;
    assert_eq!(st, StatusCode::CREATED);
    let code = body["code"].as_str().unwrap().to_string();
    assert_eq!(code.len(), 6);
    let (_, body) = call(&app, "GET", "/api/v1/me/link/status", &auth, None).await;
    assert_eq!(body["pending_code"], true);

    // Confirm (service-token) → linked.
    let confirm =
        format!(r#"{{"code":"{code}","arma_id":"{ACTOR_ARMA}","arma_character":"Test Char"}}"#);
    let (st, body) = call(
        &app,
        "POST",
        "/api/v1/ingest/link-confirm",
        &json_svc,
        Some(&confirm),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "confirm body={body}");
    assert_eq!(body["linked"], true);
    assert_eq!(body["arma_id"], ACTOR_ARMA);

    let (_, body) = call(&app, "GET", "/api/v1/me/link/status", &auth, None).await;
    assert_eq!(body["linked"], true);
    assert_eq!(body["arma_id"], ACTOR_ARMA);
    assert_eq!(body["arma_character"], "Test Char");

    // Re-confirm the consumed code → 404.
    let (st, _) = call(
        &app,
        "POST",
        "/api/v1/ingest/link-confirm",
        &json_svc,
        Some(&confirm),
    )
    .await;
    assert_eq!(st, StatusCode::NOT_FOUND);

    // No/invalid service token → 401.
    let (st, _) = call(
        &app,
        "POST",
        "/api/v1/ingest/link-confirm",
        &[(header::CONTENT_TYPE.as_str(), "application/json")],
        Some(&confirm),
    )
    .await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);

    // Clash: a second user's code confirming with actor's arma_id → 409.
    common::seed_user(
        &pool,
        USER2,
        "Identity Link User2",
        "identity-link-seed-400002",
        "enlisted",
    )
    .await;
    sqlx::query(
        "INSERT INTO identity_link_codes (code, discord_id, expires_at, created_at) \
         VALUES ('424242', $1, now() + interval '10 minutes', now()) \
         ON CONFLICT (code) DO UPDATE SET discord_id = EXCLUDED.discord_id, \
          expires_at = EXCLUDED.expires_at, consumed_at = NULL",
    )
    .bind(USER2)
    .execute(&pool)
    .await
    .unwrap_or_else(|e| panic!("seed clash code: {e}"));
    let clash = format!(r#"{{"code":"424242","arma_id":"{ACTOR_ARMA}","arma_character":"Dupe"}}"#);
    let (st, body) = call(
        &app,
        "POST",
        "/api/v1/ingest/link-confirm",
        &json_svc,
        Some(&clash),
    )
    .await;
    assert_eq!(st, StatusCode::CONFLICT);
    assert_eq!(body["error"], "arma id already linked to another account");
}

/// T-351 — pin `ingest_link_confirm`'s trim.
///
/// Pre-fix this suite only ever posted clean `"steam-xyz"` ids, so T-326's
/// `req.arma_id.trim()` (`handlers/me.rs`) and T-316's ingest bind of
/// `p.arma_id.trim()` were held by comments, not gates. A regression that stored the
/// padded wire form would make the account read as linked while every future
/// `WHERE arma_id = $1` miss — the exact catastrophe T-326 measured.
///
/// This test posts the padded form, asserts the **stored** value is trimmed, proves
/// the T-326 backfill claims pre-link orphan rows for that trimmed id, and proves a
/// subsequent match ingest resolves the account (linked=1).
#[tokio::test]
async fn padded_arma_id_is_stored_trimmed_and_resolvable() {
    let Some((app, _state, pool)) = setup().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };

    const CODE: &str = "400351";
    const SRC: &str = "m-t351-pad-trim";

    common::seed_user(
        &pool,
        PAD_ACTOR,
        "Identity Link Pad Actor",
        "identity-link-seed-400003",
        "enlisted",
    )
    .await;
    sqlx::query(
        "UPDATE users SET arma_id = NULL, arma_character = '', total_deployments = 0, \
         updated_at = now() WHERE discord_id = $1",
    )
    .bind(PAD_ACTOR)
    .execute(&pool)
    .await
    .unwrap_or_else(|e| panic!("unlink pad actor: {e}"));
    sqlx::query(
        "INSERT INTO identity_link_codes (code, discord_id, expires_at, created_at) \
         VALUES ($1, $2, now() + interval '10 minutes', now()) \
         ON CONFLICT (code) DO UPDATE SET discord_id = EXCLUDED.discord_id, \
          expires_at = EXCLUDED.expires_at, consumed_at = NULL",
    )
    .bind(CODE)
    .bind(PAD_ACTOR)
    .execute(&pool)
    .await
    .unwrap_or_else(|e| panic!("seed pad code: {e}"));

    let json_svc = [
        (header::CONTENT_TYPE.as_str(), "application/json"),
        ("x-service-token", SVC),
    ];

    // Pre-link orphan scoreline under the *trimmed* id (what ingest stores). Without the
    // confirm trim, the claim join would miss these rows forever.
    let pre_ingest = format!(
        r#"{{"match":{{"source_match_id":"{SRC}","outcome":"success","winning_faction":"USA"}},"players":[{{"arma_id":"{PAD_ARMA}","role_played":"SL","source_event_id":"e-t351","counters":{{"kills":7,"deaths":2,"team_kills":0,"longest_kill_m":100,"vehicles_destroyed":1,"is_command":false}}}}]}}"#
    );
    let (st, body) = call(
        &app,
        "POST",
        "/api/v1/ingest/match-results",
        &json_svc,
        Some(&pre_ingest),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "pre-link ingest: {body}");
    assert_eq!(body["unlinked"], 1, "orphan until link: {body}");
    let owned_before: Option<String> = sqlx::query_scalar(
        "SELECT discord_id FROM match_player_stats WHERE arma_id = $1 AND source_event_id = 'e-t351'",
    )
    .bind(PAD_ARMA)
    .fetch_one(&pool)
    .await
    .unwrap_or_else(|e| panic!("read orphan owner: {e}"));
    assert_eq!(owned_before, None, "pre-link row must be unowned");

    // Confirm with PADDED wire form — the only form this suite posts that can detect a trim drop.
    let confirm =
        format!(r#"{{"code":"{CODE}","arma_id":"{PAD_ARMA_PADDED}","arma_character":"Pad Char"}}"#);
    let (st, body) = call(
        &app,
        "POST",
        "/api/v1/ingest/link-confirm",
        &json_svc,
        Some(&confirm),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "padded confirm: {body}");
    assert_eq!(body["linked"], true);
    assert_eq!(
        body["arma_id"], PAD_ARMA,
        "response must echo the trimmed id, not the padded wire form: {body}"
    );
    assert_ne!(
        body["arma_id"].as_str(),
        Some(PAD_ARMA_PADDED),
        "padded wire form must not round-trip"
    );

    let stored: Option<String> =
        sqlx::query_scalar("SELECT arma_id FROM users WHERE discord_id = $1")
            .bind(PAD_ACTOR)
            .fetch_one(&pool)
            .await
            .unwrap_or_else(|e| panic!("read stored arma_id: {e}"));
    assert_eq!(
        stored.as_deref(),
        Some(PAD_ARMA),
        "users.arma_id must be btrim'd — stored={stored:?}"
    );
    assert_ne!(
        stored.as_deref(),
        Some(PAD_ARMA_PADDED),
        "padded bytes must not land in users.arma_id"
    );

    // T-326 claim: orphan rows for the trimmed id now belong to PAD_ACTOR.
    let owned_after: Option<String> = sqlx::query_scalar(
        "SELECT discord_id FROM match_player_stats WHERE arma_id = $1 AND source_event_id = 'e-t351'",
    )
    .bind(PAD_ARMA)
    .fetch_one(&pool)
    .await
    .unwrap_or_else(|e| panic!("read claimed owner: {e}"));
    assert_eq!(
        owned_after.as_deref(),
        Some(PAD_ACTOR),
        "backfill must claim pre-link orphans under the trimmed id"
    );
    let deployments: i64 =
        sqlx::query_scalar("SELECT total_deployments FROM users WHERE discord_id = $1")
            .bind(PAD_ACTOR)
            .fetch_one(&pool)
            .await
            .unwrap_or_else(|e| panic!("read deployments: {e}"));
    assert_eq!(
        deployments, 1,
        "link-confirm recompute must count the claimed match"
    );

    // Ingest resolver: a later match with the clean id finds the account.
    let post_ingest = format!(
        r#"{{"match":{{"source_match_id":"{SRC}-2","outcome":"success","winning_faction":"USA"}},"players":[{{"arma_id":"{PAD_ARMA}","role_played":"SL","source_event_id":"e-t351-b","counters":{{"kills":3,"deaths":0,"team_kills":0,"longest_kill_m":50,"vehicles_destroyed":0,"is_command":false}}}}]}}"#
    );
    let (st, body) = call(
        &app,
        "POST",
        "/api/v1/ingest/match-results",
        &json_svc,
        Some(&post_ingest),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "post-link ingest: {body}");
    assert_eq!(
        body["linked"], 1,
        "resolver must find the trimmed link: {body}"
    );
    assert_eq!(body["unlinked"], 0, "no orphan after trim-store: {body}");
    let post_owner: Option<String> = sqlx::query_scalar(
        "SELECT discord_id FROM match_player_stats WHERE arma_id = $1 AND source_event_id = 'e-t351-b'",
    )
    .bind(PAD_ARMA)
    .fetch_one(&pool)
    .await
    .unwrap_or_else(|e| panic!("read post-link owner: {e}"));
    assert_eq!(post_owner.as_deref(), Some(PAD_ACTOR));
}
