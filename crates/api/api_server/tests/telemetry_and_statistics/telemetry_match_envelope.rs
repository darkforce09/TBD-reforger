//! The match half of a match-results report: a partial re-ingest cannot revert or zero a match
//! that already landed, and a junk event or mission pointer is a 400 that writes nothing.
//!
//! Every test here owns its own `arma_id` / `discord_id` / `source_match_id` and clears its
//! rows at both ends, because `matches` does not cascade to `match_player_stats` and
//! `leaderboard_totals` sums every row for a `discord_id`.

use crate::telemetry_support;

use axum::Router;
use axum::http::StatusCode;
use serde_json::{Value, json};
use sqlx::PgPool;
use telemetry_support::boot;
use telemetry_support::match_reports::ReportingServer;
use uuid::Uuid;

/// Report a results body given as JSON text as the next revision of its match.
async fn report(reporter: &ReportingServer, app: &Router, body: &str) -> (StatusCode, Value) {
    let body: Value = serde_json::from_str(body).expect("a results body is JSON");
    reporter.report_results(app, &body).await
}

/// Post a results body given as JSON text as `revision` without registering its match: the
/// shape of a report the whole-body validation refuses before any registration is consulted.
async fn post_revision(
    reporter: &ReportingServer,
    app: &Router,
    revision: i64,
    body: &str,
) -> (StatusCode, Value) {
    let body: Value = serde_json::from_str(body).expect("a results body is JSON");
    reporter.post_results(app, revision, &body).await
}

/// A partial re-ingest must not walk a finished match backwards or zero a
/// scoreline. Every body below is one a buggy or retried game server could plausibly send;
/// each is a shape that returns 200 and destroys data when the guard is missing.
///
/// The seed is revision 1; each malformed body below is offered as revision 2.
#[tokio::test]
async fn partial_match_reingest_cannot_revert_or_zero() {
    let Some((app, pool)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let reporter = &ReportingServer::open(&app, &pool, "Envelope partial server").await;
    const ARMA: &str = "revert-arma-revert";
    const DISCORD: &str = "000000000000316001";
    const SRC: &str = "m-revert-revert";

    sqlx::query(
        "INSERT INTO users (discord_id, username, discord_handle, avatar_url, arma_id, arma_character, role, is_banned, ban_reason, created_at, updated_at) \
         VALUES ($1, 'Revert', 'revert', '', $2, '[TBD] Revert', 'enlisted', false, '', now(), now()) \
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
        r#"{{"match":{{"source_match_id":"{SRC}","outcome":"success","winning_faction":"USA","aar_replay_url":"https://aar.tbd/{SRC}.json","ended_at":"2026-07-26T20:14:00Z"}},"players":[{{"arma_id":"{ARMA}","role_played":"SL","source_event_id":"e-revert","counters":{{"kills":17,"deaths":3,"team_kills":1,"longest_kill_m":842,"vehicles_destroyed":4,"is_command":true,"command_win":true}}}}]}}"#
    );
    let (st, r) = report(reporter, &app, &full).await;
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
    let (st, r) = post_revision(
        reporter,
        &app,
        2,
        &format!(r#"{{"match":{{"source_match_id":"{SRC}"}},"players":[]}}"#),
    )
    .await;
    assert_eq!(st, StatusCode::BAD_REQUEST, "partial match body: {r}");
    assert_eq!(read_match(pool.clone()).await, before, "match untouched");

    // (2) A player row missing a required *identity* field — this body has neither
    // `role_played` nor a scoreline, and without the guard it zeroes a 17/3 line.
    //
    // It is rejected for the missing `role_played`, not for the missing counters: counters live
    // in an optional nested block, and omitting the block is a legal statement meaning "I make
    // no claim about the scoreline". That an omission must never be a write is asserted
    // directly by `absent_counters_are_not_a_write_on_reingest`, which sends a *well-formed*
    // counters-less row and proves the stored 17/3 survives it. The two tests together say:
    // silence never writes, and an incomplete identity is still a 400.
    let (st, r) = post_revision(
        reporter,
        &app,
        2,
        &format!(
            r#"{{"match":{{"source_match_id":"{SRC}","outcome":"success","winning_faction":"USA"}},"players":[{{"arma_id":"{ARMA}","source_event_id":"e-revert"}}]}}"#
        ),
    )
    .await;
    assert_eq!(st, StatusCode::BAD_REQUEST, "partial player body: {r}");
    assert_eq!(
        read_stats(pool.clone()).await,
        (17, 3, 1, 842, 4, true),
        "counters untouched"
    );

    // The propagation claim, checked at the source: `leaderboard_totals` sums
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
    let (st, r) = reporter
        .post(&app, "/api/v1/ingest/match-results", &json!({}))
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

/// A malformed `event_id` / `mission_id` is a 400 **through the route**, and nothing
/// is written.
///
/// The results decoder parses both pointers strictly: through a soft parser junk would become
/// `None`, the match would store with no event or mission, the attendance update would match
/// nothing, and the game server would get a **200** for a report that has silently lost its
/// attribution. This POSTs the junk.
///
/// The status code is the smaller half. The larger half is that the transaction did not
/// half-land: a 400 returned over a `matches` row that was already inserted is the same silent
/// loss in a different costume.
///
/// RED: make the decoder soft-parse either pointer and no junk POST below answers 400 —
/// this test fails on the first status assertion.
#[tokio::test]
async fn junk_event_or_mission_id_is_a_400_that_writes_nothing() {
    let Some((app, pool)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let reporter = &ReportingServer::open(&app, &pool, "Envelope junk pointer server").await;
    const ARMA: &str = "junk-arma-junk-ids";
    const DISCORD: &str = "000000000000533001";
    const SRC: &str = "m-junk-junk";
    const EV: &str = "e-junk";

    sqlx::query(
        "INSERT INTO users (discord_id, username, discord_handle, avatar_url, arma_id, arma_character, role, is_banned, ban_reason, created_at, updated_at) \
         VALUES ($1, 'Junk', 'junk', '', $2, '[TBD] Junk', 'enlisted', false, '', now(), now()) \
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

    let post = |body: String| {
        let app = app.clone();
        async move { report(reporter, &app, &body).await }
    };
    // `ids` is spliced into the match object, so each case differs only in the two pointers.
    let body = |ids: &str| {
        format!(
            r#"{{"match":{{"source_match_id":"{SRC}","outcome":"success","winning_faction":"USA"{ids}}},"players":[{{"arma_id":"{ARMA}","role_played":"SL","source_event_id":"{EV}","counters":{{"kills":2,"deaths":1,"team_kills":0,"longest_kill_m":40,"vehicles_destroyed":0,"is_command":false,"command_win":null}}}}]}}"#
        )
    };

    // Three shapes of junk, and the error must name **which** field — a game server's only
    // channel back is this string.
    for (ids, field) in [
        (r#","event_id":"not-a-uuid""#, "event_id"),
        (r#","mission_id":"12345""#, "mission_id"),
        // A truncated uuid: the shape a sender produces by slicing a real id.
        (
            r#","event_id":"9f0f4c6e-1d3a-4e2b-8c77-2a5b6d4e90""#,
            "event_id",
        ),
    ] {
        // Offered without a registration: the whole body is validated before the registration
        // is consulted, so "nothing written" covers the match row as well.
        let (st, r) = post_revision(reporter, &app, 1, &body(ids)).await;
        assert_eq!(st, StatusCode::BAD_REQUEST, "junk {field} ({ids}): {r}");
        assert_eq!(
            r["error"],
            format!("{field} must be a UUID"),
            "the 400 must name the field the sender has to fix: {r}"
        );
        assert_eq!(
            r["details"],
            json!({"code": "INVALID_MATCH_RESULTS", "field": field}),
            "the refusal names the field in its details too: {r}"
        );
    }

    // Nothing above wrote anything — not a match, not a stat row.
    let matches: i64 =
        sqlx::query_scalar("SELECT count(*) FROM matches WHERE source_match_id = $1")
            .bind(SRC)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(matches, 0, "a rejected pointer must not mint a match row");
    let stats: i64 =
        sqlx::query_scalar("SELECT count(*) FROM match_player_stats WHERE arma_id = $1")
            .bind(ARMA)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(stats, 0, "no stat row written either");

    // Control — blank is "omit", not junk (the decoder reads a blank pointer as absent), so the
    // very same body shape must still be a 200. Without this the assertions above are equally
    // satisfied by an endpoint that 400s everything.
    let (st, ok) = post(body(r#","event_id":"","mission_id":"   ""#)).await;
    assert_eq!(st, StatusCode::OK, "blank ids are omit, not junk: {ok}");
    // `fetch_all` rather than a count + aggregate: this Postgres has no `min(uuid)`, and the
    // whole-row form asserts "exactly one row, both pointers unset" in one comparison anyway.
    let landed: Vec<(Option<Uuid>, Option<Uuid>)> =
        sqlx::query_as("SELECT event_id, mission_id FROM matches WHERE source_match_id = $1")
            .bind(SRC)
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(
        landed,
        vec![(None, None)],
        "exactly one match row, both pointers unset"
    );

    clean(pool.clone()).await;
}
