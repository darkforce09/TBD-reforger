//! Server-status ingest: the heartbeat upsert, its low-FPS audit, the status read-back, and
//! the whole ingest loop end to end (status → match results → arma→discord resolve →
//! leaderboard refresh → user-stat recompute). Skips without `TEST_DATABASE_URL`.
//!
//! # Fixture ownership
//!
//! `PLAYER_DISCORD` / `PLAYER_ARMA` are the loop test's own identity, in a range no other
//! suite writes, and the seed carries `ON CONFLICT … DO UPDATE SET arma_id = EXCLUDED.arma_id`
//! — so a shared id would mean one binary rewriting another's fixture row mid-run.
//! `tests/identity_link.rs` reads both constants out of this file at compile time to keep its
//! own actors off them.

use axum::http::StatusCode;
use sqlx::PgPool;
use telemetry_support::{SVC, admin_token, boot, call};
use uuid::Uuid;

mod common;
mod telemetry_support;

/// Private player for the ingest loop — must never be the content-golden seed identity
/// (`…003`). See [`ingest_player_is_not_the_content_golden_seed_identity`].
const PLAYER_DISCORD: &str = "000000000000400003";
const PLAYER_ARMA: &str = "telemetry-arma-400003";

/// The main ingest player must not be content_golden Vance.
#[test]
fn ingest_player_is_not_the_content_golden_seed_identity() {
    assert_ne!(
        PLAYER_DISCORD, "000000000000000003",
        "PLAYER_DISCORD must not be content_golden Vance — the golden seed identity is \
         reserved for content fixtures"
    );
    assert_ne!(PLAYER_ARMA, "76561190000000003");
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
    // The body spells the stat block out in full rather than leaning on `#[serde(default)]`
    // to fill `team_kills` / `longest_kill_m` / `vehicles_destroyed` / `is_command` with
    // zeros: a zero-filling default is exactly what lets a re-ingest wipe a real scoreline.
    //
    // The block is spelled out inside `counters`, which is where it lives. Present =
    // authoritative, so this is the "full replace" path; a sender with no scoreline to state
    // may omit the block instead of being rejected
    // (`the_shipping_mod_payload_is_accepted_verbatim`).
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

/// A heartbeat is a merge, not a wipe. Unlike a match result, a partial heartbeat
/// is legitimate, so this path uses `COALESCE` (absent = no new reading) rather than
/// mandatory fields; only a heartbeat with nothing at all to say is rejected.
#[tokio::test]
async fn partial_heartbeat_merges_and_does_not_fire_a_false_low_fps_warn() {
    let Some((app, pool)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let server_id: Uuid = sqlx::query_scalar(
        "INSERT INTO servers (name, ip, port, is_active) VALUES ('Status Merge Srv', '127.0.0.1'::inet, 2316, true) RETURNING id",
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
