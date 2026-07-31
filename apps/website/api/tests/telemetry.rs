//! Telemetry ingest — server-status upsert (+ low-FPS audit + status read-back) and
//! match-results (idempotent, arma→discord resolve, leaderboard MV refresh, stats
//! recompute). Skips without `TEST_DATABASE_URL`.
//!
//! # Fixture ownership (T-400)
//!
//! `PLAYER_DISCORD` used to be the content-golden Vance snowflake (`…003`). T-334 vacated
//! the `events.rs` half of that double-booking; this suite's half still rewrote Vance via
//! `ON CONFLICT … DO UPDATE SET arma_id = EXCLUDED.arma_id`. The id is now in the T-400
//! private range. Admin tokens go through [`common::dev_login_token`].

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use serde_json::Value;
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;
use website_api::config::Config;
use website_api::state::AppState;
use website_api::{app, db};

mod common;

const SVC: &str = "test-service-token";
/// Private player — must never be content_golden Vance (`…003`). See
/// [`t400_player_is_not_content_golden_vance`].
const PLAYER_DISCORD: &str = "000000000000400003";
const PLAYER_ARMA: &str = "telemetry-arma-400003";

async fn boot() -> Option<(Router, PgPool)> {
    let url = common::require_test_database_url()?;
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    let app = app::router(AppState::new(
        pool.clone(),
        Config::for_tests(url, "tele-secret"),
    ));
    Some((app, pool))
}

async fn admin_token(app: &Router) -> String {
    common::dev_login_token(app, "telemetry", "admin").await
}

/// Class-R: the main ingest player must not be content_golden Vance (T-400 / T-334 residual).
#[test]
fn t400_player_is_not_content_golden_vance() {
    assert_ne!(
        PLAYER_DISCORD, "000000000000000003",
        "PLAYER_DISCORD must not be content_golden Vance — T-334 vacated events; T-400 \
         vacates telemetry"
    );
    assert_ne!(PLAYER_ARMA, "76561190000000003");
}

async fn call(
    app: &Router,
    method: &str,
    uri: &str,
    bearer: Option<&str>,
    svc: Option<&str>,
    body: Option<&str>,
) -> (StatusCode, Value) {
    let mut b = Request::builder().method(method).uri(uri);
    if let Some(t) = bearer {
        b = b.header(header::AUTHORIZATION, format!("Bearer {t}"));
    }
    if let Some(s) = svc {
        b = b.header("x-service-token", s);
    }
    if body.is_some() {
        b = b.header(header::CONTENT_TYPE, "application/json");
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

#[tokio::test]
async fn telemetry_ingest_closes_the_loop() {
    let Some((app, pool)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let admin = admin_token(&app).await;

    // A server row (for the status read-back) + an arma-linked player.
    let server_id: Uuid = sqlx::query_scalar(
        "INSERT INTO servers (name, ip, port, is_active) VALUES ('Tele Srv', '127.0.0.1'::inet, 2001, true) RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO users (discord_id, username, discord_handle, avatar_url, arma_id, arma_character, role, is_banned, ban_reason, created_at, updated_at) \
         VALUES ($1, 'Player', 'player', '', $2, '[TBD] Player', 'enlisted', false, '', now(), now()) \
         ON CONFLICT (discord_id) DO UPDATE SET arma_id = EXCLUDED.arma_id",
    )
    .bind(PLAYER_DISCORD)
    .bind(PLAYER_ARMA)
    .execute(&pool)
    .await
    .unwrap();

    // Healthy status ingest (service-token).
    let ok = format!(
        r#"{{"server_id":"{server_id}","is_online":true,"player_count":10,"max_players":64,"server_fps":60.0}}"#
    );
    let (st, r) = call(
        &app,
        "POST",
        "/api/v1/ingest/server-status",
        None,
        Some(SVC),
        Some(&ok),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "ingest: {r}");
    assert_eq!(r["ok"], true);

    // No service token → 401.
    let (st, _) = call(
        &app,
        "POST",
        "/api/v1/ingest/server-status",
        None,
        None,
        Some(&ok),
    )
    .await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);

    // Low-FPS ingest → crosses the threshold → WARN audit written.
    let low = format!(
        r#"{{"server_id":"{server_id}","is_online":true,"player_count":12,"server_fps":15.0}}"#
    );
    let (st, _) = call(
        &app,
        "POST",
        "/api/v1/ingest/server-status",
        None,
        Some(SVC),
        Some(&low),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    let warns: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM audit_logs WHERE action = 'server.low_fps' AND target_id = $1",
    )
    .bind(server_id.to_string())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(warns >= 1, "low-fps WARN audit written");

    // Status read-back reflects the latest ingest (numeric fps decoded to f64).
    let (st, s) = call(
        &app,
        "GET",
        &format!("/api/v1/servers/{server_id}/status"),
        Some(&admin),
        None,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(s["status"]["player_count"], 12);
    assert_eq!(s["status"]["server_fps"], 15.0);

    // Match results ingest → resolves arma→discord, records stats.
    //
    // T-316: this body used to omit `team_kills` / `longest_kill_m` /
    // `vehicles_destroyed` / `is_command` and let `#[serde(default)]` fill them with zeros.
    // That default was a mechanical carry-over of Go's zero-value JSON decoding from the
    // T-145 port, not a designed contract, and it is exactly what let a re-ingest wipe a
    // real scoreline — so the stat block is now spelled out in full.
    //
    // T-393: and it is spelled out inside `counters`, which is where the whole block lives now.
    // Present = authoritative, so this is still the "full replace" path T-316 designed; what
    // changed is that a sender with no scoreline to state may omit the block instead of being
    // rejected (`the_shipping_mod_payload_is_accepted_verbatim`).
    let match_body = format!(
        r#"{{"match":{{"source_match_id":"m-tele-1","outcome":"success","winning_faction":"USA"}},"players":[{{"arma_id":"{PLAYER_ARMA}","role_played":"SL","source_event_id":"e1","counters":{{"kills":5,"deaths":1,"team_kills":0,"longest_kill_m":0,"vehicles_destroyed":0,"is_command":false}}}}]}}"#
    );
    let (st, mr) = call(
        &app,
        "POST",
        "/api/v1/ingest/match-results",
        None,
        Some(SVC),
        Some(&match_body),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "match: {mr}");
    assert_eq!(mr["players"], 1);
    let match_id = mr["match_id"].as_str().unwrap().to_string();

    // Idempotent: same source_match_id reuses the match.
    let (st, mr2) = call(
        &app,
        "POST",
        "/api/v1/ingest/match-results",
        None,
        Some(SVC),
        Some(&match_body),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(
        mr2["match_id"],
        match_id.as_str(),
        "same source_match_id → same match"
    );

    // Leaderboard MV refreshed → the player appears with 5 kills.
    let (st, lb) = call(
        &app,
        "GET",
        "/api/v1/leaderboards?category=kd",
        Some(&admin),
        None,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    let row = lb["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["discord_id"] == PLAYER_DISCORD);
    assert!(row.is_some(), "player on the leaderboard after refresh");
    assert_eq!(row.unwrap()["kills"], 5);

    // Denormalized user stats recomputed (1 distinct match).
    let (st, stats) = call(
        &app,
        "GET",
        &format!("/api/v1/users/{PLAYER_DISCORD}/stats"),
        Some(&admin),
        None,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(stats["total_operations"], 1);
}

/// T-316 — a partial re-ingest must not walk a finished match backwards or zero a
/// scoreline. Every body below is one a buggy or retried game server could plausibly send;
/// each one used to return 200 and destroy data.
///
/// Keep the ingest calls in this test under the strict limiter's burst (1/s, burst 10).
#[tokio::test]
async fn partial_match_reingest_cannot_revert_or_zero() {
    let Some((app, pool)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    const ARMA: &str = "t316-arma-revert";
    const DISCORD: &str = "000000000000316001";
    const SRC: &str = "m-t316-revert";

    sqlx::query(
        "INSERT INTO users (discord_id, username, discord_handle, avatar_url, arma_id, arma_character, role, is_banned, ban_reason, created_at, updated_at) \
         VALUES ($1, 'T316', 't316', '', $2, '[TBD] T316', 'enlisted', false, '', now(), now()) \
         ON CONFLICT (discord_id) DO UPDATE SET arma_id = EXCLUDED.arma_id",
    )
    .bind(DISCORD)
    .bind(ARMA)
    .execute(&pool)
    .await
    .unwrap();
    // `matches` has no cascade to `match_player_stats`, so dropping only the match would
    // orphan the stat rows — and `leaderboard_totals` sums every row for a discord_id, so a
    // second run would read 34 kills instead of 17. Clear the stats first, both here and at
    // the end, and keep this test's ids to itself.
    let clean = |pool: PgPool| async move {
        sqlx::query("DELETE FROM match_player_stats WHERE arma_id = $1")
            .bind(ARMA)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM matches WHERE source_match_id = $1")
            .bind(SRC)
            .execute(&pool)
            .await
            .unwrap();
    };
    clean(pool.clone()).await;

    // The honest ingest: a completed, won match with a real scoreline and an AAR link.
    let full = format!(
        r#"{{"match":{{"source_match_id":"{SRC}","outcome":"success","winning_faction":"USA","aar_replay_url":"https://aar.tbd/{SRC}.json","ended_at":"2026-07-26T20:14:00Z"}},"players":[{{"arma_id":"{ARMA}","role_played":"SL","source_event_id":"e-t316","counters":{{"kills":17,"deaths":3,"team_kills":1,"longest_kill_m":842,"vehicles_destroyed":4,"is_command":true,"command_win":true}}}}]}}"#
    );
    let (st, r) = call(
        &app,
        "POST",
        "/api/v1/ingest/match-results",
        None,
        Some(SVC),
        Some(&full),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "seed: {r}");

    type MatchRow = (String, Option<String>, Option<String>, bool);
    let read_match = |pool: PgPool| async move {
        sqlx::query_as::<_, MatchRow>(
            "SELECT outcome::text, winning_faction, aar_replay_url, ended_at IS NOT NULL \
             FROM matches WHERE source_match_id = $1",
        )
        .bind(SRC)
        .fetch_one(&pool)
        .await
        .unwrap()
    };
    type StatRow = (i64, i64, i64, i64, i64, bool);
    let read_stats = |pool: PgPool| async move {
        sqlx::query_as::<_, StatRow>(
            "SELECT kills, deaths, team_kills, longest_kill_m, vehicles_destroyed, is_command \
             FROM match_player_stats WHERE arma_id = $1",
        )
        .bind(ARMA)
        .fetch_one(&pool)
        .await
        .unwrap()
    };

    let before = read_match(pool.clone()).await;
    assert_eq!(
        before,
        (
            "success".into(),
            Some("USA".into()),
            Some(format!("https://aar.tbd/{SRC}.json")),
            true
        )
    );
    assert_eq!(read_stats(pool.clone()).await, (17, 3, 1, 842, 4, true));

    // (1) A partial match body — this is the one that reverted `success`/`USA` to
    // `pending`/`''` and dropped both the AAR link and `ended_at`.
    let (st, r) = call(
        &app,
        "POST",
        "/api/v1/ingest/match-results",
        None,
        Some(SVC),
        Some(&format!(
            r#"{{"match":{{"source_match_id":"{SRC}"}},"players":[]}}"#
        )),
    )
    .await;
    assert_eq!(st, StatusCode::BAD_REQUEST, "partial match body: {r}");
    assert_eq!(read_match(pool.clone()).await, before, "match untouched");

    // (2) A player row missing a required *identity* field — this body has neither
    // `role_played` nor a scoreline, and it used to zero a 17/3 line.
    //
    // **T-393 re-read this case, because the reason it is a 400 changed.** It is now rejected
    // for the missing `role_played`, not for the missing counters: counters live in an optional
    // nested block, and omitting the block is a legal statement meaning "I make no claim about
    // the scoreline". What T-316 actually established — that an omission must never be a write
    // — is unchanged and is asserted directly by
    // `absent_counters_are_not_a_write_on_reingest`, which sends a *well-formed* counters-less
    // row and proves the stored 17/3 survives it. The two tests together say: silence never
    // writes, and an incomplete identity is still a 400.
    let (st, r) = call(
        &app,
        "POST",
        "/api/v1/ingest/match-results",
        None,
        Some(SVC),
        Some(&format!(
            r#"{{"match":{{"source_match_id":"{SRC}","outcome":"success","winning_faction":"USA"}},"players":[{{"arma_id":"{ARMA}","source_event_id":"e-t316"}}]}}"#
        )),
    )
    .await;
    assert_eq!(st, StatusCode::BAD_REQUEST, "partial player body: {r}");
    assert_eq!(
        read_stats(pool.clone()).await,
        (17, 3, 1, 842, 4, true),
        "counters untouched"
    );

    // The ticket's propagation claim, checked at the source: `leaderboard_totals` sums
    // `match_player_stats`, so a zeroed row really would have reached the leaderboard.
    // (`users` does NOT — it carries no kill/death columns at all.)
    sqlx::query("REFRESH MATERIALIZED VIEW CONCURRENTLY leaderboard_totals")
        .execute(&pool)
        .await
        .ok();
    let lb_kills: Option<i64> =
        sqlx::query_scalar("SELECT kills::int8 FROM leaderboard_totals WHERE discord_id = $1")
            .bind(DISCORD)
            .fetch_optional(&pool)
            .await
            .unwrap();
    assert_eq!(
        lb_kills,
        Some(17),
        "leaderboard MV still shows the real kills"
    );

    // (3) `{}` used to mint an anonymous `pending` match row on every call.
    let anon_before: i64 =
        sqlx::query_scalar("SELECT count(*) FROM matches WHERE source_match_id IS NULL")
            .fetch_one(&pool)
            .await
            .unwrap();
    let (st, r) = call(
        &app,
        "POST",
        "/api/v1/ingest/match-results",
        None,
        Some(SVC),
        Some("{}"),
    )
    .await;
    assert_eq!(st, StatusCode::BAD_REQUEST, "empty body: {r}");
    let anon_after: i64 =
        sqlx::query_scalar("SELECT count(*) FROM matches WHERE source_match_id IS NULL")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(anon_before, anon_after, "no garbage match row minted");

    clean(pool.clone()).await;
}

/// T-316 — a heartbeat is a merge, not a wipe. Unlike a match result, a partial heartbeat
/// is legitimate, so the fix here is `COALESCE` (absent = no new reading) rather than
/// mandatory fields; only a heartbeat with nothing at all to say is rejected.
#[tokio::test]
async fn partial_heartbeat_merges_and_does_not_fire_a_false_low_fps_warn() {
    let Some((app, pool)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let server_id: Uuid = sqlx::query_scalar(
        "INSERT INTO servers (name, ip, port, is_active) VALUES ('T316 Merge Srv', '127.0.0.1'::inet, 2316, true) RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    let ingest = |body: String| {
        let app = app.clone();
        async move {
            call(
                &app,
                "POST",
                "/api/v1/ingest/server-status",
                None,
                Some(SVC),
                Some(&body),
            )
            .await
        }
    };
    type StatusRow = (bool, i64, i64, f64, i64, String, String);
    let read_status = |pool: PgPool| async move {
        sqlx::query_as::<_, StatusRow>(
            "SELECT is_online, player_count, max_players, server_fps::float8, uptime_seconds, \
             COALESCE(ingame_time, ''), COALESCE(ingame_weather, '') \
             FROM server_statuses WHERE server_id = $1",
        )
        .bind(server_id)
        .fetch_one(&pool)
        .await
        .unwrap()
    };
    let counts = |pool: PgPool| async move {
        let hist: i64 =
            sqlx::query_scalar("SELECT count(*) FROM server_status_histories WHERE server_id = $1")
                .bind(server_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        let warns: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM audit_logs WHERE action = 'server.low_fps' AND target_id = $1",
        )
        .bind(server_id.to_string())
        .fetch_one(&pool)
        .await
        .unwrap();
        (hist, warns)
    };

    // A healthy, fully-populated heartbeat.
    let (st, r) = ingest(format!(
        r#"{{"server_id":"{server_id}","is_online":true,"player_count":48,"max_players":64,"server_fps":58.5,"uptime_seconds":7200,"ingame_time":"18:00","ingame_weather":"clear"}}"#
    ))
    .await;
    assert_eq!(st, StatusCode::OK, "healthy heartbeat: {r}");
    let healthy = (
        true,
        48,
        64,
        58.5,
        7200,
        "18:00".to_string(),
        "clear".to_string(),
    );
    assert_eq!(read_status(pool.clone()).await, healthy);
    assert_eq!(counts(pool.clone()).await, (1, 0));

    // The reported defect: a heartbeat carrying only liveness used to write
    // `player_count=0, server_fps=0, max_players=0, uptime=0`, append a permanent `0/0.0`
    // history sample, and fire a false `server.low_fps` WARN.
    let (st, r) = ingest(format!(r#"{{"server_id":"{server_id}","is_online":true}}"#)).await;
    assert_eq!(st, StatusCode::OK, "partial heartbeat: {r}");
    assert_eq!(
        read_status(pool.clone()).await,
        healthy,
        "nothing clobbered"
    );
    assert_eq!(
        counts(pool.clone()).await,
        (1, 0),
        "no phantom sample, no false WARN"
    );

    // A heartbeat that does carry a reading moves only that field, and the history sample
    // it appends uses the merged row rather than the omitted fields' zeros.
    let (st, _) = ingest(format!(
        r#"{{"server_id":"{server_id}","player_count":52}}"#
    ))
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(
        read_status(pool.clone()).await,
        (true, 52, 64, 58.5, 7200, "18:00".into(), "clear".into())
    );
    let sample: (i64, f64) = sqlx::query_as(
        "SELECT player_count, server_fps::float8 FROM server_status_histories \
         WHERE server_id = $1 ORDER BY recorded_at DESC, id DESC LIMIT 1",
    )
    .bind(server_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(sample, (52, 58.5), "sample carries the merged FPS, not 0.0");

    // A real low-FPS reading must still trip the edge-triggered WARN.
    let (st, _) = ingest(format!(
        r#"{{"server_id":"{server_id}","server_fps":11.5}}"#
    ))
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(
        counts(pool.clone()).await.1,
        1,
        "genuine low FPS still warns"
    );

    // A heartbeat that says nothing at all is a malformed request, not a zero reading.
    let (st, r) = ingest(format!(r#"{{"server_id":"{server_id}"}}"#)).await;
    assert_eq!(st, StatusCode::BAD_REQUEST, "bare heartbeat: {r}");
    let (st, _) = ingest("{}".to_string()).await;
    assert_eq!(st, StatusCode::BAD_REQUEST, "empty body");

    sqlx::query("DELETE FROM server_status_histories WHERE server_id = $1")
        .bind(server_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM server_statuses WHERE server_id = $1")
        .bind(server_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM servers WHERE id = $1")
        .bind(server_id)
        .execute(&pool)
        .await
        .unwrap();
}

/// T-347 — a blank `source_match_id` must never reach the UNIQUE index, and the dedupe lookup
/// must agree with the bind about what the key is.
///
/// The two halves of `upsert_match` used to disagree: the lookup guarded on `!s.is_empty()`
/// against the raw string, the INSERT bound the raw `Option`. Measured on a throwaway DB before
/// the fix, both branches destroyed data and neither told anyone:
///
/// - `"   "` passed the guard, so it became a live dedupe key. Three genuinely different matches
///   collapsed onto one row — outcome walked `success → failure → aborted`, `winning_faction`
///   ended as match #2's `RUS` under match #3's AAR link, `started_at` stayed match #1's, one
///   player's `17/3` and `2/9` were both replaced by `0/1`, and two other players' lines from two
///   different matches were reattributed to a roster that never existed. `total_deployments` read
///   `1` instead of `3` and `leaderboard_totals` read `0 kills / 1 mission` instead of `19 / 3`,
///   refreshed in the same request. All three POSTs returned **200**.
/// - `""` failed the guard and was bound anyway: POST #1 inserted `''`, and every later POST
///   re-inserted it, hit `23505` on `idx_matches_source_match_id`, and got a bare **500** —
///   permanently, for every body that sender sent afterwards.
/// - `"m-x"` and `"  m-x  "` were two different matches.
///
/// The last case is why the guard could not simply be trimmed on its own: a trimming guard with an
/// untrimmed bind is the same defect wearing different clothes. Both now read one normalized value
/// (`source_match_key`), so the padded form resolving to the same match is the *positive* proof
/// they agree, and it is asserted below alongside the rejections.
///
/// Keep the ingest calls under the strict limiter's burst (1/s, burst 10) — this test spends 6.
#[tokio::test]
async fn a_blank_source_match_id_cannot_become_a_dedupe_key() {
    let Some((app, pool)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    const ARMA: &str = "t347-arma-blank";
    const DISCORD: &str = "000000000000347001";
    const SRC: &str = "m-t347-blank";
    const EV: &str = "e-t347";

    sqlx::query(
        "INSERT INTO users (discord_id, username, discord_handle, avatar_url, arma_id, arma_character, role, is_banned, ban_reason, created_at, updated_at) \
         VALUES ($1, 'T347', 't347', '', $2, '[TBD] T347', 'enlisted', false, '', now(), now()) \
         ON CONFLICT (discord_id) DO UPDATE SET arma_id = EXCLUDED.arma_id",
    )
    .bind(DISCORD)
    .bind(ARMA)
    .execute(&pool)
    .await
    .unwrap();
    // Same reasoning as the T-316 test above: `matches` does not cascade to
    // `match_player_stats`, and `leaderboard_totals` sums every row for a discord_id, so a second
    // run would double-count. Clear the stats first, and keep this test's ids to itself.
    let clean = |pool: PgPool| async move {
        sqlx::query("DELETE FROM match_player_stats WHERE arma_id = $1")
            .bind(ARMA)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM matches WHERE source_match_id IN ($1, '', '   ')")
            .bind(SRC)
            .execute(&pool)
            .await
            .unwrap();
    };
    clean(pool.clone()).await;

    // A body that is honest in every respect except the id.
    let body = |src: &str| {
        format!(
            r#"{{"match":{{"source_match_id":"{src}","terrain":"everon","outcome":"success","winning_faction":"USA","ended_at":"2026-07-26T20:14:00Z","aar_replay_url":"https://aar.tbd/{SRC}.json"}},"players":[{{"arma_id":"{ARMA}","role_played":"SL","source_event_id":"{EV}","counters":{{"kills":17,"deaths":3,"team_kills":1,"longest_kill_m":842,"vehicles_destroyed":4,"is_command":true,"command_win":true}}}}]}}"#
        )
    };
    let post = |b: String| {
        let app = app.clone();
        async move {
            call(
                &app,
                "POST",
                "/api/v1/ingest/match-results",
                None,
                Some(SVC),
                Some(&b),
            )
            .await
        }
    };

    // Counts *blank-or-absent* source ids rather than `count(*) FROM matches` (T-229). A bare
    // global count is a cross-test assertion in a suite whose tests run in parallel: any other
    // test in this binary creating a legitimate match between the two reads fails this one, which
    // is what happened the moment T-229's test was added — 1 vs 2, reported as "no match row
    // minted", naming neither the cause nor the test that caused it.
    //
    // The predicate is the invariant itself rather than a narrower scope, so nothing is given up:
    // it is exactly "a blank id reached the table", which is what the three rejected POSTs below
    // would have done (`''`, `'   '`, and a real tab/newline — JSON decodes the `\t\n` escapes, so
    // matching them as literals would need `E'…'` and quietly match nothing). `IS NULL` covers the
    // normalize-blank-to-`None` variant that was considered and rejected. It is stable because
    // `telemetry.rs` is the only test binary that POSTs match-results and the only other direct
    // `INSERT INTO matches` in the suite (`null_tolerance.rs:558`) uses `'src-1'`.
    let blank_id_rows = |pool: PgPool| async move {
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM matches \
             WHERE source_match_id IS NULL OR btrim(source_match_id) = ''",
        )
        .fetch_one(&pool)
        .await
        .unwrap()
    };
    let matches_before = blank_id_rows(pool.clone()).await;

    // (1) Whitespace — the value that used to become a live dedupe key on a 200.
    let (st, r) = post(body("   ")).await;
    assert_eq!(st, StatusCode::BAD_REQUEST, "whitespace id: {r}");
    assert_eq!(
        r["error"],
        "source_match_id must not be blank (omit it for a match with no source id)"
    );

    // (2) `""` — the value that used to be inserted once and then 500 forever. Twice, because
    // pre-fix the *first* call was a 200 that poisoned the table for every call after it.
    for attempt in 1..=2 {
        let (st, r) = post(body("")).await;
        assert_eq!(
            st,
            StatusCode::BAD_REQUEST,
            "empty id, attempt {attempt}: {r}"
        );
    }
    let poisoned: i64 =
        sqlx::query_scalar("SELECT count(*) FROM matches WHERE source_match_id = ''")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(poisoned, 0, "no '' row can reach the unique index");

    // (3) Whitespace is not only spaces.
    let (st, r) = post(body(r"\t\n ")).await;
    assert_eq!(st, StatusCode::BAD_REQUEST, "tab/newline id: {r}");

    // Nothing above wrote anything at all — not a match, not a stat row, not a counter.
    let matches_after = blank_id_rows(pool.clone()).await;
    assert_eq!(
        matches_before, matches_after,
        "no match row minted for a blank id"
    );
    let stats: i64 =
        sqlx::query_scalar("SELECT count(*) FROM match_player_stats WHERE arma_id = $1")
            .bind(ARMA)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(stats, 0, "no stat row written");

    // (4) A real id still works, and (5) the padded form of it resolves to the SAME match rather
    // than a second row — the lookup and the bind agree on one normalized key.
    let (st, first) = post(body(SRC)).await;
    assert_eq!(st, StatusCode::OK, "real id: {first}");
    let (st, padded) = post(body(&format!("  {SRC}  "))).await;
    assert_eq!(st, StatusCode::OK, "padded id: {padded}");
    assert_eq!(
        padded["match_id"], first["match_id"],
        "a padded source_match_id is the same match, not a new one"
    );
    let rows: i64 = sqlx::query_scalar("SELECT count(*) FROM matches WHERE source_match_id = $1")
        .bind(SRC)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(rows, 1, "one match row, stored trimmed");

    clean(pool.clone()).await;
}

/// T-369 — a *corrected* re-POST must land, and attendance must follow it.
///
/// `upsert_match`'s re-ingest branch omitted `event_id`, `mission_id`, `terrain` and
/// `started_at` from the UPDATE and returned the *stored* `event_id`, so a first POST that
/// carried a `source_match_id` but no `event_id`, followed by a corrected re-POST carrying the
/// right one, marked nobody's attendance — forever — on two 200s. That is the opposite of what
/// T-316 decided for the sibling fields (`ended_at` / `winning_faction` / `aar_replay_url`,
/// where a *present* field wins), so all seven now read the same way.
///
/// The three POSTs below are the whole argument: create without the event, correct it, then
/// retry partially. Keep them under the strict limiter's burst (1/s, burst 10).
#[tokio::test]
async fn a_corrected_reingest_lands_the_event_and_marks_attendance() {
    let Some((app, pool)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    const ARMA: &str = "t369-arma-correct";
    const DISCORD: &str = "000000000000369001";
    const SRC: &str = "m-t369-correct";
    const EV: &str = "e-t369";
    const STARTED: &str = "2026-07-26T18:00:00Z";

    sqlx::query(
        "INSERT INTO users (discord_id, username, discord_handle, avatar_url, arma_id, arma_character, role, is_banned, ban_reason, created_at, updated_at) \
         VALUES ($1, 'T369', 't369', '', $2, '[TBD] T369', 'enlisted', false, '', now(), now()) \
         ON CONFLICT (discord_id) DO UPDATE SET arma_id = EXCLUDED.arma_id",
    )
    .bind(DISCORD)
    .bind(ARMA)
    .execute(&pool)
    .await
    .unwrap();
    // Same reasoning as the T-316 / T-347 tests: `matches` does not cascade to
    // `match_player_stats`, and `leaderboard_totals` sums every row for a discord_id, so a
    // second run would double-count. Clear the stats first and keep this test's ids to itself.
    let clean = |pool: PgPool| async move {
        sqlx::query("DELETE FROM match_player_stats WHERE arma_id = $1")
            .bind(ARMA)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM matches WHERE source_match_id = $1")
            .bind(SRC)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM event_registrations WHERE discord_id = $1")
            .bind(DISCORD)
            .execute(&pool)
            .await
            .unwrap();
    };
    clean(pool.clone()).await;

    // A scheduled op the player is registered for. `start_time` is in the past so
    // `recompute_user_stats`' `past_registered` denominator is non-zero and `attendance_rate`
    // is actually measurable rather than the 0.0 fallback.
    let mission_id: Uuid = sqlx::query_scalar(
        "INSERT INTO missions (title, author_id, terrain, game_mode, max_players, status, created_at, updated_at) \
         VALUES ('T369 Op', $1, 'everon', 'pve_coop', 32, 'live', now(), now()) RETURNING id",
    )
    .bind(DISCORD)
    .fetch_one(&pool)
    .await
    .unwrap();
    let event_id: Uuid = sqlx::query_scalar(
        "INSERT INTO events (name_override, start_time, status, created_by, created_at, updated_at) \
         VALUES ('T369 Event', now() - interval '2 hours', 'open', $1, now(), now()) RETURNING id",
    )
    .bind(DISCORD)
    .fetch_one(&pool)
    .await
    .unwrap();
    let event_mission_id: Uuid = sqlx::query_scalar(
        "INSERT INTO event_missions (event_id, mission_id, start_time, created_at, updated_at) \
         VALUES ($1, $2, now() - interval '2 hours', now(), now()) RETURNING id",
    )
    .bind(event_id)
    .bind(mission_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO event_registrations (event_mission_id, discord_id, state) VALUES ($1, $2, 'registered')",
    )
    .bind(event_mission_id)
    .bind(DISCORD)
    .execute(&pool)
    .await
    .unwrap();

    let post = |b: String| {
        let app = app.clone();
        async move {
            call(
                &app,
                "POST",
                "/api/v1/ingest/match-results",
                None,
                Some(SVC),
                Some(&b),
            )
            .await
        }
    };
    let players = format!(
        r#""players":[{{"arma_id":"{ARMA}","role_played":"SL","source_event_id":"{EV}","counters":{{"kills":17,"deaths":3,"team_kills":1,"longest_kill_m":842,"vehicles_destroyed":4,"is_command":true,"command_win":true}}}}]"#
    );
    // The four fields the UPDATE used to drop, read back as they are actually stored.
    type Provenance = (Option<Uuid>, Option<Uuid>, Option<String>, String);
    let read_match = |pool: PgPool| async move {
        sqlx::query_as::<_, Provenance>(
            "SELECT event_id, mission_id, terrain::text, \
             to_char(started_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') \
             FROM matches WHERE source_match_id = $1",
        )
        .bind(SRC)
        .fetch_one(&pool)
        .await
        .unwrap()
    };
    let read_state = |pool: PgPool| async move {
        sqlx::query_scalar::<_, String>(
            "SELECT state::text FROM event_registrations WHERE discord_id = $1",
        )
        .bind(DISCORD)
        .fetch_one(&pool)
        .await
        .unwrap()
    };
    let read_user = |pool: PgPool| async move {
        sqlx::query_as::<_, (i64, f64)>(
            "SELECT total_deployments, attendance_rate::float8 FROM users WHERE discord_id = $1",
        )
        .bind(DISCORD)
        .fetch_one(&pool)
        .await
        .unwrap()
    };

    // (1) The first POST: a real dedupe key, but the sender forgot the event entirely. This is
    // an honest 200 — there is no event to attribute the op to yet.
    let (st, first) = post(format!(
        r#"{{"match":{{"source_match_id":"{SRC}","outcome":"pending"}},{players}}}"#
    ))
    .await;
    assert_eq!(st, StatusCode::OK, "first ingest: {first}");
    let created = read_match(pool.clone()).await;
    assert_eq!(
        (created.0, created.1, created.2.clone()),
        (None, None, None),
        "no event/mission/terrain on the first POST"
    );
    assert_eq!(
        read_state(pool.clone()).await,
        "registered",
        "nothing to attribute yet"
    );

    // (2) The correction: same key, now carrying the event, the mission, the terrain and the
    // real start time. Pre-fix this returned 200 and changed none of them, so
    // `if let Some(eid) = event_id` saw `None` and the attendance UPDATE never ran.
    let (st, corrected) = post(format!(
        r#"{{"match":{{"source_match_id":"{SRC}","outcome":"success","winning_faction":"USA","event_id":"{event_id}","mission_id":"{mission_id}","terrain":"everon","started_at":"{STARTED}","ended_at":"2026-07-26T20:14:00Z"}},{players}}}"#
    ))
    .await;
    assert_eq!(st, StatusCode::OK, "corrected ingest: {corrected}");
    assert_eq!(
        corrected["match_id"], first["match_id"],
        "still the same match — the dedupe key did its job"
    );
    let after = read_match(pool.clone()).await;
    assert_eq!(
        after,
        (
            Some(event_id),
            Some(mission_id),
            Some("everon".to_string()),
            STARTED.to_string()
        ),
        "the corrected provenance landed"
    );
    assert_eq!(
        read_state(pool.clone()).await,
        "attended",
        "THE TICKET: the corrected event_id must reach the attendance UPDATE"
    );
    // A row set but derived numbers left short would be half a fix: `recompute_user_stats`
    // runs after the commit and re-counts `state = 'attended'` over past registrations.
    assert_eq!(
        read_user(pool.clone()).await,
        (1, 100.0),
        "attendance_rate follows the corrected row"
    );

    // (3) T-316's direction, unbroken: a partial retry must not null any of the four back out,
    // and must not stamp `started_at` with `now()` — the create path's `unwrap_or_else(Utc::now)`
    // must never reach the UPDATE.
    let (st, retry) = post(format!(
        r#"{{"match":{{"source_match_id":"{SRC}","outcome":"success"}},{players}}}"#
    ))
    .await;
    assert_eq!(st, StatusCode::OK, "partial retry: {retry}");
    assert_eq!(
        read_match(pool.clone()).await,
        after,
        "an omitted field still keeps the stored value (T-316)"
    );
    assert_eq!(read_state(pool.clone()).await, "attended", "still attended");

    clean(pool.clone()).await;
}

/// T-229 — a player whose `arma_id` resolves to no account must keep their row, and the 200 must
/// stop implying the whole roster landed.
///
/// The invisibility, measured on a throwaway database before the fix: one POST carrying
/// `kills=17 deaths=3 longest_kill_m=842 vehicles_destroyed=4` for an unlinked `arma_id` returned
/// `{"match_id":"4dc322a3-…","players":1}`, wrote the row with `discord_id` NULL, and left
/// `leaderboard_totals` with **zero** rows for that player (it filters
/// `WHERE discord_id IS NOT NULL`) and `users.total_deployments` at **0**. Nothing anywhere
/// recorded that a scoreline had gone missing — the count in the response was the *submitted*
/// count and said nothing about how much of it was countable.
///
/// The row is kept rather than rejected, and that is the decision this test pins. It is real
/// telemetry — the `arma_id` is real and the match happened — and it is *recoverable*, because
/// `ingest_link_confirm` claims exactly the `discord_id IS NULL` rows at link time (T-326). A 400
/// would also have no per-player shape: the transaction is atomic, so one unresolved player would
/// reject the whole op. And the shipping mod implements no link flow at all
/// (`TBD_ResultsReporter.c:23-35`, T-181.35), so an unresolved `arma_id` is currently *every*
/// player in *every* production match — which is why the last leg below, a roster with nobody
/// linked, has to be a 200.
///
/// **The backfill half of T-229 as filed is already closed by T-326**, and the link leg asserts it
/// from this side on purpose: the ticket's premise was that "the upsert key includes `arma_id`, [so]
/// linking later does not backfill", and the key is in fact exactly what lets the backfill find the
/// row again. Pinning it here means a regression in `handlers::me` fails the suite that owns the
/// ingest contract depending on it.
///
/// Two ingest calls only — the strict limiter is keyed on the peer IP, which is `0.0.0.0` for every
/// test in this binary, so the whole file shares one 1/s + burst-10 bucket. The roster is built to
/// prove everything in one POST, including that `unlinked_arma_ids` is *distinct* while `linked` and
/// `unlinked` count player *lines*.
#[tokio::test]
async fn an_unresolvable_arma_id_keeps_its_row_and_the_response_says_so() {
    let Some((app, pool)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    // Three identities: one linked account, one account that has not linked yet (the ticket's
    // player), and an `arma_id` with no account behind it at all.
    const LINKED_ARMA: &str = "t229-arma-linked";
    const LINKED_DISCORD: &str = "000000000000229001";
    const UNLINKED_ARMA: &str = "t229-arma-unlinked";
    const UNLINKED_DISCORD: &str = "000000000000229002";
    const ORPHAN_ARMA: &str = "t229-arma-no-account";
    const SRC: &str = "m-t229-unlinked";
    const CODE: &str = "922900";

    // Same reasoning as the T-316 / T-347 / T-369 tests: `matches` does not cascade to
    // `match_player_stats` and `leaderboard_totals` sums every row for a discord_id, so a second
    // run would double-count. Clear the stats first and keep this test's ids to itself.
    //
    // The `UPDATE … SET arma_id = NULL` is not tidiness. `users.arma_id` carries a UNIQUE index
    // (`idx_users_arma_id`), so if any *other* account is holding one of these three ids the
    // fixture insert dies on `23505` and the link-confirm leg would 409 on `ingest_link_confirm`'s
    // clash guard. Hit for real while writing this: a manual probe on the same database had
    // parked `t229-arma-linked` on a third account. Releasing first makes the test own its ids
    // outright instead of hoping they are free.
    let clean = |pool: PgPool| async move {
        sqlx::query("UPDATE users SET arma_id = NULL WHERE arma_id = ANY($1)")
            .bind(vec![
                LINKED_ARMA.to_string(),
                UNLINKED_ARMA.to_string(),
                ORPHAN_ARMA.to_string(),
            ])
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM match_player_stats WHERE arma_id = ANY($1)")
            .bind(vec![
                LINKED_ARMA.to_string(),
                UNLINKED_ARMA.to_string(),
                ORPHAN_ARMA.to_string(),
            ])
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM matches WHERE source_match_id = $1")
            .bind(SRC)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM identity_link_codes WHERE discord_id = $1")
            .bind(UNLINKED_DISCORD)
            .execute(&pool)
            .await
            .unwrap();
    };
    clean(pool.clone()).await;

    sqlx::query(
        "INSERT INTO users (discord_id, username, discord_handle, avatar_url, arma_id, arma_character, role, is_banned, ban_reason, created_at, updated_at) \
         VALUES ($1, 'T229 Linked', 't229linked', '', $2, '[TBD] Linked', 'enlisted', false, '', now(), now()) \
         ON CONFLICT (discord_id) DO UPDATE SET arma_id = EXCLUDED.arma_id",
    )
    .bind(LINKED_DISCORD)
    .bind(LINKED_ARMA)
    .execute(&pool)
    .await
    .unwrap();
    // The ticket's player: a real account with no `arma_id` yet. `arma_id = NULL` is the whole
    // premise, so it is reset on conflict rather than left at whatever a previous run linked.
    sqlx::query(
        "INSERT INTO users (discord_id, username, discord_handle, avatar_url, arma_id, arma_character, role, is_banned, ban_reason, created_at, updated_at) \
         VALUES ($1, 'T229 Unlinked', 't229unlinked', '', NULL, '', 'enlisted', false, '', now(), now()) \
         ON CONFLICT (discord_id) DO UPDATE SET arma_id = NULL, total_deployments = 0",
    )
    .bind(UNLINKED_DISCORD)
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO identity_link_codes (code, discord_id, expires_at, created_at) \
         VALUES ($1, $2, now() + interval '1 hour', now()) \
         ON CONFLICT (code) DO UPDATE SET discord_id = EXCLUDED.discord_id, \
          expires_at = EXCLUDED.expires_at, consumed_at = NULL",
    )
    .bind(CODE)
    .bind(UNLINKED_DISCORD)
    .execute(&pool)
    .await
    .unwrap();

    let post = |uri: &'static str, b: String| {
        let app = app.clone();
        async move { call(&app, "POST", uri, None, Some(SVC), Some(&b)).await }
    };
    let line = |arma: &str, ev: &str, kills: i64, deaths: i64, longest: i64, veh: i64| {
        format!(
            r#"{{"arma_id":"{arma}","role_played":"SL","source_event_id":"{ev}","counters":{{"kills":{kills},"deaths":{deaths},"team_kills":0,"longest_kill_m":{longest},"vehicles_destroyed":{veh},"is_command":false}}}}"#
        )
    };
    // `leaderboard_totals` is a materialized view refreshed in-request by every ingest, including
    // the ones concurrent tests in this binary are running. Refresh explicitly before reading it
    // so an absence assertion cannot pass (or fail) on somebody else's timing — same reason the
    // T-316 test does.
    let mv_row = |pool: PgPool, discord: &'static str| async move {
        sqlx::query("REFRESH MATERIALIZED VIEW CONCURRENTLY leaderboard_totals")
            .execute(&pool)
            .await
            .ok();
        sqlx::query_as::<_, (i64, i64, i64, i64, i64)>(
            "SELECT kills::int8, deaths::int8, longest_kill_m::int8, vehicles_destroyed::int8, \
             missions_played::int8 FROM leaderboard_totals WHERE discord_id = $1",
        )
        .bind(discord)
        .fetch_optional(&pool)
        .await
        .unwrap()
    };
    let deployments = |pool: PgPool, discord: &'static str| async move {
        sqlx::query_scalar::<_, i64>("SELECT total_deployments FROM users WHERE discord_id = $1")
            .bind(discord)
            .fetch_one(&pool)
            .await
            .unwrap()
    };

    // (1) One match, four player lines: the linked player, the unlinked player TWICE under two
    // different `source_event_id`s (two legitimate rows for one person — the dedupe key is
    // `(match_id, arma_id, source_event_id)`), and an `arma_id` nobody owns.
    let (st, r) = post(
        "/api/v1/ingest/match-results",
        format!(
            r#"{{"match":{{"source_match_id":"{SRC}","outcome":"success","winning_faction":"USA"}},"players":[{},{},{},{}]}}"#,
            line(LINKED_ARMA, "e-t229-a", 9, 1, 300, 0),
            line(UNLINKED_ARMA, "e-t229-a", 17, 3, 842, 4),
            line(UNLINKED_ARMA, "e-t229-b", 5, 1, 300, 1),
            line(ORPHAN_ARMA, "e-t229-a", 4, 2, 120, 0),
        ),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "ingest with unresolved players: {r}");
    let match_id = r["match_id"].as_str().unwrap().to_string();

    // THE TICKET: the response no longer reports only the submitted count.
    assert_eq!(r["players"], 4, "still the submitted count, unchanged");
    assert_eq!(r["linked"], 1);
    assert_eq!(r["unlinked"], 3, "player LINES with no owner");
    assert_eq!(
        r["linked"].as_i64().unwrap() + r["unlinked"].as_i64().unwrap(),
        r["players"].as_i64().unwrap(),
        "linked + unlinked == players, always"
    );
    // Distinct ids, not lines — the unlinked player appears on two lines and once in this list.
    assert_eq!(
        r["unlinked_arma_ids"],
        serde_json::json!([UNLINKED_ARMA, ORPHAN_ARMA]),
        "distinct arma_ids in first-seen order"
    );

    // (2) Nothing was rejected and nothing was dropped: all four rows are stored with their real
    // counters, three of them simply unowned.
    let rows: Vec<(String, Option<String>, i64, i64)> = sqlx::query_as(
        "SELECT arma_id, discord_id, kills, deaths FROM match_player_stats \
         WHERE match_id = $1 ORDER BY arma_id, source_event_id",
    )
    .bind(Uuid::parse_str(&match_id).unwrap())
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        rows,
        vec![
            (LINKED_ARMA.into(), Some(LINKED_DISCORD.into()), 9, 1),
            (ORPHAN_ARMA.into(), None, 4, 2),
            (UNLINKED_ARMA.into(), None, 17, 3),
            (UNLINKED_ARMA.into(), None, 5, 1),
        ],
        "every line stored; the unresolved ones kept, with a NULL owner"
    );

    // (3) The invisibility itself, which is what the ticket is about: those rows reach no
    // aggregate. Not a bug in the aggregates — a leaderboard ranks accounts, and an unowned row
    // has no account — but it is why a 200 that says nothing is a silent loss.
    assert_eq!(
        mv_row(pool.clone(), UNLINKED_DISCORD).await,
        None,
        "22 real kills are invisible to leaderboard_totals while unowned"
    );
    assert_eq!(
        deployments(pool.clone(), UNLINKED_DISCORD).await,
        0,
        "and to the deployment count"
    );
    assert!(
        mv_row(pool.clone(), LINKED_DISCORD).await.is_some(),
        "the linked player on the same roster is counted normally"
    );

    // (4) The loss is now discoverable by an operator, not only by whoever reads the game
    // server's console. Info rather than Warn on purpose: with no link flow in the shipping mod
    // this fires on every production ingest, and an always-on warning is the false
    // `server.low_fps` WARN that T-316 was filed to delete.
    let audit: (String, String, Option<String>) = sqlx::query_as(
        "SELECT severity::text, message, actor_id FROM audit_logs \
         WHERE action = 'match.unlinked_players' AND target_type = 'match' AND target_id = $1",
    )
    .bind(&match_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(audit.0, "info", "a normal, self-healing state, not a fault");
    assert_eq!(audit.2, None, "system-originated, no human actor");
    assert!(
        audit.1.contains("3 of 4 player line(s)")
            && audit.1.contains(UNLINKED_ARMA)
            && audit.1.contains(ORPHAN_ARMA),
        "the audit row names the count AND the ids, or it is unactionable: {}",
        audit.1
    );

    // (5) The backfill half of the ticket, already built by T-326: linking claims the historical
    // rows, and both aggregates catch up. Note the sums — 17+5 kills and 3+1 deaths across the
    // two lines, one distinct match — so this proves the rows were claimed, not re-ingested.
    let (st, r) = post(
        "/api/v1/ingest/link-confirm",
        format!(
            r#"{{"code":"{CODE}","arma_id":"{UNLINKED_ARMA}","arma_character":"[TBD] Unlinked"}}"#
        ),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "link confirm: {r}");
    assert_eq!(r["linked"], true);
    assert_eq!(
        mv_row(pool.clone(), UNLINKED_DISCORD).await,
        Some((22, 4, 842, 5, 1)),
        "T-326 backfill: the parked rows reach the leaderboard on link"
    );
    assert_eq!(
        deployments(pool.clone(), UNLINKED_DISCORD).await,
        1,
        "and the deployment count"
    );
    // The other unresolved line is untouched — the backfill claims one `arma_id`, not the roster.
    let still_orphan: Option<String> = sqlx::query_scalar(
        "SELECT discord_id FROM match_player_stats WHERE arma_id = $1 AND match_id = $2",
    )
    .bind(ORPHAN_ARMA)
    .bind(Uuid::parse_str(&match_id).unwrap())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(still_orphan, None, "a different arma_id stays unowned");

    sqlx::query(
        "DELETE FROM audit_logs WHERE action = 'match.unlinked_players' AND target_id = $1",
    )
    .bind(&match_id)
    .execute(&pool)
    .await
    .unwrap();
    clean(pool.clone()).await;
}

/// **The payload the shipping mod actually sends, reproduced byte-for-byte, asserted to be
/// accepted (T-393).**
///
/// This fixture is the whole point of the ticket. T-316 made nine player fields required
/// without checking the only client, and every test in this file built its own complete body,
/// so nothing in the suite ever looked at what the game server puts on the wire. The result
/// was arithmetic, not bad luck: the mod sends four keys, serde rejected on the first of the
/// five it did not send, and **every match report from every production server 400'd** —
/// match rows, per-player stats, attendance, user-stat recompute and leaderboard refresh, all
/// dead on arrival, for as long as T-316 was deployed.
///
/// # Source of truth for these bytes
///
/// `apps/mod/tbd-framework/Scripts/Game/TBD/Backend/TBD_ResultsReporter.c` — the envelope from
/// `BuildPayload` (key order and all), each row from `BuildPlayerRow`. Both hand-build JSON by
/// string concatenation, so there is no serializer that could quietly fill a field in: what
/// those two functions write is exactly what the backend receives. Reproduced here in that
/// same order so a reader can diff the two by eye.
///
/// **If you change the wire contract, this test is the tripwire.** It fails here, in CI, on a
/// laptop, instead of on a dedicated server at 20:00 on an op night with the only symptom
/// being a `400` in a console nobody is reading. Re-derive the literal from those two
/// functions rather than editing it to match the new struct — editing it to pass is exactly
/// the check that was missing.
///
/// The field values are shaped like production: `source_match_id` is `BuildSourceMatchId`'s
/// `missionId@startedAt#tick`, `started_at`/`ended_at` are `UtcNowIso8601`'s fixed-width
/// RFC 3339, and `source_event_id` is the *same* string as the match's `event_id` because
/// `CollectPlayers` and `BuildPayload` both read `TBD_BackendConfig.GetEventId()`.
#[tokio::test]
async fn the_shipping_mod_payload_is_accepted_verbatim() {
    let Some((app, pool)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    // Both arma_ids stay unlinked on purpose: `TBD_ResultsReporter.c:23-35` says no player has
    // an `arma_id` in production until T-181.35 ships the link flow, so this is the real
    // population, and the T-229 unlinked-reporting path is on the same code path as the fix.
    const A1: &str = "t393-arma-mod-a";
    const A2: &str = "t393-arma-mod-b";
    const EV: &str = "9f0f4c6e-1d3a-4e2b-8c77-2a5b6d4e9011";
    const MISSION: &str = "3c1d5b7a-8e42-4f19-9a6d-71b0c2e8f455";
    const SRC: &str = "3c1d5b7a-8e42-4f19-9a6d-71b0c2e8f455@2026-07-26T20:03:11Z#183472";
    /// Author of the two rows below. Private to this test (`…393002`) so no other suite's
    /// `DELETE FROM missions WHERE author_id = $1` sweep can take them out from under it.
    const AUTHOR: &str = "000000000000393002";

    let clean = |pool: PgPool| async move {
        sqlx::query("DELETE FROM match_player_stats WHERE arma_id = ANY($1)")
            .bind(vec![A1.to_string(), A2.to_string()])
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM matches WHERE source_match_id = $1")
            .bind(SRC)
            .execute(&pool)
            .await
            .unwrap();
    };
    clean(pool.clone()).await;

    // T-585 — `0019_ingest_pointer_foreign_keys.sql` constrains `matches.event_id` and
    // `matches.mission_id`, so the two ids the mod sends must now name rows that exist. Before
    // it, this test posted pointers to nothing, got a 200, and stored a match whose `event_id`
    // dangled — the exact silent-attribution-loss 0019 was written to end.
    //
    // **THE PAYLOAD BELOW IS UNCHANGED, DELIBERATELY.** The fix is to make its parents real, not
    // to re-aim the literal: the literal IS the contract this test guards, and editing it to
    // pass is precisely the move the doc comment above forbids. Both rows are therefore pinned
    // to `EV` / `MISSION` rather than taking `RETURNING id`, which also keeps `SRC` honest —
    // it is `BuildSourceMatchId`'s `missionId@startedAt#tick`, so its leading uuid has to stay
    // the mission's.
    //
    // `ON CONFLICT DO NOTHING`: each test binary provisions its own database, but a re-run
    // against a surviving one must converge rather than collide on the pinned primary key.
    // The rows are left behind on purpose — a parent that outlives the match is the normal
    // shape, and deleting them here would exercise 0019's `ON DELETE SET NULL` instead of the
    // ingest path this test is about.
    //
    // No `users` row is minted for `AUTHOR`: `missions.author_id` / `events.created_by` carry no
    // foreign key (0018 abstention (ii) — authorship is an open policy decision), and the two
    // `arma_id`s must stay unlinked for the `unlinked: 2` assertion below. If an authorship FK
    // ever lands, this is one of the call sites that needs a real user.
    sqlx::query(
        "INSERT INTO missions (id, title, author_id, terrain, game_mode, max_players, status, created_at, updated_at) \
         VALUES ($1, 'T393 Shipping Mod Mission', $2, 'everon', 'pve_coop', 32, 'live', now(), now()) \
         ON CONFLICT (id) DO NOTHING",
    )
    .bind(Uuid::parse_str(MISSION).expect("MISSION is a uuid literal"))
    .bind(AUTHOR)
    .execute(&pool)
    .await
    .expect("seed the mission the shipping payload names");
    sqlx::query(
        "INSERT INTO events (id, name_override, start_time, status, created_by, created_at, updated_at) \
         VALUES ($1, 'T393 Shipping Mod Event', now() - interval '2 hours', 'open', $2, now(), now()) \
         ON CONFLICT (id) DO NOTHING",
    )
    .bind(Uuid::parse_str(EV).expect("EV is a uuid literal"))
    .bind(AUTHOR)
    .execute(&pool)
    .await
    .expect("seed the event the shipping payload names");

    // ---- BEGIN golden payload — TBD_ResultsReporter.c BuildPayload + BuildPlayerRow ----
    let golden = format!(
        r#"{{"match":{{"source_match_id":"{SRC}","event_id":"{EV}","mission_id":"{MISSION}","terrain":"everon","started_at":"2026-07-26T20:03:11Z","ended_at":"2026-07-26T21:14:02Z","outcome":"success","winning_faction":"USA"}},"players":[{{"arma_id":"{A1}","role_played":"Squad Leader","deaths":1,"source_event_id":"{EV}"}},{{"arma_id":"{A2}","role_played":"Rifleman","deaths":0,"source_event_id":"{EV}"}}]}}"#
    );
    // ---- END golden payload ----

    let (st, r) = call(
        &app,
        "POST",
        "/api/v1/ingest/match-results",
        None,
        Some(SVC),
        Some(&golden),
    )
    .await;
    assert_eq!(
        st,
        StatusCode::OK,
        "the shipping mod payload must be accepted: {r}"
    );
    assert_eq!(r["players"], 2);
    assert_eq!(r["unlinked"], 2, "nobody is linked in production yet");
    let match_id = Uuid::parse_str(r["match_id"].as_str().unwrap()).unwrap();

    // The match half landed in full — this is the half that was never in doubt, and the point
    // of asserting it is that it was ALSO lost, because the 400 rejected the whole request.
    let m: (String, Option<String>, Option<String>, Option<Uuid>) = sqlx::query_as(
        "SELECT outcome::text, winning_faction, terrain::text, event_id FROM matches WHERE id = $1",
    )
    .bind(match_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        m,
        (
            "success".into(),
            Some("USA".into()),
            Some("everon".into()),
            Some(Uuid::parse_str(EV).unwrap())
        )
    );

    // Both player rows exist with their identity core intact, and — the T-393/T-397 contract —
    // with no counters written, because the payload states none. T-397 made the columns
    // NULLable: a fresh insert stores NULL ("not measured"), not the old DDL `DEFAULT 0`.
    // On a *re*-ingest these columns are not named in the UPDATE, which is what
    // `absent_counters_are_not_a_write_on_reingest` proves.
    type Row = (
        String,
        String,
        Option<i64>,
        Option<i64>,
        Option<i64>,
        Option<i64>,
        Option<i64>,
        Option<bool>,
        Option<bool>,
    );
    let rows: Vec<Row> = sqlx::query_as(
        "SELECT arma_id, role_played, kills, deaths, team_kills, longest_kill_m, \
         vehicles_destroyed, is_command, command_win FROM match_player_stats \
         WHERE match_id = $1 ORDER BY arma_id",
    )
    .bind(match_id)
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        rows,
        vec![
            (
                A1.into(),
                "Squad Leader".into(),
                None,
                None,
                None,
                None,
                None,
                None,
                None
            ),
            (
                A2.into(),
                "Rifleman".into(),
                None,
                None,
                None,
                None,
                None,
                None,
                None
            ),
        ],
        "identity + role stored; no counter claimed, so NULL not 0"
    );

    // **Named out loud because it is a real cost, not an oversight:** the mod's top-level
    // `"deaths":1` is an unknown field under the split contract and is ignored, so A1's stored
    // `deaths` is NULL above rather than 1. `deaths` sits inside the counters block because
    // `leaderboard_totals.kd_ratio` is `sum(kills)/sum(deaths)` — a `deaths` writable without
    // `kills` is a `kd_ratio` corruptible by a low-fidelity re-ingest. Recovering that one
    // number is a mod-side change (emit a complete `counters` object); it is not a reason to
    // let one counter travel alone. The alternative — rejecting a top-level `deaths` — would
    // 400 this very payload, which is the defect this test exists to prevent.
    assert!(
        rows[0].3.is_none(),
        "top-level deaths is ignored; absent counters store NULL (T-397)"
    );

    sqlx::query("DELETE FROM audit_logs WHERE target_id = $1")
        .bind(match_id.to_string())
        .execute(&pool)
        .await
        .unwrap();
    clean(pool.clone()).await;
}

/// **A counters block is all-or-nothing: a partial one is a 400 (T-393).**
///
/// The split makes the *block* optional; it does not make the fields inside it optional. That
/// distinction is the entire anti-corruption property. "No counters" is a legal statement
/// meaning "I make no claim" and writes nothing; "some counters" is a sender that has half a
/// scoreline and does not know it, and five silent zeros beside two real numbers is precisely
/// the corrupt row T-316 was filed to kill. There is no `#[serde(default)]` inside
/// `PlayerCountersInput`, so a missing key is a decode error, exactly as before.
#[tokio::test]
async fn a_partial_counters_object_is_still_a_400() {
    let Some((app, pool)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    const ARMA: &str = "t393-arma-partial";
    const SRC: &str = "m-t393-partial";
    const EV: &str = "e-t393-partial";

    let clean = |pool: PgPool| async move {
        sqlx::query("DELETE FROM match_player_stats WHERE arma_id = $1")
            .bind(ARMA)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM matches WHERE source_match_id = $1")
            .bind(SRC)
            .execute(&pool)
            .await
            .unwrap();
    };
    clean(pool.clone()).await;

    let body = |players: String| {
        format!(
            r#"{{"match":{{"source_match_id":"{SRC}","outcome":"success","winning_faction":"USA"}},"players":[{players}]}}"#
        )
    };
    let post = |app: Router, b: String| async move {
        call(
            &app,
            "POST",
            "/api/v1/ingest/match-results",
            None,
            Some(SVC),
            Some(&b),
        )
        .await
    };

    // Seed a real scoreline so every rejection below has something it could have destroyed.
    let (st, r) = post(
        app.clone(),
        body(format!(
            r#"{{"arma_id":"{ARMA}","role_played":"SL","source_event_id":"{EV}","counters":{{"kills":17,"deaths":3,"team_kills":1,"longest_kill_m":842,"vehicles_destroyed":4,"is_command":true,"command_win":true}}}}"#
        )),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "seed: {r}");

    let read = |pool: PgPool| async move {
        sqlx::query_as::<_, (i64, i64, i64, i64, i64, bool)>(
            "SELECT kills, deaths, team_kills, longest_kill_m, vehicles_destroyed, is_command \
             FROM match_player_stats WHERE arma_id = $1",
        )
        .bind(ARMA)
        .fetch_one(&pool)
        .await
        .unwrap()
    };
    const SEEDED: (i64, i64, i64, i64, i64, bool) = (17, 3, 1, 842, 4, true);
    assert_eq!(read(pool.clone()).await, SEEDED);

    // (1) One key present, the rest missing — the shape a half-built sender produces.
    let (st, r) = post(
        app.clone(),
        body(format!(
            r#"{{"arma_id":"{ARMA}","role_played":"SL","source_event_id":"{EV}","counters":{{"kills":9}}}}"#
        )),
    )
    .await;
    assert_eq!(st, StatusCode::BAD_REQUEST, "one-key counters: {r}");
    assert_eq!(read(pool.clone()).await, SEEDED, "nothing written");

    // (2) All but one — the shape a sender produces after adding a field to the DB and
    // forgetting the wire. `is_command` is not a "counter" in the arithmetic sense, which is
    // exactly why `GREATEST` was rejected in T-316 and why the block is all-or-nothing rather
    // than field-by-field.
    let (st, r) = post(
        app.clone(),
        body(format!(
            r#"{{"arma_id":"{ARMA}","role_played":"SL","source_event_id":"{EV}","counters":{{"kills":9,"deaths":2,"team_kills":0,"longest_kill_m":100,"vehicles_destroyed":1}}}}"#
        )),
    )
    .await;
    assert_eq!(
        st,
        StatusCode::BAD_REQUEST,
        "counters missing is_command: {r}"
    );
    assert_eq!(read(pool.clone()).await, SEEDED, "nothing written");

    // (3) The pre-T-393 flat body. Unknown fields are ignored by serde (and must be — denying
    // them would 400 the mod's extra `deaths` and re-open the defect), so without a tripwire
    // this would have been a cheerful 200 that stored nothing: the sender's counters dropped
    // on the floor, its 200 implying they landed. That is the T-316 failure mode in new
    // clothes — a silent loss where the sender believes it stated something — so the flat
    // shape is rejected by name instead.
    let (st, r) = post(
        app.clone(),
        body(format!(
            r#"{{"arma_id":"{ARMA}","role_played":"SL","source_event_id":"{EV}","kills":9,"deaths":2,"team_kills":0,"longest_kill_m":100,"vehicles_destroyed":1,"is_command":false}}"#
        )),
    )
    .await;
    assert_eq!(st, StatusCode::BAD_REQUEST, "legacy flat body: {r}");
    assert!(
        r["error"].as_str().unwrap_or_default().contains("counters"),
        "the error must name the shape to move to, since a game server's only channel back is \
         this string: {r}"
    );
    assert_eq!(read(pool.clone()).await, SEEDED, "nothing written");

    clean(pool.clone()).await;
}

/// **Absent counters are not a write — the T-316 property, restated (T-393).**
///
/// This is the assertion that matters most in the file. The contract split would be worthless
/// (and dangerous) if omitting the block merely meant "send zeros politely": the whole reason
/// counters were made required was that an omission used to overwrite a real scoreline with
/// `kills=0 … command_win=NULL`, which `refresh_leaderboard` then summed in the same request.
/// After the split an omission writes *nothing at all* — the counters-absent SQL statement does
/// not name those columns, so they are not read, not re-bound, and not rewritten.
///
/// The second POST deliberately changes `role_played`, and the test asserts that change landed.
/// Without it the whole thing would be vacuous: a request that silently failed, or a handler
/// that skipped the row entirely, would also leave the counters untouched and this test would
/// pass while proving nothing. The role change is the receipt that the upsert really ran and
/// really wrote this row, and touched every column it was entitled to touch and no others.
#[tokio::test]
async fn absent_counters_are_not_a_write_on_reingest() {
    let Some((app, pool)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    const ARMA: &str = "t393-arma-noclaim";
    const DISCORD: &str = "000000000000393001";
    const SRC: &str = "m-t393-noclaim";
    const EV: &str = "e-t393-noclaim";

    // Link the player so the leaderboard half of the property is observable too — an unowned
    // row never reaches `leaderboard_totals` (T-229), so an unlinked player could not show that
    // a zeroing would have propagated.
    sqlx::query(
        "INSERT INTO users (discord_id, username, discord_handle, avatar_url, arma_id, arma_character, role, is_banned, ban_reason, created_at, updated_at) \
         VALUES ($1, 'T393', 't393', '', $2, '[TBD] T393', 'enlisted', false, '', now(), now()) \
         ON CONFLICT (discord_id) DO UPDATE SET arma_id = EXCLUDED.arma_id",
    )
    .bind(DISCORD)
    .bind(ARMA)
    .execute(&pool)
    .await
    .unwrap();
    let clean = |pool: PgPool| async move {
        sqlx::query("DELETE FROM match_player_stats WHERE arma_id = $1")
            .bind(ARMA)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM matches WHERE source_match_id = $1")
            .bind(SRC)
            .execute(&pool)
            .await
            .unwrap();
    };
    clean(pool.clone()).await;

    let read = |pool: PgPool| async move {
        sqlx::query_as::<_, (String, i64, i64, i64, i64, i64, bool, Option<bool>)>(
            "SELECT role_played, kills, deaths, team_kills, longest_kill_m, vehicles_destroyed, \
             is_command, command_win FROM match_player_stats WHERE arma_id = $1",
        )
        .bind(ARMA)
        .fetch_one(&pool)
        .await
        .unwrap()
    };

    // (1) A full report: the scoreline is stated, so it is authoritative and lands whole.
    let (st, r) = call(
        &app,
        "POST",
        "/api/v1/ingest/match-results",
        None,
        Some(SVC),
        Some(&format!(
            r#"{{"match":{{"source_match_id":"{SRC}","outcome":"success","winning_faction":"USA"}},"players":[{{"arma_id":"{ARMA}","role_played":"SL","source_event_id":"{EV}","counters":{{"kills":17,"deaths":3,"team_kills":1,"longest_kill_m":842,"vehicles_destroyed":4,"is_command":true,"command_win":true}}}}]}}"#
        )),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "full report: {r}");
    assert_eq!(
        read(pool.clone()).await,
        ("SL".into(), 17, 3, 1, 842, 4, true, Some(true))
    );

    // (2) The same row re-ingested with NO counters — the shipping mod's shape — and with a
    // corrected role, so the write is provable. Under T-316 this body was a 400; before T-316
    // it was a silent zeroing. It is now neither: it states a role and says nothing about the
    // scoreline.
    let (st, r) = call(
        &app,
        "POST",
        "/api/v1/ingest/match-results",
        None,
        Some(SVC),
        Some(&format!(
            r#"{{"match":{{"source_match_id":"{SRC}","outcome":"success","winning_faction":"USA"}},"players":[{{"arma_id":"{ARMA}","role_played":"PL","source_event_id":"{EV}"}}]}}"#
        )),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "counters-less re-ingest: {r}");
    assert_eq!(
        read(pool.clone()).await,
        ("PL".into(), 17, 3, 1, 842, 4, true, Some(true)),
        "the role was replaced (so the upsert DID run on this row) and every counter survived \
         untouched — including `command_win`, whose NULL would have been the tri-state's own \
         silent loss"
    );

    // (3) And the propagation the ticket is actually about: `leaderboard_totals` sums
    // `match_player_stats`, so a zeroed row really would have reached the leaderboard inside
    // the same request via `refresh_leaderboard`.
    sqlx::query("REFRESH MATERIALIZED VIEW CONCURRENTLY leaderboard_totals")
        .execute(&pool)
        .await
        .ok();
    let lb_kills: Option<i64> =
        sqlx::query_scalar("SELECT kills::int8 FROM leaderboard_totals WHERE discord_id = $1")
            .bind(DISCORD)
            .fetch_optional(&pool)
            .await
            .unwrap();
    assert_eq!(
        lb_kills,
        Some(17),
        "the leaderboard still shows the real kills after a counters-less re-ingest"
    );

    clean(pool.clone()).await;
}

/// T-230 — attendance marks only the *played* event_mission, not every mission on the event.
///
/// Pre-fix the ingest ran:
///   UPDATE event_registrations SET state = 'attended'
///    WHERE event_mission_id IN (SELECT id FROM event_missions WHERE event_id = $1)
/// which flipped registrations on missions that were never played. `decorate_events` /
/// the dashboard count only `registered`/`waitlisted`, so the unplayed mission's roster
/// collapsed to zero the moment a sibling op completed.
///
/// RED: restore the `WHERE event_id = $1` subquery (drop the `matches` JOIN on
/// `mission_id`) and both registrations below become `attended` — this test fails.
#[tokio::test]
async fn attendance_marks_only_the_played_event_mission() {
    let Some((app, pool)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    const ARMA: &str = "t230-arma-scope";
    const DISCORD: &str = "000000000000230001";
    const SRC: &str = "m-t230-scope";
    const EV: &str = "e-t230";

    sqlx::query(
        "INSERT INTO users (discord_id, username, discord_handle, avatar_url, arma_id, arma_character, role, is_banned, ban_reason, created_at, updated_at) \
         VALUES ($1, 'T230', 't230', '', $2, '[TBD] T230', 'enlisted', false, '', now(), now()) \
         ON CONFLICT (discord_id) DO UPDATE SET arma_id = EXCLUDED.arma_id",
    )
    .bind(DISCORD)
    .bind(ARMA)
    .execute(&pool)
    .await
    .unwrap();

    let clean = |pool: PgPool| async move {
        sqlx::query("DELETE FROM match_player_stats WHERE arma_id = $1")
            .bind(ARMA)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM matches WHERE source_match_id = $1")
            .bind(SRC)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM event_registrations WHERE discord_id = $1")
            .bind(DISCORD)
            .execute(&pool)
            .await
            .unwrap();
    };
    clean(pool.clone()).await;

    let mission_played: Uuid = sqlx::query_scalar(
        "INSERT INTO missions (title, author_id, terrain, game_mode, max_players, status, created_at, updated_at) \
         VALUES ('T230 Played', $1, 'everon', 'pve_coop', 32, 'live', now(), now()) RETURNING id",
    )
    .bind(DISCORD)
    .fetch_one(&pool)
    .await
    .unwrap();
    let mission_other: Uuid = sqlx::query_scalar(
        "INSERT INTO missions (title, author_id, terrain, game_mode, max_players, status, created_at, updated_at) \
         VALUES ('T230 Other', $1, 'everon', 'pve_coop', 32, 'live', now(), now()) RETURNING id",
    )
    .bind(DISCORD)
    .fetch_one(&pool)
    .await
    .unwrap();
    let event_id: Uuid = sqlx::query_scalar(
        "INSERT INTO events (name_override, start_time, status, created_by, created_at, updated_at) \
         VALUES ('T230 Multi-mission', now() - interval '2 hours', 'open', $1, now(), now()) RETURNING id",
    )
    .bind(DISCORD)
    .fetch_one(&pool)
    .await
    .unwrap();
    let em_played: Uuid = sqlx::query_scalar(
        "INSERT INTO event_missions (event_id, mission_id, start_time, created_at, updated_at) \
         VALUES ($1, $2, now() - interval '2 hours', now(), now()) RETURNING id",
    )
    .bind(event_id)
    .bind(mission_played)
    .fetch_one(&pool)
    .await
    .unwrap();
    let em_other: Uuid = sqlx::query_scalar(
        "INSERT INTO event_missions (event_id, mission_id, start_time, created_at, updated_at) \
         VALUES ($1, $2, now() - interval '1 hour', now(), now()) RETURNING id",
    )
    .bind(event_id)
    .bind(mission_other)
    .fetch_one(&pool)
    .await
    .unwrap();
    for em in [em_played, em_other] {
        sqlx::query(
            "INSERT INTO event_registrations (event_mission_id, discord_id, state) \
             VALUES ($1, $2, 'registered')",
        )
        .bind(em)
        .bind(DISCORD)
        .execute(&pool)
        .await
        .unwrap();
    }

    let body = format!(
        r#"{{"match":{{"source_match_id":"{SRC}","outcome":"success","winning_faction":"USA","event_id":"{event_id}","mission_id":"{mission_played}","terrain":"everon","ended_at":"2026-07-26T20:14:00Z"}},"players":[{{"arma_id":"{ARMA}","role_played":"SL","source_event_id":"{EV}","counters":{{"kills":5,"deaths":1,"team_kills":0,"longest_kill_m":0,"vehicles_destroyed":0,"is_command":false}}}}]}}"#
    );
    let (st, r) = call(
        &app,
        "POST",
        "/api/v1/ingest/match-results",
        None,
        Some(SVC),
        Some(&body),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "ingest: {r}");

    let state_played: String = sqlx::query_scalar(
        "SELECT state::text FROM event_registrations WHERE event_mission_id = $1 AND discord_id = $2",
    )
    .bind(em_played)
    .bind(DISCORD)
    .fetch_one(&pool)
    .await
    .unwrap();
    let state_other: String = sqlx::query_scalar(
        "SELECT state::text FROM event_registrations WHERE event_mission_id = $1 AND discord_id = $2",
    )
    .bind(em_other)
    .bind(DISCORD)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        state_played, "attended",
        "THE TICKET: the played mission's registration must flip to attended"
    );
    assert_eq!(
        state_other, "registered",
        "THE TICKET / RED: the unplayed sibling must stay registered — pre-fix both were attended"
    );

    // The roster-collapse side effect: decorate/dashboard only count registered|waitlisted.
    // After a scoped mark, the unplayed mission still contributes one registered seat.
    let em_ids = vec![em_played, em_other];
    let still_registered: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM event_registrations \
         WHERE event_mission_id = ANY($1) AND state::text IN ('registered', 'waitlisted')",
    )
    .bind(&em_ids)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        still_registered, 1,
        "unplayed mission keeps the event's registered count non-zero"
    );

    // Event-only ingest (no mission_id) must not invent "mark every mission" either.
    const SRC2: &str = "m-t230-event-only";
    sqlx::query("DELETE FROM matches WHERE source_match_id = $1")
        .bind(SRC2)
        .execute(&pool)
        .await
        .unwrap();
    // Reset the played reg so a wrong all-event write would be visible on both rows.
    sqlx::query("UPDATE event_registrations SET state = 'registered' WHERE discord_id = $1")
        .bind(DISCORD)
        .execute(&pool)
        .await
        .unwrap();
    let body2 = format!(
        r#"{{"match":{{"source_match_id":"{SRC2}","outcome":"success","event_id":"{event_id}"}},"players":[{{"arma_id":"{ARMA}","role_played":"SL","source_event_id":"{EV}","counters":{{"kills":1,"deaths":0,"team_kills":0,"longest_kill_m":0,"vehicles_destroyed":0,"is_command":false}}}}]}}"#
    );
    let (st, r) = call(
        &app,
        "POST",
        "/api/v1/ingest/match-results",
        None,
        Some(SVC),
        Some(&body2),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "event-only ingest: {r}");
    let both: Vec<(Uuid, String)> = sqlx::query_as(
        "SELECT event_mission_id, state::text FROM event_registrations \
         WHERE discord_id = $1 ORDER BY event_mission_id",
    )
    .bind(DISCORD)
    .fetch_all(&pool)
    .await
    .unwrap();
    assert!(
        both.iter().all(|(_, s)| s == "registered"),
        "event-only ingest must not mark attendance without a mission_id; got {both:?}"
    );

    clean(pool.clone()).await;
    sqlx::query("DELETE FROM match_player_stats WHERE arma_id = $1")
        .bind(ARMA)
        .execute(&pool)
        .await
        .unwrap();
    let srcs = vec![SRC, SRC2];
    sqlx::query("DELETE FROM matches WHERE source_match_id = ANY($1)")
        .bind(&srcs)
        .execute(&pool)
        .await
        .unwrap();
}

/// T-397 — INSERT without counters stores NULL, not 0.
///
/// Pre-fix the counters-absent statement omitted the columns and DDL `DEFAULT 0` filled
/// them. A stored 0 was a scored 0. RED: restore the omit-columns INSERT (or re-add
/// `NOT NULL DEFAULT 0`) and `deaths` comes back `Some(0)` — this test fails.
#[tokio::test]
async fn insert_without_counters_stores_null_not_zero() {
    let Some((app, pool)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    const ARMA: &str = "t397-arma-null-insert";
    const DISCORD: &str = "000000000000397101";
    const SRC: &str = "m-t397-null-insert";
    const EV: &str = "e-t397-null-insert";

    sqlx::query(
        "INSERT INTO users (discord_id, username, discord_handle, avatar_url, arma_id, arma_character, role, is_banned, ban_reason, created_at, updated_at) \
         VALUES ($1, 'T397i', 't397i', '', $2, '[TBD] T397i', 'enlisted', false, '', now(), now()) \
         ON CONFLICT (discord_id) DO UPDATE SET arma_id = EXCLUDED.arma_id",
    )
    .bind(DISCORD)
    .bind(ARMA)
    .execute(&pool)
    .await
    .unwrap();
    let clean = |pool: PgPool| async move {
        sqlx::query("DELETE FROM match_player_stats WHERE arma_id = $1")
            .bind(ARMA)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM matches WHERE source_match_id = $1")
            .bind(SRC)
            .execute(&pool)
            .await
            .unwrap();
    };
    clean(pool.clone()).await;

    let (st, r) = call(
        &app,
        "POST",
        "/api/v1/ingest/match-results",
        None,
        Some(SVC),
        Some(&format!(
            r#"{{"match":{{"source_match_id":"{SRC}","outcome":"success","winning_faction":"USA"}},"players":[{{"arma_id":"{ARMA}","role_played":"SL","source_event_id":"{EV}"}}]}}"#
        )),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "identity-only insert: {r}");

    #[derive(Debug, sqlx::FromRow)]
    struct CountersNull {
        kills: Option<i64>,
        deaths: Option<i64>,
        team_kills: Option<i64>,
        longest_kill_m: Option<i64>,
        vehicles_destroyed: Option<i64>,
        is_command: Option<bool>,
        command_win: Option<bool>,
    }
    let row: CountersNull = sqlx::query_as(
        "SELECT kills, deaths, team_kills, longest_kill_m, vehicles_destroyed, is_command, command_win \
         FROM match_player_stats WHERE arma_id = $1",
    )
    .bind(ARMA)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(
        row.kills.is_none()
            && row.deaths.is_none()
            && row.team_kills.is_none()
            && row.longest_kill_m.is_none()
            && row.vehicles_destroyed.is_none()
            && row.is_command.is_none()
            && row.command_win.is_none(),
        "absent counters must store NULL on first insert, not DEFAULT 0; got {row:?}"
    );

    clean(pool.clone()).await;
}

/// T-397 — `leaderboard_totals` ignores NULL deaths; it must not invent a measured zero.
///
/// Match A: counters absent → NULL deaths. Match B: full report 17/3. SUM(deaths) must be 3
/// (NULL ignored), not 3+0. kd = 17/3 ≈ 5.67. RED: put `DEFAULT 0` back (or COALESCE NULL→0
/// inside the MV sum) and a phantom death-zero re-enters the aggregate story.
#[tokio::test]
async fn leaderboard_mv_does_not_invent_deaths_from_null() {
    let Some((app, pool)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    const ARMA: &str = "t397-arma-mv-null";
    const DISCORD: &str = "000000000000397102";
    const SRC_A: &str = "m-t397-mv-a";
    const SRC_B: &str = "m-t397-mv-b";
    const EV_A: &str = "e-t397-mv-a";
    const EV_B: &str = "e-t397-mv-b";

    sqlx::query(
        "INSERT INTO users (discord_id, username, discord_handle, avatar_url, arma_id, arma_character, role, is_banned, ban_reason, created_at, updated_at) \
         VALUES ($1, 'T397m', 't397m', '', $2, '[TBD] T397m', 'enlisted', false, '', now(), now()) \
         ON CONFLICT (discord_id) DO UPDATE SET arma_id = EXCLUDED.arma_id",
    )
    .bind(DISCORD)
    .bind(ARMA)
    .execute(&pool)
    .await
    .unwrap();
    let clean = |pool: PgPool| async move {
        sqlx::query("DELETE FROM match_player_stats WHERE arma_id = $1")
            .bind(ARMA)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM matches WHERE source_match_id = ANY($1)")
            .bind(vec![SRC_A.to_string(), SRC_B.to_string()])
            .execute(&pool)
            .await
            .unwrap();
    };
    clean(pool.clone()).await;

    let post = |app: Router, body: String| async move {
        call(
            &app,
            "POST",
            "/api/v1/ingest/match-results",
            None,
            Some(SVC),
            Some(&body),
        )
        .await
    };

    // Row A — mod path: identity only.
    let (st, r) = post(
        app.clone(),
        format!(
            r#"{{"match":{{"source_match_id":"{SRC_A}","outcome":"success","winning_faction":"USA"}},"players":[{{"arma_id":"{ARMA}","role_played":"SL","source_event_id":"{EV_A}"}}]}}"#
        ),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "row A: {r}");

    // Row B — full fidelity.
    let (st, r) = post(
        app.clone(),
        format!(
            r#"{{"match":{{"source_match_id":"{SRC_B}","outcome":"success","winning_faction":"USA"}},"players":[{{"arma_id":"{ARMA}","role_played":"SL","source_event_id":"{EV_B}","counters":{{"kills":17,"deaths":3,"team_kills":1,"longest_kill_m":842,"vehicles_destroyed":4,"is_command":true,"command_win":true}}}}]}}"#
        ),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "row B: {r}");

    let null_deaths: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM match_player_stats WHERE arma_id = $1 AND deaths IS NULL",
    )
    .bind(ARMA)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(null_deaths, 1, "exactly one unmeasured deaths row");

    sqlx::query("REFRESH MATERIALIZED VIEW CONCURRENTLY leaderboard_totals")
        .execute(&pool)
        .await
        .ok();
    let lb: (i64, i64, Option<f64>, i64) = sqlx::query_as(
        "SELECT kills::int8, deaths::int8, kd_ratio::float8, missions_played::int8 \
         FROM leaderboard_totals WHERE discord_id = $1",
    )
    .bind(DISCORD)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(lb.0, 17, "SUM(kills) ignores NULL");
    assert_eq!(lb.1, 3, "SUM(deaths) ignores NULL — must not invent a 0");
    assert_eq!(lb.3, 2, "both matches still count as played");
    let kd = lb.2.expect("kd_ratio present when measured deaths exist");
    assert!((kd - 5.67).abs() < 1e-9, "17/3 → 5.67, got {kd}");

    clean(pool.clone()).await;
}
