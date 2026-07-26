//! Telemetry ingest — server-status upsert (+ low-FPS audit + status read-back) and
//! match-results (idempotent, arma→discord resolve, leaderboard MV refresh, stats
//! recompute). Skips without `TEST_DATABASE_URL`.

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

const SVC: &str = "test-service-token";
const PLAYER_DISCORD: &str = "000000000000000003";
const PLAYER_ARMA: &str = "test-arma-999";

async fn boot() -> Option<(Router, PgPool)> {
    let url = std::env::var("TEST_DATABASE_URL").ok()?;
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    let app = app::router(AppState::new(
        pool.clone(),
        Config::for_tests(url, "tele-secret"),
    ));
    Some((app, pool))
}

async fn admin_token(app: &Router) -> String {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/auth/dev-login?role=admin")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let loc = resp.headers()[header::LOCATION].to_str().unwrap();
    loc.split_once('#')
        .unwrap()
        .1
        .split('&')
        .find_map(|p| p.strip_prefix("access_token="))
        .unwrap()
        .to_string()
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
    let match_body = format!(
        r#"{{"match":{{"source_match_id":"m-tele-1","outcome":"success","winning_faction":"USA"}},"players":[{{"arma_id":"{PLAYER_ARMA}","role_played":"SL","kills":5,"deaths":1,"team_kills":0,"longest_kill_m":0,"vehicles_destroyed":0,"is_command":false,"source_event_id":"e1"}}]}}"#
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
        r#"{{"match":{{"source_match_id":"{SRC}","outcome":"success","winning_faction":"USA","aar_replay_url":"https://aar.tbd/{SRC}.json","ended_at":"2026-07-26T20:14:00Z"}},"players":[{{"arma_id":"{ARMA}","role_played":"SL","kills":17,"deaths":3,"team_kills":1,"longest_kill_m":842,"vehicles_destroyed":4,"is_command":true,"command_win":true,"source_event_id":"e-t316"}}]}}"#
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

    // (2) A player row with the counters omitted — this zeroed a 17/3 scoreline.
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
