//! Attendance attribution: which `event_registrations` row a landed match marks, what a
//! corrected or re-pointed report does to a mark it already granted, and when a retraction is
//! justified. Skips without `TEST_DATABASE_URL`.
//!
//! `event_registrations` carries no `match_id`, so attendance is attributed *through* the live
//! `matches` rows — which is what makes the write reversible and what every assertion here
//! ultimately pins.

use axum::http::StatusCode;
use sqlx::PgPool;
use telemetry_support::{SVC, boot, call};
use uuid::Uuid;

mod common;
mod telemetry_support;

/// A *corrected* re-POST must land, and attendance must follow it.
///
/// A re-ingest branch that omitted `event_id`, `mission_id`, `terrain` and `started_at` from
/// the UPDATE and returned the *stored* `event_id` would leave a first POST that carries a
/// `source_match_id` but no `event_id`, followed by a corrected re-POST carrying the right one,
/// marking nobody's attendance — forever — on two 200s. The sibling fields (`ended_at` /
/// `winning_faction` / `aar_replay_url`) let a *present* field win, and all seven read alike.
///
/// The three POSTs below are the whole argument: create without the event, correct it, then
/// retry partially. Keep them under the strict limiter's burst (1/s, burst 10).
#[tokio::test]
async fn a_corrected_reingest_lands_the_event_and_marks_attendance() {
    let Some((app, pool)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    const ARMA: &str = "attend-arma-correct";
    const DISCORD: &str = "000000000000369001";
    const SRC: &str = "m-attend-correct";
    const EV: &str = "e-attend";
    const STARTED: &str = "2026-07-26T18:00:00Z";

    sqlx::query(
        "INSERT INTO users (discord_id, username, discord_handle, avatar_url, arma_id, arma_character, role, is_banned, ban_reason, created_at, updated_at) \
         VALUES ($1, 'Attend', 'attend', '', $2, '[TBD] Attend', 'enlisted', false, '', now(), now()) \
         ON CONFLICT (discord_id) DO UPDATE SET arma_id = EXCLUDED.arma_id",
    )
    .bind(DISCORD)
    .bind(ARMA)
    .execute(&pool)
    .await
    .unwrap();
    // Same reasoning as the sibling envelope tests: `matches` does not cascade to
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
         VALUES ('Attend Op', $1, 'everon', 'pve_coop', 32, 'live', now(), now()) RETURNING id",
    )
    .bind(DISCORD)
    .fetch_one(&pool)
    .await
    .unwrap();
    let event_id: Uuid = sqlx::query_scalar(
        "INSERT INTO events (name_override, start_time, status, created_by, created_at, updated_at) \
         VALUES ('Attend Event', now() - interval '2 hours', 'open', $1, now(), now()) RETURNING id",
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
        "the corrected event_id must reach the attendance UPDATE"
    );
    // A row set but derived numbers left short would be half a fix: `recompute_user_stats`
    // runs after the commit and re-counts `state = 'attended'` over past registrations.
    assert_eq!(
        read_user(pool.clone()).await,
        (1, 100.0),
        "attendance_rate follows the corrected row"
    );

    // (3) The absent-keeps direction, unbroken: a partial retry must not null any of the four
    // back out, and must not stamp `started_at` with `now()` — the create path's
    // `unwrap_or_else(Utc::now)` must never reach the UPDATE.
    let (st, retry) = post(format!(
        r#"{{"match":{{"source_match_id":"{SRC}","outcome":"success"}},{players}}}"#
    ))
    .await;
    assert_eq!(st, StatusCode::OK, "partial retry: {retry}");
    assert_eq!(
        read_match(pool.clone()).await,
        after,
        "an omitted field still keeps the stored value"
    );
    assert_eq!(read_state(pool.clone()).await, "attended", "still attended");

    clean(pool.clone()).await;
}

/// Attendance marks only the *played* event_mission, not every mission on the event.
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
    const ARMA: &str = "scope-arma-scope";
    const DISCORD: &str = "000000000000230001";
    const SRC: &str = "m-scope-scope";
    const EV: &str = "e-scope";

    sqlx::query(
        "INSERT INTO users (discord_id, username, discord_handle, avatar_url, arma_id, arma_character, role, is_banned, ban_reason, created_at, updated_at) \
         VALUES ($1, 'Scope', 'scope', '', $2, '[TBD] Scope', 'enlisted', false, '', now(), now()) \
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
         VALUES ('Scope Played', $1, 'everon', 'pve_coop', 32, 'live', now(), now()) RETURNING id",
    )
    .bind(DISCORD)
    .fetch_one(&pool)
    .await
    .unwrap();
    let mission_other: Uuid = sqlx::query_scalar(
        "INSERT INTO missions (title, author_id, terrain, game_mode, max_players, status, created_at, updated_at) \
         VALUES ('Scope Other', $1, 'everon', 'pve_coop', 32, 'live', now(), now()) RETURNING id",
    )
    .bind(DISCORD)
    .fetch_one(&pool)
    .await
    .unwrap();
    let event_id: Uuid = sqlx::query_scalar(
        "INSERT INTO events (name_override, start_time, status, created_by, created_at, updated_at) \
         VALUES ('Scope Multi-mission', now() - interval '2 hours', 'open', $1, now(), now()) RETURNING id",
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
        "the played mission's registration must flip to attended"
    );
    assert_eq!(
        state_other, "registered",
        "the unplayed sibling must stay registered — a join on the event alone marks both attended"
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
    const SRC2: &str = "m-scope-event-only";
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

/// Re-pointing a match at a different event retracts the attendance it granted, and only when
/// no other match still justifies it.
///
/// EV1→EV2 is reachable, and an attendance SET that marks EV2 without undoing EV1 inflates
/// `attendance_rate` to 100% off two past registrations both reading `attended`. The write is
/// reversible because it attributes through live match rows: `event_registrations` carries no
/// `match_id` of its own.
///
/// Both halves are asserted, because the guard is the hard half:
///
/// 1. **retract** — once SRC1 has moved EV1→EV2 and nothing else points at EV1, the EV1
///    registration goes back to `registered`.
/// 2. **`NOT EXISTS` guard** — while SRC2 still points at EV1 carrying this player's stat row,
///    the EV1 registration must stay `attended`. A retract firing here would strip attendance
///    off an operation the player really did play.
///
/// `registered` (not `waitlisted`) is the restore target: it is the state attendance is
/// granted *from*, so it is the state undoing it returns to.
///
/// RED (half 1): delete the `if let Some((old_event, old_mission)) = retract_from` block in
/// `ingest_match_results` and step 4 leaves EV1 `attended` — this test fails.
/// RED (half 2): delete the `AND NOT EXISTS (…)` clause from that same UPDATE and step 3
/// retracts EV1 early, while SRC2 still justifies it — this test fails.
#[tokio::test]
async fn re_pointing_a_match_retracts_prior_attendance_only_when_unjustified() {
    let Some((app, pool)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    const ARMA: &str = "repoint-arma-repoint";
    const DISCORD: &str = "000000000000540001";
    const SRC1: &str = "m-repoint-one";
    const SRC2: &str = "m-repoint-two";
    const EV: &str = "e-repoint";

    sqlx::query(
        "INSERT INTO users (discord_id, username, discord_handle, avatar_url, arma_id, arma_character, role, is_banned, ban_reason, created_at, updated_at) \
         VALUES ($1, 'Repoint', 'repoint', '', $2, '[TBD] Repoint', 'enlisted', false, '', now(), now()) \
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
            .bind(vec![SRC1.to_string(), SRC2.to_string()])
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

    // One mission played under two different events — the shape a re-point corrects. The pair
    // `(event_id, mission_id)` is what `event_missions` is unique on and what both the SET and
    // the retract key off, so moving only `event_id` is the minimal real move.
    let mission_id: Uuid = sqlx::query_scalar(
        "INSERT INTO missions (title, author_id, terrain, game_mode, max_players, status, created_at, updated_at) \
         VALUES ('Repoint Played', $1, 'everon', 'pve_coop', 32, 'live', now(), now()) RETURNING id",
    )
    .bind(DISCORD)
    .fetch_one(&pool)
    .await
    .unwrap();
    let event_1: Uuid = sqlx::query_scalar(
        "INSERT INTO events (name_override, start_time, status, created_by, created_at, updated_at) \
         VALUES ('Repoint First Op', now() - interval '3 hours', 'open', $1, now(), now()) RETURNING id",
    )
    .bind(DISCORD)
    .fetch_one(&pool)
    .await
    .unwrap();
    let event_2: Uuid = sqlx::query_scalar(
        "INSERT INTO events (name_override, start_time, status, created_by, created_at, updated_at) \
         VALUES ('Repoint Second Op', now() - interval '2 hours', 'open', $1, now(), now()) RETURNING id",
    )
    .bind(DISCORD)
    .fetch_one(&pool)
    .await
    .unwrap();
    let mut ems = Vec::new();
    for event in [event_1, event_2] {
        let em: Uuid = sqlx::query_scalar(
            "INSERT INTO event_missions (event_id, mission_id, start_time, created_at, updated_at) \
             VALUES ($1, $2, now() - interval '2 hours', now(), now()) RETURNING id",
        )
        .bind(event)
        .bind(mission_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO event_registrations (event_mission_id, discord_id, state) \
             VALUES ($1, $2, 'registered')",
        )
        .bind(em)
        .bind(DISCORD)
        .execute(&pool)
        .await
        .unwrap();
        ems.push(em);
    }
    let (em_1, em_2) = (ems[0], ems[1]);

    let state = |pool: PgPool, em: Uuid| async move {
        sqlx::query_scalar::<_, String>(
            "SELECT state::text FROM event_registrations \
             WHERE event_mission_id = $1 AND discord_id = $2",
        )
        .bind(em)
        .bind(DISCORD)
        .fetch_one(&pool)
        .await
        .unwrap()
    };
    let post = |src: &str, event: Uuid| {
        let app = app.clone();
        let body = format!(
            r#"{{"match":{{"source_match_id":"{src}","outcome":"success","winning_faction":"USA","event_id":"{event}","mission_id":"{mission_id}","terrain":"everon"}},"players":[{{"arma_id":"{ARMA}","role_played":"SL","source_event_id":"{EV}","counters":{{"kills":3,"deaths":1,"team_kills":0,"longest_kill_m":80,"vehicles_destroyed":0,"is_command":false,"command_win":null}}}}]}}"#
        );
        async move {
            call(
                &app,
                "POST",
                "/api/v1/ingest/match-results",
                None,
                Some(SVC),
                Some(&body),
            )
            .await
        }
    };

    // 1. SRC1 lands on EV1 — the ordinary path into `attended`.
    let (st, r1) = post(SRC1, event_1).await;
    assert_eq!(st, StatusCode::OK, "SRC1 @ EV1: {r1}");
    assert_eq!(
        r1["linked"], 1,
        "the player must resolve, or attendance is never in play and this test is vacuous"
    );
    assert_eq!(state(pool.clone(), em_1).await, "attended", "EV1 marked");
    assert_eq!(
        state(pool.clone(), em_2).await,
        "registered",
        "EV2 untouched"
    );

    // 2. SRC2 lands on EV1 too. Two live matches now justify the same attendance.
    let (st, r2) = post(SRC2, event_1).await;
    assert_eq!(st, StatusCode::OK, "SRC2 @ EV1: {r2}");
    assert_ne!(
        r2["match_id"], r1["match_id"],
        "two distinct source ids are two distinct matches"
    );
    assert_eq!(state(pool.clone(), em_1).await, "attended");

    // 3. Re-point SRC1 → EV2. EV1 is still justified by SRC2, so the guard must hold it.
    let (st, r3) = post(SRC1, event_2).await;
    assert_eq!(st, StatusCode::OK, "SRC1 re-pointed → EV2: {r3}");
    assert_eq!(
        r3["match_id"], r1["match_id"],
        "a re-point is the same match row, not a new one"
    );
    assert_eq!(
        state(pool.clone(), em_1).await,
        "attended",
        "THE GUARD / RED: SRC2 still points at EV1 with this player's stat row, so the retract \
         must not fire — dropping the NOT EXISTS clause strips attendance off an op that was \
         really played"
    );
    assert_eq!(
        state(pool.clone(), em_2).await,
        "attended",
        "the re-point marks the event it moved to"
    );

    // 4. Re-point SRC2 → EV2 as well. Nothing points at EV1 any more.
    let (st, r4) = post(SRC2, event_2).await;
    assert_eq!(st, StatusCode::OK, "SRC2 re-pointed → EV2: {r4}");
    assert_eq!(
        state(pool.clone(), em_1).await,
        "registered",
        "with no match left pointing at EV1, the attendance it granted must be \
         retracted — otherwise both registrations stay `attended` and attendance_rate inflates \
         to 100%"
    );
    assert_eq!(
        state(pool.clone(), em_2).await,
        "attended",
        "EV2 keeps the attendance it earned"
    );

    // The match rows really did move; otherwise everything above is an assertion about a no-op.
    let pointed: Vec<(String, Option<Uuid>)> = sqlx::query_as(
        "SELECT source_match_id, event_id FROM matches WHERE source_match_id = ANY($1) \
         ORDER BY source_match_id",
    )
    .bind(vec![SRC1.to_string(), SRC2.to_string()])
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        pointed,
        vec![
            (SRC1.to_string(), Some(event_2)),
            (SRC2.to_string(), Some(event_2)),
        ],
        "both matches ended up on EV2"
    );

    clean(pool.clone()).await;
}
