//! LIMIT/OFFSET paging over the content golden's players plus thirty tied players yields every
//! row exactly once, in the one order the ORDER BY whitelist specifies.
//!
//! Why the defect needs Postgres: a bounded (`LIMIT 2`) top-N sort and the full sort of the
//! next page are free to order tied rows differently, so without a total ORDER BY one row can
//! appear on two pages while another appears on none. Six golden players are too few for that:
//! a board that small is sorted the same way on every page, so the set comparison below could
//! not fail. The thirty tied players, equal on every ranking column, make each page's sort pick
//! among thirty equal rows, which is what lets a missing tie-breaker repeat and skip rows. The
//! whitelist's shape (every arm ends in the `lt.discord_id ASC` tie-breaker; nothing off-list
//! reaches ORDER BY) is pinned by the pure unit tests that stay next to the handler in
//! `crates/api/api_command_center/src/handlers/leaderboards.rs`.
//!
//! The board ranks every player in its database, so the test runs on an isolated database of
//! this binary ([`common::require_isolated_test_database_url`]): dropped, recreated and migrated
//! on first use, the database-name allow-list asserted on both the operator's name and the
//! derived one. The content golden and the tied players are applied on top here; the tied
//! players stay out of the content golden, whose leaderboard fixture they would change.
//!
//! Never a `skip:` — a missing `TEST_DATABASE_URL` is a FAIL. The whole point of this test is
//! the database; `cargo xtask db test-it` always exports it.

use crate::common;

use std::collections::BTreeSet;

use api_command_center::handlers::leaderboards::{LeaderboardQuery, get_leaderboards};
use api_configuration::configuration::Config;
use api_state::AppState;

use api_http_layer::middleware::AuthUser;
use axum::extract::{Query, State};
use axum::http::{StatusCode, Uri};
use axum::response::{IntoResponse, Json};
use serde_json::{Value, json};
use sqlx::PgPool;

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

/// The content golden, applied on top of this binary's migrated database. Every INSERT is
/// `ON CONFLICT … DO UPDATE` and §12 refreshes the view, so applying it is idempotent — also over
/// the `dev-login` row `common` primes (`…001`, a golden player too).
const CONTENT_GOLDEN: &str = include_str!("../../../api_database/seeds/content_golden.sql");
/// The six golden players with `match_player_stats` rows (content_golden §7).
const GOLDEN_PLAYERS: [&str; 6] = [
    "000000000000000001",
    "000000000000000002",
    "000000000000000003",
    "000000000000000004",
    "000000000000000005",
    "000000000000000006",
];
/// How many tied players [`TIED_PLAYERS_SQL`] adds. Each plays one match with the same line, so
/// all of them share one value in every category: kd 1.00, one mission, a 100 m longest kill, no
/// team kills and a command win rate of 0, the last two shared with golden players as well.
const TIED_PLAYERS: usize = 30;
/// Thirty members, `900000000000000001`…`900000000000000030`, each with one identical stat line
/// in golden match …f000-000000000001, and the view rebuilt over them.
const TIED_PLAYERS_SQL: &str = "
    INSERT INTO users (discord_id, username, discord_handle, avatar_url, arma_id, arma_character,
                       role, is_banned, created_at, updated_at)
    SELECT format('9%s', lpad(n::text, 17, '0')), format('Tied %s', n), '', '',
           format('tied-arma-%s', n), '', 'enlisted', false,
           '2026-07-01 00:00:00+00', '2026-07-01 00:00:00+00'
    FROM generate_series(1, 30) AS n
    ON CONFLICT (discord_id) DO NOTHING;
    INSERT INTO match_player_stats (match_id, discord_id, arma_id, role_played, kills, deaths,
                                    team_kills, longest_kill_m, vehicles_destroyed, is_command,
                                    command_win, source_event_id, created_at)
    SELECT '00000000-0000-4000-f000-000000000001', format('9%s', lpad(n::text, 17, '0')),
           format('tied-arma-%s', n), 'Rifleman', 5, 5, 0, 100, 0, false, NULL,
           'rf-evt-20260620-01', '2026-06-20 21:48:00+00'
    FROM generate_series(1, 30) AS n
    ON CONFLICT DO NOTHING;
    REFRESH MATERIALIZED VIEW leaderboard_totals;";
/// Page size: every tie of [`TIED_PLAYERS`] rows spans fifteen pages.
const PAGE: i64 = 2;
/// The route's largest page. The whole board has to fit in one, or the unpaged read the pages
/// are compared with would itself be a truncated page.
const LARGEST_PAGE: i64 = 50;

/// An isolated database of this binary, seeded with the content golden and the tied players:
/// the board ranks every player in the database, so it cannot share rows with the binary's other
/// tests. [`common::require_isolated_test_database_url`] has dropped, recreated and migrated it,
/// and refused any name outside the database-name allow-list. Returns its URL and an open pool.
async fn provision_golden_database() -> (String, PgPool) {
    let url = common::require_isolated_test_database_url("leaderboards");
    let pool = api_database::connect(&url)
        .await
        .unwrap_or_else(|e| panic!("connect to `{url}`: {e}"));
    sqlx::raw_sql(CONTENT_GOLDEN)
        .execute(&pool)
        .await
        .unwrap_or_else(|e| {
            panic!("apply crates/api/api_database/seeds/content_golden.sql to `{url}`: {e}")
        });
    sqlx::raw_sql(TIED_PLAYERS_SQL)
        .execute(&pool)
        .await
        .unwrap_or_else(|e| panic!("add the tied players to `{url}`: {e}"));
    (url, pool)
}

/// The handler ignores the bearer beyond requiring one.
fn bearer() -> AuthUser {
    AuthUser {
        session_claims: api_http_layer::authentication_primitives::Claims {
            sub: "paging-test".into(),
            sid: uuid::Uuid::new_v4().into(),
            iss: "tbd-reforger".into(),
            aud: "tbd-website".into(),
            iat: 0,
            exp: i64::MAX,
            role: "enlisted".into(),
            arma_linked: true,
        },
        membership_stale: false,
        membership_override_active: false,
        can_manage_membership_override: false,
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
        Ok(query(category, limit, offset)),
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

/// The category's ranking column on one wire row; `None` for SQL NULL (`kd_ratio`).
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

/// `(score DESC NULLS LAST, discord_id ASC)` — the order every arm specifies.
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
    let state = api_server::composition::application_state(
        pool,
        Config::for_tests(url, "paging-test-secret"),
    );

    // Acceptance 2: the whitelist is still the only source of ORDER BY text.
    let rejected = get_leaderboards(State(state.clone()), bearer(), Ok(query("bogus", PAGE, 0)))
        .await
        .expect_err("an unknown category must be rejected, never ordered by");
    assert_eq!(rejected.into_response().status(), StatusCode::BAD_REQUEST);

    for category in CATEGORIES {
        let whole = board(&state, category, LARGEST_PAGE, 0).await;
        let whole_ids: Vec<&str> = whole.iter().map(id).collect();
        let expected: BTreeSet<&str> = whole_ids.iter().copied().collect();
        assert_eq!(
            whole.len(),
            GOLDEN_PLAYERS.len() + TIED_PLAYERS,
            "{category}: the board is the golden and tied players, and it fits in one \
             {LARGEST_PAGE}-row read: {whole_ids:?}"
        );
        for player in GOLDEN_PLAYERS {
            assert!(
                expected.contains(player),
                "{category}: golden player {player} is not on the board: {whole_ids:?}"
            );
        }
        assert!(
            largest_tie(category, &whole) >= TIED_PLAYERS,
            "{category}: the board no longer ties the {TIED_PLAYERS} tied players, so paging it \
             proves nothing: {whole:?}"
        );

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
