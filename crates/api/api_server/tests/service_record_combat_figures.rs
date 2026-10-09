//! The derived combat figures on `GET /api/v1/me/deployments`, and the tripwire for the
//! two figures that are not derivable at all. Skips without `TEST_DATABASE_URL`.
//!
//! Every number asserted here is checked against arithmetic written out in the comments, not
//! against whatever the query happened to return: a K/D that merely *appears* proves nothing.

use api_configuration::configuration::Config;

use api_member_activity::leaderboard_view;
use api_server::router::router;
use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use serde_json::Value;
use sqlx::PgPool;
use tower::ServiceExt;

mod common;
mod telemetry_support;

use telemetry_support::match_reports::ReportingServer;

/// The dev-login admin (`handlers/dev.rs:14`). `/me/deployments` reports only the caller, so
/// exercising the route on real numbers means seeding this identity specifically.
const DEV_ID: &str = "000000000000000001";
/// All seeded rows carry this `source_event_id` so cleanup can be exact and can never reach a row
/// another test owns.
const EV: &str = "e-combat-combat";

async fn setup() -> Option<(Router, String, PgPool)> {
    let url = common::require_test_database_url()?;
    let pool = api_database::connect(&url).await.expect("connect");
    api_database::migrate(&pool).await.expect("migrate");
    let app = router(api_server::composition::application_state(
        pool.clone(),
        Config::for_tests(url, "combat-secret"),
    ));
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/auth/dev-login?role=admin")
                .body(Body::empty())
                .expect("the request builds"),
        )
        .await
        .unwrap();
    let loc = resp.headers()[header::LOCATION]
        .to_str()
        .expect("the Location header is ASCII");
    let access = loc
        .split_once('#')
        .expect("the Location carries a fragment")
        .1
        .split('&')
        .find_map(|p| p.strip_prefix("access_token="))
        .expect("the fragment carries an access token")
        .to_string();
    Some((app, access, pool))
}

async fn deployments(app: &Router, tok: &str) -> Value {
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/me/deployments")
        .header(header::AUTHORIZATION, format!("Bearer {tok}"))
        .body(Body::empty())
        .expect("the request builds");
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK, "GET /me/deployments");
    let bytes = to_bytes(resp.into_body(), 4 * 1024 * 1024)
        .await
        .expect("the response body reads to the end");
    serde_json::from_slice(&bytes).expect("the body decodes as JSON")
}

/// Drop every row this file seeds and rebuild the view, so each phase below starts from a known
/// state regardless of what ran before it.
///
/// `matches` has no cascade to `match_player_stats` (there is no FK between them at all —
/// `pg_constraint` on the child table lists only NOT NULLs), so both tables are cleaned explicitly.
/// Leaving stat rows behind would be silent: `leaderboard_totals` sums *every* row for a
/// `discord_id`, so one orphan makes every K/D in this file wrong.
async fn reset(pool: &PgPool) {
    sqlx::query("DELETE FROM match_player_stats WHERE source_event_id = $1")
        .bind(EV)
        .execute(pool)
        .await
        .expect("clean stats");
    sqlx::query("DELETE FROM matches WHERE source_match_id LIKE 'm-combat-%'")
        .execute(pool)
        .await
        .expect("clean matches");
    leaderboard_view::refresh_leaderboard(pool)
        .await
        .expect("refresh MV");
}

/// Seed one match plus the caller's stat line in it.
///
/// A real `matches` row is inserted rather than an orphan stat row on purpose: `api_operations/src/handlers/member_service_record.rs`
/// documents its zero-date branch as the "unreachable orphan-match path (a MatchPlayerStat always
/// references a real match)", and a test that manufactures orphans would quietly make that comment
/// false.
async fn seed_match(
    pool: &PgPool,
    tag: &str,
    kills: i64,
    deaths: i64,
    is_command: bool,
    command_win: Option<bool>,
) {
    let match_id: uuid::Uuid = sqlx::query_scalar(
        "INSERT INTO matches (source_match_id, started_at, outcome, created_at) \
         VALUES ($1, now(), 'success', now()) RETURNING id",
    )
    .bind(format!("m-combat-{tag}"))
    .fetch_one(pool)
    .await
    .expect("seed match");
    sqlx::query(
        "INSERT INTO match_player_stats \
         (match_id, discord_id, arma_id, role_played, kills, deaths, team_kills, longest_kill_m, \
          vehicles_destroyed, is_command, command_win, source_event_id, created_at) \
         VALUES ($1, $2, $3, 'SL', $4, $5, 0, 0, 0, $6, $7, $8, now())",
    )
    .bind(match_id)
    .bind(DEV_ID)
    .bind(format!("arma-combat-{tag}"))
    .bind(kills)
    .bind(deaths)
    .bind(is_command)
    .bind(command_win)
    .bind(EV)
    .execute(pool)
    .await
    .expect("seed stat");
}

fn f(body: &Value, key: &str) -> f64 {
    body[key]
        .as_f64()
        .unwrap_or_else(|| panic!("`{key}` should be a number, got {}", body[key]))
}

/// One test function, not five: these phases mutate shared rows for a single identity, and running
/// them as separate `#[tokio::test]`s would let cargo interleave them on one database.
#[tokio::test]
async fn derived_combat_figures_match_hand_computation() {
    let Some((app, tok, pool)) = setup().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };

    // ── Phase 1 — the zero case: a player with no ingested matches ──
    //
    // `leaderboard_totals` groups by `discord_id`, so this identity has no row in the view and
    // there is nothing measured to report. Both ratios must be `null`. `0.0` here would be the
    // original defect wearing a humbler number: a claim that we watched and found nothing.
    reset(&pool).await;
    let body = deployments(&app, &tok).await;
    let obj = body.as_object().expect("object body");
    for key in [
        "kills",
        "deaths",
        "kd_ratio",
        "command_games",
        "command_wins",
        "command_win_rate",
    ] {
        // `get_my_deployments` builds all six keys unconditionally in one `json!`, and every
        // failure path before it returns a non-200 that `deployments()` already asserted on. So a
        // 200 response missing one of these is not reachable from *this* source tree — it means the
        // test binary was linked against an `api_server` that predates the handler change.
        //
        // That is a real, measured failure mode, not a hypothetical: a shared
        // `CARGO_TARGET_DIR` clobbers artifacts across worktrees under a *stable* filename hash, so
        // a fresh test object can link a stale rlib. A fresh database does not fix it — the wrong
        // code is in the binary, not the data. It reproduces deliberately: this exact assertion,
        // on this exact line, against a handler that does not build the six keys, on a virgin DB.
        assert!(
            obj.contains_key(key),
            "`{key}` missing from a 200 response, which this source tree cannot produce — \
             `api_operations/src/handlers/member_service_record.rs` builds it unconditionally. Almost certainly a stale link, \
             not a logic bug: rebuild with a private CARGO_TARGET_DIR under /var/tmp and confirm \
             the binary is yours (`grep 'FROM leaderboard_totals WHERE discord_id' <test-binary>` \
             must hit) before believing this failure. Full response: {body}"
        );
    }
    assert!(
        body["kd_ratio"].is_null(),
        "no matches must serialise kd_ratio as null, not 0 — got {}",
        body["kd_ratio"]
    );
    assert!(
        body["command_win_rate"].is_null(),
        "no matches must serialise command_win_rate as null — got {}",
        body["command_win_rate"]
    );
    // Counts are honest at zero: "no kill records exist" is true of this player.
    assert_eq!(body["kills"], 0);
    assert_eq!(body["deaths"], 0);
    assert_eq!(body["command_games"], 0);
    assert_eq!(body["command_wins"], 0);

    // ── Phase 2 — real stats, hand-computed ──
    //
    //   match a: 17 kills,  4 deaths, command, WON
    //   match b:  8 kills,  6 deaths, command, LOST
    //   match c:  2 kills, 10 deaths, not a command slot (command_win NULL)
    //
    //   kills            = 17 + 8 + 2  = 27
    //   deaths           =  4 + 6 + 10 = 20
    //   K/D              = 27 / 20     = 1.35        (round(…, 2) in the view: 1.35 exactly)
    //   command_games    = 2                          (a and b; c is not a command slot)
    //   command_wins     = 1                          (a only)
    //   command_win_rate = 1 / 2       = 0.5         (round(…, 3): 0.500)
    reset(&pool).await;
    seed_match(&pool, "a", 17, 4, true, Some(true)).await;
    seed_match(&pool, "b", 8, 6, true, Some(false)).await;
    seed_match(&pool, "c", 2, 10, false, None).await;
    leaderboard_view::refresh_leaderboard(&pool)
        .await
        .expect("refresh MV");

    let body = deployments(&app, &tok).await;
    assert_eq!(body["kills"], 27, "17 + 8 + 2");
    assert_eq!(body["deaths"], 20, "4 + 6 + 10");
    assert!(
        (f(&body, "kd_ratio") - 1.35).abs() < 1e-9,
        "27 / 20 = 1.35, got {}",
        body["kd_ratio"]
    );
    assert_eq!(
        body["command_games"], 2,
        "two command slots, not three matches"
    );
    assert_eq!(body["command_wins"], 1);
    assert!(
        (f(&body, "command_win_rate") - 0.5).abs() < 1e-9,
        "1 of 2 command games = 0.5, got {}",
        body["command_win_rate"]
    );
    // The whole point of shipping the raw counts: the ratio is checkable from the response alone.
    assert!(
        (f(&body, "kills") / f(&body, "deaths") - f(&body, "kd_ratio")).abs() < 5e-3,
        "kd_ratio must be reproducible from the kills and deaths in the same response"
    );

    // ── Phase 3 — zero deaths must not divide by zero ──
    //
    // Postgres errors on `numeric / 0`, so this is a real failure mode, not a hypothetical. The
    // view guards it with `CASE WHEN sum(deaths) = 0 THEN sum(kills)`, which means a player who has
    // never died reads as their kill count (7), and Deployments agrees with the Leaderboard because
    // both read the same expression.
    reset(&pool).await;
    seed_match(&pool, "flawless", 7, 0, false, None).await;
    leaderboard_view::refresh_leaderboard(&pool)
        .await
        .expect("refresh MV");
    let body = deployments(&app, &tok).await;
    assert_eq!(body["deaths"], 0);
    assert!(
        (f(&body, "kd_ratio") - 7.0).abs() < 1e-9,
        "7 kills / 0 deaths takes the view's CASE branch and reads 7, got {}",
        body["kd_ratio"]
    );

    // ── Phase 4 — played, but never held command ──
    //
    // The common case for most of the roster, and the one the view alone gets wrong: its
    // `command_win_rate` flattens "never commanded" to `0`, indistinguishable from "commanded
    // twice and lost both". `command_games` is `NULLIF(count(…), 0)`, so the handler keys the null
    // off that column instead. A rendered "Win Rate 0%" for a rifleman who was never eligible is
    // exactly the fabrication this suite guards against, inverted.
    reset(&pool).await;
    seed_match(&pool, "grunt", 3, 3, false, None).await;
    leaderboard_view::refresh_leaderboard(&pool)
        .await
        .expect("refresh MV");
    let body = deployments(&app, &tok).await;
    assert!(
        (f(&body, "kd_ratio") - 1.0).abs() < 1e-9,
        "3 / 3 = 1.0, got {}",
        body["kd_ratio"]
    );
    assert_eq!(body["command_games"], 0);
    assert!(
        body["command_win_rate"].is_null(),
        "never-commanded must be null, not 0% — got {}",
        body["command_win_rate"]
    );

    // ── Phase 5 — a measured zero is not the same as no measurement ──
    //
    // This player was in a match and neither killed nor died. `0.0` is the correct answer and it
    // must be *sent*, which is why phase 1 asserts null rather than the handler defaulting
    // everything to zero: the two states have to stay distinguishable on the wire.
    reset(&pool).await;
    seed_match(&pool, "quiet", 0, 0, false, None).await;
    leaderboard_view::refresh_leaderboard(&pool)
        .await
        .expect("refresh MV");
    let body = deployments(&app, &tok).await;
    assert!(
        !body["kd_ratio"].is_null(),
        "a player with rows has a measured K/D, even at zero"
    );
    assert!((f(&body, "kd_ratio") - 0.0).abs() < 1e-9);

    reset(&pool).await;
}

/// The tripwire for "favourite weapon" / "favourite asset".
///
/// An assert-absent test. It pins the measured fact the removal rests on — nothing in this schema
/// observes what a player carried or drove — and, more usefully, it fails the day someone adds the
/// column, with instructions. Without it the next agent either re-derives this whole investigation
/// or, worse, reads `orbat_slots.loadout` and ships authored slot intent as observed telemetry.
///
/// It reads `information_schema` rather than a golden because the claim is about the schema itself,
/// and a golden could only show that today's response omits the key.
#[tokio::test]
async fn no_column_records_what_a_player_actually_used() {
    let Some(url) = common::require_test_database_url() else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };

    let pool = api_database::connect(&url).await.expect("connect");
    api_database::migrate(&pool).await.expect("migrate");

    // Guard the guard: a typo in the table name would make the query below vacuously pass.
    let columns: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM information_schema.columns \
         WHERE table_schema = 'public' AND table_name = 'match_player_stats'",
    )
    .fetch_one(&pool)
    .await
    .expect("count columns");
    assert!(
        columns >= 14,
        "expected the full match_player_stats column set, found {columns} — has the table been \
         renamed? This test asserts an absence and is worthless if it is looking at nothing."
    );

    let found: Vec<String> = sqlx::query_scalar(
        "SELECT column_name FROM information_schema.columns \
         WHERE table_schema = 'public' AND table_name = 'match_player_stats' \
           AND column_name ~* '(weapon|firearm|rifle|gun|vehicle_used|asset)' \
           AND column_name <> 'vehicles_destroyed' \
         ORDER BY column_name",
    )
    .fetch_all(&pool)
    .await
    .expect("scan columns");

    assert!(
        found.is_empty(),
        "match_player_stats now has {found:?} — per-player equipment telemetry has landed. \
         Derive the favourite weapon/asset from it (most frequent across the player's rows), \
         surface the fields on `GET /me/deployments` in `api_operations/src/handlers/member_service_record.rs`, mirror them on \
         `dto.rs::Deployments` with a recaptured golden, restore the `FavLoadout` readouts in \
         `frontend/src/deployments.rs`, and delete this test. Until then those two panels have no \
         data source: `orbat_slots.loadout` is authored slot intent, not what was carried, and \
         `vehicles_destroyed` counts vehicles killed, not driven."
    );
}

type Scoreline = (i64, i64, i64, i64, i64, bool, Option<bool>);

/// Report a results body given as JSON text as the next revision of its match.
async fn ingest(reporter: &ReportingServer, app: &Router, body: &str) -> (StatusCode, Value) {
    let body: Value = serde_json::from_str(body).expect("a results body is JSON");
    reporter.report_results(app, &body).await
}

/// Player lines stored for `arma`.
async fn stored_lines(pool: &PgPool, arma: &str) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM match_player_stats WHERE arma_id = $1")
        .bind(arma)
        .fetch_one(pool)
        .await
        .expect("count lines")
}

/// Assert a 400 that names player line `index` and the flat key `field`.
fn assert_refused_line(status: StatusCode, answer: &Value, index: usize, field: &str) {
    assert_eq!(status, StatusCode::BAD_REQUEST, "{answer}");
    assert_eq!(
        answer["details"]["code"], "INVALID_MATCH_RESULTS",
        "{answer}"
    );
    assert_eq!(
        answer["details"]["index"], index,
        "the refusal names the line: {answer}"
    );
    assert_eq!(
        answer["details"]["field"], field,
        "the refusal names the key: {answer}"
    );
}

async fn read_score(pool: &PgPool, arma: &str) -> Scoreline {
    sqlx::query_as(
        "SELECT kills, deaths, team_kills, longest_kill_m, vehicles_destroyed, is_command, command_win \
         FROM match_player_stats WHERE arma_id = $1",
    )
    .bind(arma)
    .fetch_one(pool)
    .await
    .expect("scoreline row")
}

async fn wipe_ingest(pool: &PgPool, arma: &str, src: &str) {
    sqlx::query("DELETE FROM match_player_stats WHERE arma_id = $1")
        .bind(arma)
        .execute(pool)
        .await
        .expect("wipe stats");
    sqlx::query("DELETE FROM matches WHERE source_match_id = $1")
        .bind(src)
        .execute(pool)
        .await
        .expect("wipe match");
}

/// A flat counter key on a player line is an unknown key: the whole report is a 400 that names
/// the offending line and key, and no line of it is stored — not even the valid one before it.
#[tokio::test]
async fn flat_counter_payload_is_a_400_naming_its_line() {
    let Some((app, _, pool)) = setup().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let reporter = ReportingServer::open(&app, &pool, "Flat payload server").await;
    const ARMA: &str = "t9404-arma-flat";
    const ARMA_VALID: &str = "t9404-arma-flat-valid";
    const SRC: &str = "m-t9404-flat";
    const EV: &str = "e-t9404-flat";
    wipe_ingest(&pool, ARMA, SRC).await;
    wipe_ingest(&pool, ARMA_VALID, SRC).await;

    let body = format!(
        r#"{{"match":{{"source_match_id":"{SRC}","outcome":"success","winning_faction":"USA"}},"players":[{{"arma_id":"{ARMA_VALID}","role_played":"SL","source_event_id":"{EV}","counters":{{"kills":1,"deaths":0,"team_kills":0,"longest_kill_m":0,"vehicles_destroyed":0,"is_command":false}}}},{{"arma_id":"{ARMA}","role_played":"SL","source_event_id":"{EV}","kills":17,"deaths":3,"team_kills":1,"longest_kill_m":842,"vehicles_destroyed":4,"is_command":true,"command_win":true}}]}}"#
    );
    let (st, r) = ingest(&reporter, &app, &body).await;
    assert_refused_line(st, &r, 1, "kills");
    assert_eq!(
        stored_lines(&pool, ARMA).await,
        0,
        "the flat line is not stored"
    );
    assert_eq!(
        stored_lines(&pool, ARMA_VALID).await,
        0,
        "the refusal covers the whole report"
    );

    wipe_ingest(&pool, ARMA, SRC).await;
    wipe_ingest(&pool, ARMA_VALID, SRC).await;
}

/// Of the flat, nested and mixed (nested plus leftover flat keys) shapes, only the nested one
/// is accepted: the flat and mixed lines are 400s naming their line and key, and store nothing.
#[tokio::test]
async fn flat_and_mixed_counter_shapes_are_refused_and_nested_stores() {
    let Some((app, _, pool)) = setup().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let reporter = ReportingServer::open(&app, &pool, "Counter shapes server").await;
    const ARMA_F: &str = "t9404-arma-shape-f";
    const ARMA_N: &str = "t9404-arma-shape-n";
    const ARMA_B: &str = "t9404-arma-shape-b";
    const SRC_F: &str = "m-t9404-shape-f";
    const SRC_N: &str = "m-t9404-shape-n";
    const SRC_B: &str = "m-t9404-shape-b";
    const EV: &str = "e-t9404-shape";
    wipe_ingest(&pool, ARMA_F, SRC_F).await;
    wipe_ingest(&pool, ARMA_N, SRC_N).await;
    wipe_ingest(&pool, ARMA_B, SRC_B).await;

    let counters = r#"{"kills":17,"deaths":3,"team_kills":1,"longest_kill_m":842,"vehicles_destroyed":4,"is_command":true,"command_win":true}"#;
    let flat = format!(
        r#"{{"match":{{"source_match_id":"{SRC_F}","outcome":"success"}},"players":[{{"arma_id":"{ARMA_F}","role_played":"SL","source_event_id":"{EV}","kills":17,"deaths":3,"team_kills":1,"longest_kill_m":842,"vehicles_destroyed":4,"is_command":true,"command_win":true}}]}}"#
    );
    let nested = format!(
        r#"{{"match":{{"source_match_id":"{SRC_N}","outcome":"success"}},"players":[{{"arma_id":"{ARMA_N}","role_played":"SL","source_event_id":"{EV}","counters":{counters}}}]}}"#
    );
    // Nested plus a conflicting leftover flat kills: refused rather than silently resolved.
    let both = format!(
        r#"{{"match":{{"source_match_id":"{SRC_B}","outcome":"success"}},"players":[{{"arma_id":"{ARMA_B}","role_played":"SL","source_event_id":"{EV}","kills":99,"deaths":99,"counters":{counters}}}]}}"#
    );

    let (st, r) = ingest(&reporter, &app, &flat).await;
    assert_refused_line(st, &r, 0, "kills");
    let (st, r) = ingest(&reporter, &app, &nested).await;
    assert_eq!(st, StatusCode::OK, "nested: {r}");
    let (st, r) = ingest(&reporter, &app, &both).await;
    assert_refused_line(st, &r, 0, "kills");

    assert_eq!(
        stored_lines(&pool, ARMA_F).await,
        0,
        "the flat line is not stored"
    );
    assert_eq!(
        read_score(&pool, ARMA_N).await,
        (17, 3, 1, 842, 4, true, Some(true)),
        "the nested scoreline lands in full"
    );
    assert_eq!(
        stored_lines(&pool, ARMA_B).await,
        0,
        "the mixed line is not stored"
    );

    wipe_ingest(&pool, ARMA_F, SRC_F).await;
    wipe_ingest(&pool, ARMA_N, SRC_N).await;
    wipe_ingest(&pool, ARMA_B, SRC_B).await;
}

/// A deaths-only flat line is a 400 naming its line; the nested line the shipping reporter
/// sends for the same round stores deaths (not NULL) with zeros for unmeasured fields.
#[tokio::test]
async fn deaths_only_flat_line_is_refused_and_nested_equivalent_stores_deaths() {
    let Some((app, _, pool)) = setup().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let reporter = ReportingServer::open(&app, &pool, "Deaths only server").await;
    const ARMA_F: &str = "t9404-arma-deaths-f";
    const ARMA_N: &str = "t9404-arma-deaths-n";
    const SRC_F: &str = "m-t9404-deaths-f";
    const SRC_N: &str = "m-t9404-deaths-n";
    const EV: &str = "e-t9404-deaths";
    wipe_ingest(&pool, ARMA_F, SRC_F).await;
    wipe_ingest(&pool, ARMA_N, SRC_N).await;

    let flat = format!(
        r#"{{"match":{{"source_match_id":"{SRC_F}","outcome":"success"}},"players":[{{"arma_id":"{ARMA_F}","role_played":"Squad Leader","deaths":1,"source_event_id":"{EV}"}}]}}"#
    );
    let nested = format!(
        r#"{{"match":{{"source_match_id":"{SRC_N}","outcome":"success"}},"players":[{{"arma_id":"{ARMA_N}","role_played":"Squad Leader","source_event_id":"{EV}","counters":{{"kills":0,"deaths":1,"team_kills":0,"longest_kill_m":0,"vehicles_destroyed":0,"is_command":false,"command_win":null}}}}]}}"#
    );
    let (st, r) = ingest(&reporter, &app, &flat).await;
    assert_refused_line(st, &r, 0, "deaths");
    let (st, r) = ingest(&reporter, &app, &nested).await;
    assert_eq!(st, StatusCode::OK, "reporter nested: {r}");
    assert_eq!(
        stored_lines(&pool, ARMA_F).await,
        0,
        "the flat line is not stored"
    );
    assert_eq!(
        read_score(&pool, ARMA_N).await,
        (0, 1, 0, 0, 0, false, None),
        "nested deaths must be stored"
    );

    wipe_ingest(&pool, ARMA_F, SRC_F).await;
    wipe_ingest(&pool, ARMA_N, SRC_N).await;
}
