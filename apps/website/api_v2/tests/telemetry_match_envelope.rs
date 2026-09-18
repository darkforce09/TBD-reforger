//! The match half of a match-results report: the dedupe key, the event / mission pointers,
//! the terrain, and what a partial or malformed envelope is allowed to do to a match that
//! already landed. Skips without `TEST_DATABASE_URL`.
//!
//! Every test here owns its own `arma_id` / `discord_id` / `source_match_id` and clears its
//! rows at both ends, because `matches` does not cascade to `match_player_stats` and
//! `leaderboard_totals` sums every row for a `discord_id`.

use axum::http::StatusCode;
use sqlx::PgPool;
use telemetry_support::{SVC, boot, call};
use uuid::Uuid;

mod common;
mod telemetry_support;

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
    // every test binary provisions its own database, so the only rows in `matches` are the ones
    // the tests compiled into this file wrote, and every one of those states a real source id.
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

/// T-501 — a community terrain name degrades the match report; it does not reject it.
///
/// `parse_terrain_opt` soft-fails unknown names to `None` (T-402) because the mission schema
/// constrains terrain to `^[a-z][a-z0-9_]*$`, so community missions legitimately carry names
/// outside `everon|arland|custom`; 400-ing a whole report over one of them was the "production
/// ingest 400s" failure mode for those senders. That was pinned only by a unit test on the
/// helper, which cannot see whether the route wired it up.
///
/// A soft-fail has **two** halves and both have to hold: the request has to succeed, *and* the
/// degradation has to be what actually landed. Asserting the 200 alone would pass over a
/// handler that returns 200 and stores `everon` — a guess dressed as a reading.
///
/// RED (half 1): make `upsert_match` reject an unknown terrain instead of soft-failing (the
/// pre-T-402 behaviour) and the first POST is a 400 — this test fails.
/// RED (half 2): make `parse_terrain_opt` fall back to `Some(TerrainType::Everon)` instead of
/// `None`; every POST still returns 200 while `matches.terrain` reads `everon` — this test
/// fails on the database assertion, which is the assertion a status code cannot make.
#[tokio::test]
async fn community_terrain_soft_fails_to_null_without_dropping_the_report() {
    let Some((app, pool)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    const ARMA: &str = "t501-arma-terrain";
    const DISCORD: &str = "000000000000501001";
    const SRC_A: &str = "m-t501-kolguyev";
    const SRC_B: &str = "m-t501-anizay";
    const EV: &str = "e-t501";

    sqlx::query(
        "INSERT INTO users (discord_id, username, discord_handle, avatar_url, arma_id, arma_character, role, is_banned, ban_reason, created_at, updated_at) \
         VALUES ($1, 'T501', 't501', '', $2, '[TBD] T501', 'enlisted', false, '', now(), now()) \
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

    let post = |body: String| {
        let app = app.clone();
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
    // Honest in every respect except that the terrain is one this platform has never heard of.
    let body = |src: &str, terrain: &str| {
        format!(
            r#"{{"match":{{"source_match_id":"{src}","terrain":"{terrain}","outcome":"success","winning_faction":"USA","ended_at":"2026-07-26T20:14:00Z"}},"players":[{{"arma_id":"{ARMA}","role_played":"SL","source_event_id":"{EV}","counters":{{"kills":7,"deaths":2,"team_kills":0,"longest_kill_m":310,"vehicles_destroyed":1,"is_command":false,"command_win":null}}}}]}}"#
        )
    };

    // (1) `kolguyev` — a real community terrain. The report lands, minus the terrain.
    let (st, first) = post(body(SRC_A, "kolguyev")).await;
    assert_eq!(
        st,
        StatusCode::OK,
        "a community terrain must not 400 the whole report: {first}"
    );
    assert_eq!(first["players"], 1);
    assert_eq!(
        first["linked"], 1,
        "the player is linked, so the row counts"
    );
    assert_eq!(first["unlinked"], 0);

    let stored: (Option<String>, String, Option<String>, bool) = sqlx::query_as(
        "SELECT terrain::text, outcome::text, winning_faction, ended_at IS NOT NULL \
         FROM matches WHERE source_match_id = $1",
    )
    .bind(SRC_A)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        stored.0, None,
        "THE TICKET: an unknown terrain stores NULL — it must never be guessed onto a known pin"
    );
    // The degrade must be *only* the terrain. If anything else went missing, "soft-fail" would
    // just be a quieter way to lose the report.
    assert_eq!(stored.1, "success", "the outcome still landed");
    assert_eq!(stored.2.as_deref(), Some("USA"), "the winner still landed");
    assert!(stored.3, "ended_at still landed");
    let counters: (Option<i64>, Option<i64>) = sqlx::query_as(
        "SELECT s.kills, s.deaths FROM match_player_stats s \
         INNER JOIN matches m ON m.id = s.match_id \
         WHERE s.arma_id = $1 AND m.source_match_id = $2",
    )
    .bind(ARMA)
    .bind(SRC_A)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        counters,
        (Some(7), Some(2)),
        "the scoreline landed in full — degraded means the terrain, not the match"
    );

    // (2) A second community name, so the first is not a one-off in the enum's neighbourhood.
    let (st, second) = post(body(SRC_B, "anizay")).await;
    assert_eq!(st, StatusCode::OK, "second community terrain: {second}");
    let t_b: Option<String> =
        sqlx::query_scalar("SELECT terrain::text FROM matches WHERE source_match_id = $1")
            .bind(SRC_B)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(t_b, None, "`anizay` soft-fails to NULL as well");

    // (3) Control — same route, same match, a *known* pin. Without it, both assertions above
    // are equally satisfied by a handler that never writes this column at all.
    let (st, third) = post(body(SRC_A, "everon")).await;
    assert_eq!(st, StatusCode::OK, "known terrain: {third}");
    assert_eq!(
        third["match_id"], first["match_id"],
        "a corrected re-POST is the same match, not a second row"
    );
    let t_a: Option<String> =
        sqlx::query_scalar("SELECT terrain::text FROM matches WHERE source_match_id = $1")
            .bind(SRC_A)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        t_a.as_deref(),
        Some("everon"),
        "a known terrain still writes through — the NULLs above are the soft-fail, not a \
         handler that cannot write this column"
    );

    clean(pool.clone()).await;
}

/// T-533 — a malformed `event_id` / `mission_id` is a 400 **through the route**, and nothing
/// is written.
///
/// `parse_uuid_opt_strict` is T-355: junk used to become `None` through the soft parser, the
/// match stored with no event or mission, the attendance UPDATE matched nothing, and the game
/// server got a **200** for a report that had silently lost its attribution. The helper's unit
/// tests and the Class-R source pin both hold, but neither can answer the only question a game
/// server actually asks — what does the endpoint do. This POSTs the junk.
///
/// The status code is the smaller half. The larger half is that the transaction did not
/// half-land: a 400 returned over a `matches` row that was already inserted is the same silent
/// loss in a different costume.
///
/// RED: swap `parse_uuid_opt_strict` back to `parse_uuid_opt` at both `upsert_match` call
/// sites (the T-355 pre-fix) and every junk POST below returns 200 with a match row written —
/// this test fails on the first status assertion and again on "nothing written".
#[tokio::test]
async fn junk_event_or_mission_id_is_a_400_that_writes_nothing() {
    let Some((app, pool)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    const ARMA: &str = "t533-arma-junk-ids";
    const DISCORD: &str = "000000000000533001";
    const SRC: &str = "m-t533-junk";
    const EV: &str = "e-t533";

    sqlx::query(
        "INSERT INTO users (discord_id, username, discord_handle, avatar_url, arma_id, arma_character, role, is_banned, ban_reason, created_at, updated_at) \
         VALUES ($1, 'T533', 't533', '', $2, '[TBD] T533', 'enlisted', false, '', now(), now()) \
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
        let (st, r) = post(body(ids)).await;
        assert_eq!(st, StatusCode::BAD_REQUEST, "junk {field} ({ids}): {r}");
        assert_eq!(
            r["error"],
            format!("invalid {field}"),
            "the 400 must name the field the sender has to fix: {r}"
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

    // Control — blank is "omit", not junk (`parse_uuid_opt_strict` returns `Ok(None)`), so the
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
