//! What reaches `leaderboard_totals` and the denormalised user counters, and what does not:
//! a player line whose `arma_id` resolves to no account is kept but unowned (and claimed later
//! by the link flow), and a counters column nobody measured stays NULL instead of becoming a
//! measured zero. Skips without `TEST_DATABASE_URL`.
//!
//! The materialized view is refreshed in-request by every ingest, so each test refreshes it
//! explicitly before reading it rather than depending on a sibling's timing.

use axum::Router;
use axum::http::StatusCode;
use sqlx::PgPool;
use telemetry_support::{SVC, boot, call};
use uuid::Uuid;

mod common;
mod telemetry_support;

/// A player whose `arma_id` resolves to no account must keep their row, and the 200 must not
/// imply the whole roster landed.
///
/// The invisibility, measured on a throwaway database: one POST carrying
/// `kills=17 deaths=3 longest_kill_m=842 vehicles_destroyed=4` for an unlinked `arma_id` returned
/// `{"match_id":"4dc322a3-…","players":1}`, wrote the row with `discord_id` NULL, and left
/// `leaderboard_totals` with **zero** rows for that player (it filters
/// `WHERE discord_id IS NOT NULL`) and `users.total_deployments` at **0**. Nothing anywhere
/// recorded that a scoreline had gone missing — the count in the response was the *submitted*
/// count and said nothing about how much of it was countable.
///
/// The row is kept rather than rejected, and that is the decision this test pins. It is real
/// telemetry — the `arma_id` is real and the match happened — and it is *recoverable*, because
/// `ingest_link_confirm` claims exactly the `discord_id IS NULL` rows at link time. A 400
/// would also have no per-player shape: the transaction is atomic, so one unresolved player would
/// reject the whole op. And the shipping mod implements no link flow at all
/// (`TBD_ResultsReporter.c:23-35`), so an unresolved `arma_id` is currently *every*
/// player in *every* production match — which is why the last leg below, a roster with nobody
/// linked, has to be a 200.
///
/// **The backfill belongs to `ingest_link_confirm`**, and the link leg asserts it from this
/// side on purpose: the upsert key includes `arma_id`, and that key is exactly what lets the
/// backfill find the row again. Pinning it here means a regression in the link-confirm handler
/// fails the suite that owns the ingest contract depending on it.
///
/// Two ingest calls only — the strict limiter is keyed on the peer IP, which is `0.0.0.0` for every
/// test in this binary, so every test compiled into this file shares one 1/s + burst-10 bucket. The roster is built to
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

    // Same reasoning as the sibling envelope tests: `matches` does not cascade to
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
    // so an absence assertion cannot pass (or fail) on somebody else's timing — the same reason
    // the partial-reingest test refreshes.
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

    // (4) The loss is discoverable by an operator, not only by whoever reads the game
    // server's console. Info rather than Warn on purpose: with no link flow in the shipping mod
    // this fires on every production ingest, and an always-on warning is the false
    // `server.low_fps` WARN class all over again.
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

    // (5) The backfill half, owned by link-confirm: linking claims the historical
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
        "backfill: the parked rows reach the leaderboard on link"
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

/// `leaderboard_totals` ignores NULL deaths; it must not invent a measured zero.
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

/// The leaderboard must not collapse "nobody ever measured deaths" into "measured,
/// and it was none". The **installed** view is ratcheted here too, not just the arithmetic.
///
/// [`leaderboard_mv_does_not_invent_deaths_from_null`] above is the SUM pin, and it uses a
/// *mixed* fixture: one counters-absent row beside a measured `17/3`. That is precisely why it
/// cannot see this. `SUM` ignores NULL by definition, so `sum(COALESCE(deaths, 0))` and
/// `COALESCE(sum(deaths), 0)` are **the same number on every fixture** — `0 + 3 = 3` — and
/// poisoning the view that way leaves the pin green. The blind spot was never the sum. It is
/// `count(deaths) FILTER (WHERE deaths IS NOT NULL)`, the one expression in `0014` that can
/// tell an unmeasured row from a measured zero.
///
/// So this pins the pair that differs in nothing else:
///
/// | player | rows                    | MV `deaths` | MV `kd_ratio`             |
/// |--------|-------------------------|-------------|---------------------------|
/// | A      | 2 × counters absent     | `0`         | **NULL** — never measured |
/// | B      | 1 × `kills=4, deaths=0` | `0`         | **4** — measured as none  |
///
/// Both read `deaths = 0`. Only `kd_ratio` separates them, and a leaderboard that prints
/// "0 deaths, KD 0.00" for player A has invented a scoreline nobody reported — the
/// invented-zero failure mode wearing the aggregate's clothes.
///
/// The second half reads the **live** view definition rather than the migration text. `0014`
/// is applied and checksummed, so it can only be superseded, never edited; what a later
/// migration actually left in the database is therefore the thing that has to be true. That is
/// what makes "`COALESCE` moved inside the aggregate" a failing test instead of a code review.
///
/// RED — **verified**: rewrite every `deaths` reference in `0014`'s aggregate as
/// `COALESCE(deaths, 0)` (the poisoning the ticket names). The guard then reads
/// `count(COALESCE(deaths, 0)) FILTER (WHERE COALESCE(deaths, 0) IS NOT NULL)`, which is 2 for
/// player A rather than 0, so it stops firing and `kd_ratio` falls through to
/// `COALESCE(sum(kills), 0)` = `Some(0.0)`. This test fails; the sibling
/// `leaderboard_mv_does_not_invent_deaths_from_null` **passes** under the same poisoning, which
/// is the blind spot restated as an experiment.
///
/// Note what does *not* RED, because it is the trap: deleting the guard line alone leaves
/// `sum(deaths)` NULL for an all-unmeasured player, so `sum(deaths) = 0` is NULL and the ELSE
/// branch divides by NULL — `kd_ratio` is NULL again and this test would still pass. The guard
/// only becomes load-bearing once something has coalesced the NULLs away, which is exactly the
/// pairing `0014` was written to hold together.
#[tokio::test]
async fn leaderboard_kd_is_null_when_deaths_were_never_measured() {
    let Some((app, pool)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    // Player A — every row unmeasured. Player B — one row that really did measure zero deaths.
    const ARMA_A: &str = "t493-arma-never-measured";
    const DISCORD_A: &str = "000000000000493101";
    const ARMA_B: &str = "t493-arma-measured-zero";
    const DISCORD_B: &str = "000000000000493102";
    const SRC_A1: &str = "m-t493-never-1";
    const SRC_A2: &str = "m-t493-never-2";
    const SRC_B: &str = "m-t493-zero";
    const EV: &str = "e-t493";

    sqlx::query(
        "INSERT INTO users (discord_id, username, discord_handle, avatar_url, arma_id, arma_character, role, is_banned, ban_reason, created_at, updated_at) \
         VALUES ($1, 'T493a', 't493a', '', $2, '[TBD] T493a', 'enlisted', false, '', now(), now()) \
         ON CONFLICT (discord_id) DO UPDATE SET arma_id = EXCLUDED.arma_id",
    )
    .bind(DISCORD_A)
    .bind(ARMA_A)
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO users (discord_id, username, discord_handle, avatar_url, arma_id, arma_character, role, is_banned, ban_reason, created_at, updated_at) \
         VALUES ($1, 'T493b', 't493b', '', $2, '[TBD] T493b', 'enlisted', false, '', now(), now()) \
         ON CONFLICT (discord_id) DO UPDATE SET arma_id = EXCLUDED.arma_id",
    )
    .bind(DISCORD_B)
    .bind(ARMA_B)
    .execute(&pool)
    .await
    .unwrap();

    // Same reasoning as the sibling NULL-deaths tests: `matches` does not cascade to
    // `match_player_stats`, and the view SUMs every row for a discord_id, so a second run
    // against a surviving database would double-count.
    let clean = |pool: PgPool| async move {
        sqlx::query("DELETE FROM match_player_stats WHERE arma_id = ANY($1)")
            .bind(vec![ARMA_A.to_string(), ARMA_B.to_string()])
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM matches WHERE source_match_id = ANY($1)")
            .bind(vec![
                SRC_A1.to_string(),
                SRC_A2.to_string(),
                SRC_B.to_string(),
            ])
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

    // Player A: two identity-only reports — the mod path, which claims nothing about the
    // scoreline at all.
    for src in [SRC_A1, SRC_A2] {
        let (st, r) = post(format!(
            r#"{{"match":{{"source_match_id":"{src}","outcome":"success","winning_faction":"USA"}},"players":[{{"arma_id":"{ARMA_A}","role_played":"SL","source_event_id":"{EV}"}}]}}"#
        ))
        .await;
        assert_eq!(st, StatusCode::OK, "player A {src}: {r}");
    }
    // Player B: one full report whose measured deaths really is zero.
    let (st, r) = post(format!(
        r#"{{"match":{{"source_match_id":"{SRC_B}","outcome":"success","winning_faction":"USA"}},"players":[{{"arma_id":"{ARMA_B}","role_played":"SL","source_event_id":"{EV}","counters":{{"kills":4,"deaths":0,"team_kills":0,"longest_kill_m":120,"vehicles_destroyed":0,"is_command":false,"command_win":null}}}}]}}"#
    ))
    .await;
    assert_eq!(st, StatusCode::OK, "player B: {r}");

    // The fixture *is* the argument, so prove the fixture is the one claimed. Without this a
    // mis-seeded run could satisfy every assertion below for the wrong reason.
    let a_measured: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM match_player_stats WHERE arma_id = $1 AND deaths IS NOT NULL",
    )
    .bind(ARMA_A)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        a_measured, 0,
        "player A must have no measured deaths reading"
    );
    let b_zero: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM match_player_stats WHERE arma_id = $1 AND deaths = 0",
    )
    .bind(ARMA_B)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(b_zero, 1, "player B must have exactly one measured zero");

    sqlx::query("REFRESH MATERIALIZED VIEW CONCURRENTLY leaderboard_totals")
        .execute(&pool)
        .await
        .ok();
    let row = |pool: PgPool, discord: &'static str| async move {
        sqlx::query_as::<_, (i64, i64, Option<f64>, i64)>(
            "SELECT kills::int8, deaths::int8, kd_ratio::float8, missions_played::int8 \
             FROM leaderboard_totals WHERE discord_id = $1",
        )
        .bind(discord)
        .fetch_one(&pool)
        .await
        .unwrap()
    };
    let a = row(pool.clone(), DISCORD_A).await;
    let b = row(pool.clone(), DISCORD_B).await;

    // THE TICKET. Both players report `deaths = 0`; only `kd_ratio` says which of the two is a
    // scoreline somebody actually measured.
    assert_eq!(
        a.1, 0,
        "player A sums to 0 deaths — NULLs contribute nothing"
    );
    assert_eq!(b.1, 0, "player B measured 0 deaths");
    assert_eq!(
        a.2, None,
        "THE TICKET / RED: an all-unmeasured row set must leave kd_ratio NULL, not 0 — got {:?}. \
         With the FILTER guard removed this is Some(0.0), which reads on the leaderboard as a \
         scoreline nobody reported.",
        a.2
    );
    let b_kd = b.2.expect(
        "a measured zero-deaths row set IS a scoreline — kd_ratio must not be NULL here, or the \
         view has collapsed the two cases the other way round",
    );
    assert!(
        (b_kd - 4.0).abs() < 1e-9,
        "measured 0 deaths → kd_ratio falls back to total kills (4), got {b_kd}"
    );
    assert_eq!(a.0, 0, "player A never claimed a kill either");
    assert_eq!(a.3, 2, "both of player A's matches still count as played");

    // ---- ratchet the INSTALLED view, not the migration text ----
    let viewdef: String =
        sqlx::query_scalar("SELECT pg_get_viewdef('public.leaderboard_totals'::regclass, true)")
            .fetch_one(&pool)
            .await
            .unwrap();
    // A ratchet that silently matched nothing would be the exact defect this slice exists to
    // remove, so prove the definition was read before asserting anything about it.
    assert!(
        viewdef.len() > 200 && viewdef.contains("kd_ratio"),
        "pg_get_viewdef returned nothing recognisable — the two assertions below would pass \
         over an input they never examined: {viewdef}"
    );
    let norm = viewdef.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        norm.contains("deaths IS NOT NULL"),
        "the installed leaderboard_totals must keep its FILTER (WHERE deaths IS NOT NULL) \
         guard — it is the only expression that separates unmeasured from zero:\n{viewdef}"
    );
    let lowered = norm.to_lowercase();
    assert!(
        !lowered.contains("coalesce(deaths") && !lowered.contains("coalesce(s.deaths"),
        "COALESCE must not be applied to the `deaths` column inside the aggregate — that is what \
         turns 'not measured' into a measured zero:\n{viewdef}"
    );

    clean(pool.clone()).await;
}
