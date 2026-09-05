//! T-311 — LIMIT/OFFSET paging over the T-194 content golden's 4-way ties yields every row
//! exactly once, in the one order the ORDER BY whitelist specifies.
//!
//! Why the defect needs Postgres: a bounded (`LIMIT 2`) top-N sort and the full sort of the
//! next page are free to order tied rows differently, so without a total ORDER BY one row can
//! appear on two pages while another appears on none. The whitelist's shape (every arm ends in
//! the `lt.discord_id ASC` tie-breaker; nothing off-list reaches ORDER BY) is pinned by the pure
//! unit tests that stay next to the handler in `src/handlers/telemetry/leaderboards.rs`.
//!
//! Why it lives in `tests/` and not in that file: the T-542/T-558 Class-R pin
//! (`common::t542_no_raw_test_database_url_reads_outside_common`) forbids a raw
//! `TEST_DATABASE_URL` read anywhere under `src/**` — only [`common::require_test_database_url`]
//! may read it, and `tests/common` is not reachable from a lib test. So this binary gets the
//! T-534 shape every other suite has: its own `<base>_leaderboards_paging_it` database, dropped
//! and recreated on first use, migrated, the T-381 allow-list asserted on both the operator's
//! name and the derived one. The content golden is applied on top here.
//!
//! Never a `skip:` — a missing `TEST_DATABASE_URL` is a FAIL. The whole point of this test is
//! the database; the wave gate and `cargo xtask db test-it` always export it.

mod common;

use std::collections::BTreeSet;

use axum::extract::{Query, State};
use axum::http::{StatusCode, Uri};
use axum::response::{IntoResponse, Json};
use serde_json::{Value, json};
use sqlx::PgPool;
use website_api::config::Config;
use website_api::db;
use website_api::handlers::telemetry::leaderboards::{LeaderboardQuery, get_leaderboards};
use website_api::middleware::AuthUser;
use website_api::state::AppState;

/// Every category `order_clause` whitelists. Keep in step with its `match` — and with the
/// `CATEGORIES` pin beside it in `leaderboards.rs`, which checks every arm's shape while this
/// test pages every one of them.
const CATEGORIES: [&str; 5] = [
    "kd",
    "command_win",
    "missions",
    "longest_kill",
    "team_kills",
];

/// The T-194 content golden, applied on top of this binary's migrated database. Every INSERT is
/// `ON CONFLICT … DO UPDATE` and §12 refreshes the view, so applying it is idempotent — also over
/// the `dev-login` row `common` primes (`…001`, a golden player too).
const CONTENT_GOLDEN: &str = include_str!("../seeds/content_golden.sql");
/// The six golden players with `match_player_stats` rows (content_golden §7) — the whole
/// board. Ties the paging crosses: `missions_played` = 2 for …001-…004, `team_kills` = 0 for
/// …001/…002/…004/…005, `command_win_rate` = 0 for …003-…006 (4-way each); `kd` and
/// `longest_kill` are tie-free by construction.
const GOLDEN_PLAYERS: [&str; 6] = [
    "000000000000000001",
    "000000000000000002",
    "000000000000000003",
    "000000000000000004",
    "000000000000000005",
    "000000000000000006",
];
/// Categories whose golden board carries a 4-way tie — the pin that keeps the paging test
/// from going vacuous if the seed ever changes.
const FOUR_WAY_TIED: [&str; 3] = ["missions", "team_kills", "command_win"];
/// Page size that splits every 4-way tie across at least two pages.
const PAGE: i64 = 2;

/// This binary's database, seeded with the content golden. [`common::require_test_database_url`]
/// has already dropped, recreated and migrated it (T-534) and refused any name outside the T-381
/// allow-list; a missing URL is a FAIL here, not a skip. Returns its URL and an open pool.
async fn provision_golden_database() -> (String, PgPool) {
    let url = common::require_test_database_url().unwrap_or_else(|| {
        panic!(
            "TEST_DATABASE_URL required — a missing DB URL is a FAIL, not a skip (T-311). \
             The wave gate exports it from ensure_gate_db; `cargo xtask db test-it` sets it \
             to rust_it; by hand: postgres://tbd:tbd@localhost:5434/<name>_it?sslmode=disable"
        )
    });
    let pool = db::connect(&url)
        .await
        .unwrap_or_else(|e| panic!("connect to `{url}`: {e}"));
    sqlx::raw_sql(CONTENT_GOLDEN)
        .execute(&pool)
        .await
        .unwrap_or_else(|e| panic!("apply seeds/content_golden.sql to `{url}`: {e}"));
    (url, pool)
}

/// The handler ignores the bearer beyond requiring one.
fn bearer() -> AuthUser {
    AuthUser {
        discord_id: GOLDEN_PLAYERS[0].into(),
        role: "admin".into(),
        arma_linked: true,
    }
}

/// `?category=…&limit=…&offset=…` through the real `Query` extractor — `LeaderboardQuery`'s
/// fields are private to the handler module, and the wire path is what the route runs anyway.
fn query(category: &str, limit: i64, offset: i64) -> Query<LeaderboardQuery> {
    let uri: Uri =
        format!("/api/v1/leaderboards?category={category}&limit={limit}&offset={offset}")
            .parse()
            .unwrap_or_else(|e| panic!("leaderboards URI for `{category}`: {e}"));
    Query::try_from_uri(&uri).unwrap_or_else(|e| panic!("parse `{uri}`: {e}"))
}

/// `GET /api/v1/leaderboards?category=…&limit=…&offset=…` through the real handler.
async fn board(state: &AppState, category: &str, limit: i64, offset: i64) -> Vec<Value> {
    let Json(body) = get_leaderboards(
        State(state.clone()),
        bearer(),
        query(category, limit, offset),
    )
    .await
    .unwrap_or_else(|e| {
        panic!("GET /leaderboards?category={category}&limit={limit}&offset={offset}: {e:?}")
    });
    assert_eq!(
        body["category"], category,
        "envelope echoes the category: {body}"
    );
    body["data"]
        .as_array()
        .cloned()
        .unwrap_or_else(|| panic!("`data` must be an array: {body}"))
}

fn id(row: &Value) -> &str {
    row["discord_id"]
        .as_str()
        .unwrap_or_else(|| panic!("row without discord_id: {row}"))
}

/// The category's ranking column on one wire row; `None` for SQL NULL (`kd_ratio`, T-397).
fn score(category: &str, row: &Value) -> Option<f64> {
    let column = match category {
        "kd" => "kd_ratio",
        "command_win" => "command_win_rate",
        "missions" => "missions_played",
        "longest_kill" => "longest_kill_m",
        "team_kills" => "team_kills",
        other => panic!("no ranking column for `{other}`"),
    };
    row[column].as_f64()
}

/// `(score DESC NULLS LAST, discord_id ASC)` — the order every arm specifies after T-311.
fn in_order(category: &str, a: &Value, b: &Value) -> bool {
    match (score(category, a), score(category, b)) {
        (Some(x), Some(y)) if x != y => x > y,
        (Some(_), None) => true,
        (None, Some(_)) => false,
        _ => id(a) < id(b),
    }
}

/// Size of the largest group of equal scores in `rows`.
fn largest_tie(category: &str, rows: &[Value]) -> usize {
    rows.iter()
        .map(|row| {
            rows.iter()
                .filter(|other| score(category, other) == score(category, row))
                .count()
        })
        .max()
        .unwrap_or(0)
}

#[tokio::test]
async fn paging_the_golden_ties_yields_every_row_exactly_once() {
    let (url, pool) = provision_golden_database().await;
    let state = AppState::new(pool, Config::for_tests(url, "t311-test-secret"));

    // Acceptance 2: the whitelist is still the only source of ORDER BY text.
    let rejected = get_leaderboards(State(state.clone()), bearer(), query("bogus", PAGE, 0))
        .await
        .expect_err("an unknown category must be rejected, never ordered by");
    assert_eq!(rejected.into_response().status(), StatusCode::BAD_REQUEST);

    for category in CATEGORIES {
        let whole = board(&state, category, 50, 0).await;
        let whole_ids: Vec<&str> = whole.iter().map(id).collect();
        let expected: BTreeSet<&str> = whole_ids.iter().copied().collect();
        for player in GOLDEN_PLAYERS {
            assert!(
                expected.contains(player),
                "{category}: golden player {player} is not on the board: {whole_ids:?}"
            );
        }
        if FOUR_WAY_TIED.contains(&category) {
            assert!(
                largest_tie(category, &whole) >= 4,
                "{category}: the golden no longer makes a 4-way tie, so paging it proves \
                 nothing: {whole:?}"
            );
        }

        // Acceptance 1: LIMIT 2 pages across the tie — every row exactly once, one order.
        let mut paged: Vec<Value> = Vec::new();
        let mut offset = 0;
        loop {
            let rows = board(&state, category, PAGE, offset).await;
            if rows.is_empty() {
                break;
            }
            assert!(
                rows.len() <= PAGE as usize,
                "{category}: page at offset {offset} overflowed LIMIT {PAGE}: {rows:?}"
            );
            for (i, row) in rows.iter().enumerate() {
                assert_eq!(
                    row["rank"],
                    json!(offset + i as i64 + 1),
                    "{category}: rank is the global position: {row}"
                );
            }
            paged.extend(rows);
            offset += PAGE;
            assert!(
                offset <= 100,
                "{category}: runaway paging past offset {offset}"
            );
        }
        let paged_ids: Vec<&str> = paged.iter().map(id).collect();
        let unique: BTreeSet<&str> = paged_ids.iter().copied().collect();
        assert_eq!(
            unique.len(),
            paged_ids.len(),
            "{category}: a row was repeated across LIMIT {PAGE} pages: {paged_ids:?}"
        );
        assert_eq!(
            unique, expected,
            "{category}: LIMIT {PAGE} pages skipped or invented rows — paged {paged_ids:?}, \
             whole board {whole_ids:?}"
        );
        assert_eq!(
            paged_ids, whole_ids,
            "{category}: page-by-page order is not the unpaged order"
        );
        for pair in paged.windows(2) {
            assert!(
                in_order(category, &pair[0], &pair[1]),
                "{category}: not in (score DESC NULLS LAST, discord_id ASC) order: {} then {}",
                pair[0],
                pair[1]
            );
        }
    }
}
