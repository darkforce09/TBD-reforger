//! Arma identity-link flow: create a code, confirm it with a game server's machine credential,
//! clash, unlink.
//! Skips unless `TEST_DATABASE_URL` points at a migrated DB.
//!
//! # Fixture ownership + DB target guard
//!
//! Authenticating as the shared `dev-login` snowflake ([`common::DEV_LOGIN_USER`]) is forbidden
//! here: `GET /me` computes `arma_linked` from the **database** row
//! (`api_identity_and_access::handlers::member_profile`), and
//! `auth_refresh.rs` asserts `arma_linked == true` on that same shared id, so nulling that row's
//! `arma_id` and relinking it interleaves ahead of auth_refresh under a concurrent
//! `cargo test -p api_server` and fails it. Actors here live in a private snowflake range and
//! are minted via [`common::access_token`] (writes nothing on the shared row).
//! [`common::require_test_database_url`] refuses `tbd_reforger` before any UPDATE/DELETE;
//! cleanup stays scoped to suite-owned discord_id / arma_id values (never the shared row).
//!
//! # Intra-suite seed race
//!
//! `arma_link_flow` and `padded_arma_id_is_stored_trimmed_and_resolvable` both call
//! [`setup`], which `seed_user`s ACTOR with placeholder `identity-link-seed-400001` before
//! clearing it. Under `cargo test` (parallel by default) two `setup()`s can race on
//! `UNIQUE(arma_id)` (`idx_users_arma_id`): one holds the placeholder while the other
//! `ON CONFLICT … SET arma_id = EXCLUDED.arma_id` hits the same value still owned by any
//! row (or the clash partner still holding `identity-link-seed-400002`). Isolated
//! `--test identity_link` often passes; full `cargo test -p api_server` on the shared
//! gate DB fails. [`DB_LOCK`] therefore serialises the async tests, and `setup` releases the
//! seed placeholders before `seed_user` (same pattern as live arma ids).

use crate::{common, telemetry_support};

use api_configuration::configuration::Config;
use api_state::AppState;

use api_server::router::router;
use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use serde_json::Value;
use sqlx::PgPool;
use telemetry_support::match_reports::ReportingServer;
use tower::ServiceExt;

/// Serialise DB-touching tests in this binary — both async tests share ACTOR /
/// seed placeholders on one database (see the seed race in the module docs). Pattern:
/// `null_tolerance_reads.rs`.
static DB_LOCK: std::sync::LazyLock<tokio::sync::Mutex<()>> =
    std::sync::LazyLock::new(|| tokio::sync::Mutex::new(()));

/// Primary actor under test — namespaced so no sibling binary can rewrite its `arma_id`.
const ACTOR: &str = "000000000000400001";
/// Clash partner for the 409 path (same private range).
const USER2: &str = "000000000000400002";
/// Padded-trim actor (private range). Must NOT be telemetry `PLAYER_DISCORD`
/// (`…400003`) — that collision piles this suite's kills onto the telemetry leaderboard row.
const PAD_ACTOR: &str = "000000000000400013";
const ACTOR_ARMA: &str = "identity-link-arma-400001";
/// Canonical (trimmed) Steam id for the trim pin.
const PAD_ARMA: &str = "identity-link-arma-padded-400013";
/// Wire form that must NOT land in `users.arma_id` — spaces around the id.
const PAD_ARMA_PADDED: &str = "  identity-link-arma-padded-400013  ";
/// Placeholders `seed_user` writes before the unlink step — must be released first.
const SEED_ARMA_ACTOR: &str = "identity-link-seed-400001";
const SEED_ARMA_USER2: &str = "identity-link-seed-400002";
const SEED_ARMA_PAD: &str = "identity-link-seed-400013";

/// A `host_agent` credential of `server`: a real machine credential of the wrong executor kind.
async fn host_agent_secret(pool: &PgPool, server: uuid::Uuid) -> String {
    let credential = uuid::Uuid::new_v4();
    let secret = format!(
        "tbdm_{}_{}",
        credential.simple(),
        api_http_layer::authentication_primitives::random_token(32)
    );
    sqlx::query(
        "INSERT INTO server_machine_credentials (id, server_id, executor_kind, secret_sha256, label, created_by)
         VALUES ($1, $2, 'host_agent', $3, 'Identity link host agent', $4)",
    )
    .bind(credential)
    .bind(server)
    .bind(api_http_layer::authentication_primitives::hash_token(&secret))
    .bind(common::DEV_LOGIN_USER)
    .execute(pool)
    .await
    .expect("store host agent credential");
    secret
}

async fn setup() -> Option<(Router, AppState, PgPool)> {
    // Unset → skip; set-but-live-DB → panic before connect/UPDATE/DELETE.
    let url = common::require_test_database_url()?;
    let pool = api_database::connect(&url).await.expect("connect");
    api_database::migrate(&pool).await.expect("migrate");

    // Scoped cleanup only — never touch `common::DEV_LOGIN_USER`. Fail loud on SQL errors: a
    // swallowed error hides a unique-index / migrate race and leaves a poisoned row behind.
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
    // Release live arma ids *and* seed placeholders (UNIQUE, non-partial). Without the
    // placeholders, a concurrent/prior `seed_user` still holding them trips
    // `idx_users_arma_id` on the next `setup`.
    sqlx::query("UPDATE users SET arma_id = NULL WHERE arma_id = ANY($1)")
        .bind(vec![
            ACTOR_ARMA.to_string(),
            PAD_ARMA.to_string(),
            SEED_ARMA_ACTOR.to_string(),
            SEED_ARMA_USER2.to_string(),
            SEED_ARMA_PAD.to_string(),
        ])
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
            "m-link-pad-trim".to_string(),
            "m-link-pad-trim-2".to_string(),
        ])
        .execute(&pool)
        .await
        .unwrap_or_else(|e| panic!("identity_link release pad match: {e}"));

    common::seed_user(
        &pool,
        ACTOR,
        "Identity Link Actor",
        SEED_ARMA_ACTOR,
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

    let state = api_server::composition::application_state(
        pool.clone(),
        Config::for_tests(url, "identity-secret"),
    );
    let app = router(state.clone());
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
        .expect("the request builds");
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = to_bytes(resp.into_body(), usize::MAX)
        .await
        .expect("the response body reads to the end");
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

#[tokio::test]
async fn arma_link_flow() {
    let _serial = DB_LOCK.lock().await;
    let Some((app, state, pool)) = setup().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };

    // Private actor JWT — does not rewrite `DEV_LOGIN_USER`.
    let access = common::access_token(&state, "identity_link", ACTOR, "admin", false).await;
    let bearer = format!("Bearer {access}");
    let auth = [(header::AUTHORIZATION.as_str(), bearer.as_str())];
    let reporter = ReportingServer::open(&app, &pool, "Identity link server").await;
    let machine = format!("Bearer {}", reporter.session.secret);
    let json_machine = [
        (header::CONTENT_TYPE.as_str(), "application/json"),
        (header::AUTHORIZATION.as_str(), machine.as_str()),
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

    // Confirm (game server machine credential) → linked.
    let confirm =
        format!(r#"{{"code":"{code}","arma_id":"{ACTOR_ARMA}","arma_character":"Test Char"}}"#);
    let (st, body) = call(
        &app,
        "POST",
        "/api/v1/ingest/link-confirm",
        &json_machine,
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

    // An identical retry confirms the existing result without consuming the code again.
    let (st, _) = call(
        &app,
        "POST",
        "/api/v1/ingest/link-confirm",
        &json_machine,
        Some(&confirm),
    )
    .await;
    assert_eq!(st, StatusCode::OK);

    let changed_retry = serde_json::json!({"code": code, "arma_id": ACTOR_ARMA, "arma_character": "Uncommitted name"}).to_string();
    let (st, retry_body) = call(
        &app,
        "POST",
        "/api/v1/ingest/link-confirm",
        &json_machine,
        Some(&changed_retry),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(
        retry_body["arma_character"], "Test Char",
        "retry returns the persisted result"
    );

    // No machine credential → 401.
    let (st, _) = call(
        &app,
        "POST",
        "/api/v1/ingest/link-confirm",
        &[(header::CONTENT_TYPE.as_str(), "application/json")],
        Some(&confirm),
    )
    .await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);
    // The retired shared service-token header grants nothing.
    let (st, _) = call(
        &app,
        "POST",
        "/api/v1/ingest/link-confirm",
        &[
            (header::CONTENT_TYPE.as_str(), "application/json"),
            ("x-service-token", "test-service-token"),
        ],
        Some(&confirm),
    )
    .await;
    assert_eq!(
        st,
        StatusCode::UNAUTHORIZED,
        "the retired header is not a credential"
    );
    // A machine credential of the host agent kind is authenticated but not allowed.
    let host_agent = format!(
        "Bearer {}",
        host_agent_secret(&pool, reporter.server_id).await
    );
    let (st, _) = call(
        &app,
        "POST",
        "/api/v1/ingest/link-confirm",
        &[
            (header::CONTENT_TYPE.as_str(), "application/json"),
            (header::AUTHORIZATION.as_str(), host_agent.as_str()),
        ],
        Some(&confirm),
    )
    .await;
    assert_eq!(
        st,
        StatusCode::FORBIDDEN,
        "only a mod_runtime credential confirms links"
    );

    // Clash: a second user's code confirming with actor's arma_id → 409.
    common::seed_user(
        &pool,
        USER2,
        "Identity Link User2",
        SEED_ARMA_USER2,
        "enlisted",
    )
    .await;
    sqlx::query("UPDATE users SET arma_id = NULL WHERE discord_id = $1")
        .bind(USER2)
        .execute(&pool)
        .await
        .unwrap();
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
        &json_machine,
        Some(&clash),
    )
    .await;
    assert_eq!(st, StatusCode::CONFLICT);
    assert_eq!(body["error"], "arma id already linked to another account");
}

/// Pin `ingest_link_confirm`'s trim.
///
/// A suite that only ever posts clean `"steam-xyz"` ids leaves `req.arma_id.trim()`
/// (`api_identity_and_access::handlers::arma_link_codes`) and the ingest bind of `p.arma_id.trim()`
/// held by comments, not
/// gates. A regression that stored the padded wire form makes the account read as linked
/// while every future `WHERE arma_id = $1` misses.
///
/// This test posts the padded form, asserts the **stored** value is trimmed, proves
/// the backfill claims pre-link orphan rows for that trimmed id, and proves a
/// subsequent match ingest resolves the account (linked=1).
#[tokio::test]
async fn padded_arma_id_is_stored_trimmed_and_resolvable() {
    let _serial = DB_LOCK.lock().await;
    let Some((app, _state, pool)) = setup().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };

    const CODE: &str = "400351";
    const SRC: &str = "m-link-pad-trim";

    common::seed_user(
        &pool,
        PAD_ACTOR,
        "Identity Link Pad Actor",
        SEED_ARMA_PAD,
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

    let reporter = ReportingServer::open(&app, &pool, "Identity link padded server").await;
    let machine = format!("Bearer {}", reporter.session.secret);
    let json_machine = [
        (header::CONTENT_TYPE.as_str(), "application/json"),
        (header::AUTHORIZATION.as_str(), machine.as_str()),
    ];

    // Pre-link orphan scoreline under the *trimmed* id (what ingest stores). Without the
    // confirm trim, the claim join would miss these rows forever.
    let pre_ingest = format!(
        r#"{{"match":{{"source_match_id":"{SRC}","outcome":"success","winning_faction":"USA"}},"players":[{{"arma_id":"{PAD_ARMA}","role_played":"SL","source_event_id":"e-link","counters":{{"kills":7,"deaths":2,"team_kills":0,"longest_kill_m":100,"vehicles_destroyed":1,"is_command":false}}}}]}}"#
    );
    let pre_ingest: Value = serde_json::from_str(&pre_ingest).expect("a results body is JSON");
    let (st, body) = reporter.report_results(&app, &pre_ingest).await;
    assert_eq!(st, StatusCode::OK, "pre-link ingest: {body}");
    assert_eq!(body["unlinked"], 1, "orphan until link: {body}");
    let owned_before: Option<String> = sqlx::query_scalar(
        "SELECT discord_id FROM match_player_stats WHERE arma_id = $1 AND source_event_id = 'e-link'",
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
        &json_machine,
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

    // Backfill claim: orphan rows for the trimmed id now belong to PAD_ACTOR.
    let owned_after: Option<String> = sqlx::query_scalar(
        "SELECT discord_id FROM match_player_stats WHERE arma_id = $1 AND source_event_id = 'e-link'",
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
        r#"{{"match":{{"source_match_id":"{SRC}-2","outcome":"success","winning_faction":"USA"}},"players":[{{"arma_id":"{PAD_ARMA}","role_played":"SL","source_event_id":"e-link-b","counters":{{"kills":3,"deaths":0,"team_kills":0,"longest_kill_m":50,"vehicles_destroyed":0,"is_command":false}}}}]}}"#
    );
    let post_ingest: Value = serde_json::from_str(&post_ingest).expect("a results body is JSON");
    let (st, body) = reporter.report_results(&app, &post_ingest).await;
    assert_eq!(st, StatusCode::OK, "post-link ingest: {body}");
    assert_eq!(
        body["linked"], 1,
        "resolver must find the trimmed link: {body}"
    );
    assert_eq!(body["unlinked"], 0, "no orphan after trim-store: {body}");
    let post_owner: Option<String> = sqlx::query_scalar(
        "SELECT discord_id FROM match_player_stats WHERE arma_id = $1 AND source_event_id = 'e-link-b'",
    )
    .bind(PAD_ARMA)
    .fetch_one(&pool)
    .await
    .unwrap_or_else(|e| panic!("read post-link owner: {e}"));
    assert_eq!(post_owner.as_deref(), Some(PAD_ACTOR));
}
