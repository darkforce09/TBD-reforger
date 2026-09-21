//! The player half of a match-results report: the counters block is optional, but it is
//! all-or-nothing when present, an absent one writes nothing at all, and a first insert
//! without one stores NULL rather than a scored zero. The shipping mod's own payload is
//! pinned here byte-for-byte, because it is the sender that made the distinction load-bearing.
//! Skips without `TEST_DATABASE_URL`.

use axum::Router;
use axum::http::StatusCode;
use sqlx::PgPool;
use telemetry_support::{SVC, boot, call};
use uuid::Uuid;

mod common;
mod telemetry_support;

/// **The payload the shipping mod actually sends, reproduced byte-for-byte, asserted to be
/// accepted.**
///
/// This fixture is the whole point of the suite. Every other test in this file builds its own
/// complete body, so without it nothing here looks at what the game server puts on the wire.
/// The consequence is arithmetic, not bad luck: the mod sends four player keys, so a contract
/// that requires a fifth makes serde reject on the first key the mod does not send, and
/// **every match report from every production server 400s** — match rows, per-player stats,
/// attendance, user-stat recompute and leaderboard refresh, all dead on arrival.
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
    // Both arma_ids stay unlinked on purpose: `TBD_ResultsReporter.c:23-35` says no player
    // carries an `arma_id` in production until the link flow ships, so this is the real
    // population, and the unlinked-reporting path is the same code path as a linked report.
    const A1: &str = "counters-arma-mod-a";
    const A2: &str = "counters-arma-mod-b";
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

    // `0019_ingest_pointer_foreign_keys.sql` constrains `matches.event_id` and
    // `matches.mission_id`, so the two ids the mod sends must name rows that exist. Posting
    // pointers to nothing would store a match whose `event_id` dangles — the exact
    // silent attribution loss 0019 exists to end.
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
         VALUES ($1, 'Counters Shipping Mod Mission', $2, 'everon', 'pve_coop', 32, 'live', now(), now()) \
         ON CONFLICT (id) DO NOTHING",
    )
    .bind(Uuid::parse_str(MISSION).expect("MISSION is a uuid literal"))
    .bind(AUTHOR)
    .execute(&pool)
    .await
    .expect("seed the mission the shipping payload names");
    sqlx::query(
        "INSERT INTO events (id, name_override, start_time, status, created_by, created_at, updated_at) \
         VALUES ($1, 'Counters Shipping Mod Event', now() - interval '2 hours', 'open', $2, now(), now()) \
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

    // Both player rows exist with their identity core intact. The flat fold turns the shipping
    // top-level `deaths` into a complete scoreline (zeros for fields the mod does not measure).
    // Identity-only re-ingest (no nested block and no flat keys) still writes no counters —
    // that is `absent_counters_are_not_a_write_on_reingest`.
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
                Some(0),
                Some(1),
                Some(0),
                Some(0),
                Some(0),
                Some(false),
                None
            ),
            (
                A2.into(),
                "Rifleman".into(),
                Some(0),
                Some(0),
                Some(0),
                Some(0),
                Some(0),
                Some(false),
                None
            ),
        ],
        "shipping flat deaths fold into a complete scoreline"
    );

    // The flat fold recovers the shipping `deaths`. Nested `counters` is all-or-nothing when
    // present; identity-only bodies write no counters.

    assert_eq!(
        rows[0].3,
        Some(1),
        "top-level deaths is stored via the flat fold"
    );

    sqlx::query("DELETE FROM audit_logs WHERE target_id = $1")
        .bind(match_id.to_string())
        .execute(&pool)
        .await
        .unwrap();
    clean(pool.clone()).await;
}

/// **A counters block is all-or-nothing: a partial one is a 400.**
///
/// The *block* is optional; the fields inside it are not. That distinction is the entire
/// anti-corruption property. "No counters" is a legal statement meaning "I make no claim" and
/// writes nothing; "some counters" is a sender that has half a scoreline and does not know it,
/// and five silent zeros beside two real numbers is precisely the corrupt row this contract
/// refuses. There is no `#[serde(default)]` inside `PlayerCountersInput`, so a missing key is a
/// decode error.
#[tokio::test]
async fn a_partial_counters_object_is_still_a_400() {
    let Some((app, pool)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    const ARMA: &str = "counters-arma-partial";
    const SRC: &str = "m-counters-partial";
    const EV: &str = "e-counters-partial";

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
    // exactly why a `GREATEST` merge is wrong here and why the block is all-or-nothing rather
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

    // (3) The flat body. The handler folds it into nested counters when the nested block is
    // absent, so this is a 200 that stores the stated scoreline (not a 400, and not a silent
    // drop). Partial *nested* blocks above still 400.
    let (st, r) = post(
        app.clone(),
        body(format!(
            r#"{{"arma_id":"{ARMA}","role_played":"SL","source_event_id":"{EV}","kills":9,"deaths":2,"team_kills":0,"longest_kill_m":100,"vehicles_destroyed":1,"is_command":false}}"#
        )),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "flat body folds: {r}");
    assert_eq!(
        read(pool.clone()).await,
        (9, 2, 0, 100, 1, false),
        "flat payload stores the stated scoreline"
    );

    clean(pool.clone()).await;
}

/// **Absent counters are not a write.**
///
/// This is the assertion that matters most in the file. The optional block would be worthless
/// (and dangerous) if omitting it merely meant "send zeros politely": an omission that wrote
/// `kills=0 … command_win=NULL` would overwrite a real scoreline, and `refresh_leaderboard`
/// would sum the zeros in the same request. An omission writes *nothing at all* — the
/// counters-absent SQL statement does not name those columns, so they are not read, not
/// re-bound, and not rewritten.
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
    const ARMA: &str = "counters-arma-noclaim";
    const DISCORD: &str = "000000000000393001";
    const SRC: &str = "m-counters-noclaim";
    const EV: &str = "e-counters-noclaim";

    // Link the player so the leaderboard half of the property is observable too — an unowned
    // row never reaches `leaderboard_totals`, so an unlinked player could not show that
    // a zeroing would have propagated.
    sqlx::query(
        "INSERT INTO users (discord_id, username, discord_handle, avatar_url, arma_id, arma_character, role, is_banned, ban_reason, created_at, updated_at) \
         VALUES ($1, 'Counters', 'counters', '', $2, '[TBD] Counters', 'enlisted', false, '', now(), now()) \
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
    // corrected role, so the write is provable. This body is neither a 400 nor a silent
    // zeroing: it states a role and says nothing about the scoreline.
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

    // (3) And the propagation this property protects: `leaderboard_totals` sums
    // `match_player_stats`, so a zeroed row really would reach the leaderboard inside the
    // same request via `refresh_leaderboard`.
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

/// An INSERT without counters stores NULL, not 0.
///
/// A counters-absent statement that omitted the columns would let DDL `DEFAULT 0` fill them,
/// and a stored 0 reads as a scored 0. RED: switch to an omit-columns INSERT (or re-add
/// `NOT NULL DEFAULT 0`) and `deaths` comes back `Some(0)` — this test fails.
#[tokio::test]
async fn insert_without_counters_stores_null_not_zero() {
    let Some((app, pool)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    const ARMA: &str = "absent-arma-null-insert";
    const DISCORD: &str = "000000000000397101";
    const SRC: &str = "m-absent-null-insert";
    const EV: &str = "e-absent-null-insert";

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
