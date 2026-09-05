//! Leaderboards + player stats + SSE server-status — Rust port of `handlers/leaderboards.go`.

use std::convert::Infallible;

use async_stream::stream;
use axum::extract::{Path, Query, State};
use axum::http::HeaderName;
use axum::response::sse::{Event, Sse};
use axum::response::{IntoResponse, Json, Response};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::QueryBuilder;
use tokio::sync::broadcast::error::RecvError;
use uuid::Uuid;

use crate::error::ApiError;
use crate::handlers::load_user;
use crate::middleware::AuthUser;
use crate::models::ServerStatus;
use crate::state::AppState;

/// One ranked entry joined with the user's display info. Numeric MV columns are
/// cast (`::int8` / `::float8`) into the wire types.
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct LeaderboardRow {
    pub discord_id: String,
    pub username: String,
    pub avatar_url: String,
    pub kills: i64,
    pub deaths: i64,
    /// `NULL` when no `match_player_stats` row for this player has a measured `deaths`
    /// reading (T-397). Distinct from `0.0` (measured zero-death / flawless aggregate).
    pub kd_ratio: Option<f64>,
    pub team_kills: i64,
    pub longest_kill_m: i64,
    pub vehicles_destroyed: i64,
    pub missions_played: i64,
    pub command_wins: i64,
    pub command_win_rate: f64,
    #[sqlx(default)]
    pub rank: i64,
}

/// Whitelisted category → ORDER BY clause (avoids injection). Every arm ends in the
/// `lt.discord_id ASC` tie-breaker (T-311): the view is unique on `discord_id`, so equal
/// scores page deterministically — without it LIMIT/OFFSET over the 4-way ties in the
/// T-194 golden may repeat one row across pages and skip another.
fn order_clause(category: &str) -> Option<&'static str> {
    match category {
        "kd" => Some("lt.kd_ratio DESC NULLS LAST, lt.discord_id ASC"),
        "command_win" => Some("lt.command_win_rate DESC NULLS LAST, lt.discord_id ASC"),
        "missions" => Some("lt.missions_played DESC, lt.discord_id ASC"),
        "longest_kill" => Some("lt.longest_kill_m DESC, lt.discord_id ASC"),
        "team_kills" => Some("lt.team_kills DESC, lt.discord_id ASC"),
        _ => None,
    }
}

const LB_SELECT: &str = "SELECT lt.discord_id, COALESCE(u.username, '') AS username, COALESCE(u.avatar_url, '') AS avatar_url, \
    lt.kills::int8 AS kills, lt.deaths::int8 AS deaths, lt.kd_ratio::float8 AS kd_ratio, \
    lt.team_kills::int8 AS team_kills, lt.longest_kill_m::int8 AS longest_kill_m, \
    lt.vehicles_destroyed::int8 AS vehicles_destroyed, lt.missions_played::int8 AS missions_played, \
    lt.command_wins::int8 AS command_wins, lt.command_win_rate::float8 AS command_win_rate, \
    0::int8 AS rank ";

#[derive(Debug, Deserialize)]
pub struct LeaderboardQuery {
    category: Option<String>,
    q: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
}

/// `GET /api/v1/leaderboards` — ranked board for a category, searchable by name.
///
/// @route GET /api/v1/leaderboards
pub async fn get_leaderboards(
    State(state): State<AppState>,
    _u: AuthUser,
    Query(q): Query<LeaderboardQuery>,
) -> Result<Json<Value>, ApiError> {
    let category = q.category.as_deref().unwrap_or("kd").to_string();
    let Some(order) = order_clause(&category) else {
        return Err(ApiError::bad_request("unknown category"));
    };
    let limit = q.limit.filter(|&n| n > 0 && n <= 100).unwrap_or(20).min(50);
    let offset = q.offset.filter(|&n| n >= 0).unwrap_or(0);
    let search = q.q.as_deref().unwrap_or("").trim().to_string();

    // Dynamic ORDER BY comes only from the hardcoded whitelist; values are bound.
    let mut qb = QueryBuilder::new(LB_SELECT);
    qb.push("FROM leaderboard_totals lt JOIN users u ON u.discord_id = lt.discord_id AND u.deleted_at IS NULL WHERE u.username ILIKE ");
    qb.push_bind(format!("%{search}%"));
    qb.push(" ORDER BY ").push(order);
    qb.push(" LIMIT ")
        .push_bind(limit)
        .push(" OFFSET ")
        .push_bind(offset);

    let mut rows: Vec<LeaderboardRow> = qb
        .build_query_as()
        .fetch_all(&state.pool)
        .await
        .map_err(ApiError::from)?;
    for (i, row) in rows.iter_mut().enumerate() {
        row.rank = offset + i as i64 + 1;
    }
    Ok(Json(json!({ "category": category, "data": rows })))
}

/// `GET /api/v1/users/:discordId/stats` — one player's aggregate card.
///
/// @route GET /api/v1/users/:discordId/stats
pub async fn get_user_stats(
    State(state): State<AppState>,
    _u: AuthUser,
    Path(discord_id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let Some(user) = load_user(&state.pool, &discord_id).await? else {
        return Err(ApiError::not_found("user not found"));
    };

    let mut qb = QueryBuilder::new(LB_SELECT);
    qb.push("FROM leaderboard_totals lt JOIN users u ON u.discord_id = lt.discord_id WHERE lt.discord_id = ");
    qb.push_bind(&discord_id);
    let row: Option<LeaderboardRow> = qb
        .build_query_as()
        .fetch_optional(&state.pool)
        .await
        .map_err(ApiError::from)?;

    let stats = row.unwrap_or(LeaderboardRow {
        discord_id: user.discord_id.clone(),
        username: user.username.clone(),
        avatar_url: user.avatar_url.clone(),
        kills: 0,
        deaths: 0,
        kd_ratio: None,
        team_kills: 0,
        longest_kill_m: 0,
        vehicles_destroyed: 0,
        missions_played: 0,
        command_wins: 0,
        command_win_rate: 0.0,
        rank: 0,
    });

    Ok(Json(json!({
        "stats": stats,
        "total_operations": user.total_deployments,
        "attendance_rate": user.attendance_rate,
    })))
}

/// `GET /api/v1/servers/:id/status/stream` — SSE live server-status feed.
///
/// @route GET /api/v1/servers/:id/status/stream
pub async fn stream_server_status(
    State(state): State<AppState>,
    _u: AuthUser,
    Path(id): Path<String>,
) -> Response {
    let topic = format!("server:{id}");
    let mut rx = state.hub.subscribe(&topic);
    let pool = state.pool.clone();
    let uuid = Uuid::parse_str(&id).ok();

    let body = stream! {
        // Current snapshot first, so the client renders without delay.
        if let Some(sid) = uuid {
            let snap: Result<Option<ServerStatus>, _> = sqlx::query_as(
                "SELECT server_id, is_online, player_count, max_players, server_fps::float8 AS server_fps, uptime_seconds, current_match_id, COALESCE(ingame_time, '') AS ingame_time, COALESCE(ingame_weather, '') AS ingame_weather, COALESCE(updated_at, '0001-01-01 00:00:00+00'::timestamptz) AS updated_at FROM server_statuses WHERE server_id = $1",
            ).bind(sid).fetch_optional(&pool).await;
            if let Ok(Some(status)) = snap
                && let Ok(js) = serde_json::to_string(&status) {
                yield Ok::<Event, Infallible>(Event::default().data(js));
            }
        }
        loop {
            match rx.recv().await {
                Ok(bytes) => yield Ok(Event::default().data(String::from_utf8_lossy(&bytes))),
                Err(RecvError::Lagged(_)) => continue,
                Err(RecvError::Closed) => break,
            }
        }
    };

    (
        [(HeaderName::from_static("x-accel-buffering"), "no")],
        Sse::new(body),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    //! T-311 — every whitelisted ORDER BY arm carries the `lt.discord_id ASC` tie-breaker, and
    //! LIMIT/OFFSET paging over the T-194 content golden's 4-way ties yields every row exactly
    //! once, in the one order the whitelist specifies.
    //!
    //! Why a DB test lives in the handler file: the ticket's `owns` is this file alone, and the
    //! defect is only observable against Postgres — a bounded (`LIMIT 2`) top-N sort and the
    //! full sort of the next page are free to order tied rows differently, so without a total
    //! ORDER BY one row can appear on two pages while another appears on none. The harness below
    //! is the `tests/common` T-534 shape (one throwaway `<base>_<suite>_it` database, dropped and
    //! recreated per run, T-381 allow-list asserted on both names) because `tests/common` is not
    //! reachable from a lib test.

    use std::collections::BTreeSet;

    use axum::http::StatusCode;
    use sqlx::{AssertSqlSafe, Connection, PgConnection, PgPool};

    use super::*;
    use crate::config::Config;

    /// Every category `order_clause` whitelists. Keep in step with its `match` — the string pin
    /// checks all of them and the DB test pages all of them.
    const CATEGORIES: [&str; 5] = [
        "kd",
        "command_win",
        "missions",
        "longest_kill",
        "team_kills",
    ];
    /// The T-311 tie-breaker. `leaderboard_totals` is built `WHERE discord_id IS NOT NULL` with a
    /// UNIQUE index on it (migration 0014), so the column is a total order on its own.
    const TIEBREAK: &str = ", lt.discord_id ASC";

    /// The T-194 content golden, applied on top of a migrated database. Every INSERT is
    /// `ON CONFLICT … DO UPDATE` and §12 refreshes the view, so applying it is idempotent.
    const CONTENT_GOLDEN: &str = include_str!("../../../seeds/content_golden.sql");
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

    #[test]
    fn every_whitelist_arm_ends_with_the_tiebreaker() {
        for category in CATEGORIES {
            let clause = order_clause(category)
                .unwrap_or_else(|| panic!("`{category}` is a whitelisted category"));
            assert!(
                clause.ends_with(TIEBREAK),
                "ORDER BY arm for `{category}` has no tie-breaker (T-311): {clause:?}"
            );
            let primary = &clause[..clause.len() - TIEBREAK.len()];
            assert!(
                primary.starts_with("lt.") && !primary.ends_with(','),
                "ORDER BY arm for `{category}` must rank a `lt.` column before the tie-breaker: {clause:?}"
            );
        }
    }

    #[test]
    fn unknown_category_never_reaches_order_by() {
        for bad in [
            "",
            "bogus",
            "KD",
            "lt.kd_ratio DESC",
            "kd; DROP TABLE users",
        ] {
            assert!(
                order_clause(bad).is_none(),
                "{bad:?} must not be whitelisted"
            );
        }
    }

    /// `TEST_DATABASE_URL`, refused unless its database name is on the T-381 allow-list. A
    /// missing URL is a FAIL, not a skip — the whole point of this test is the database.
    fn base_database_url() -> String {
        let url = std::env::var("TEST_DATABASE_URL").unwrap_or_else(|_| {
            panic!(
                "TEST_DATABASE_URL required — a missing DB URL is a FAIL, not a skip (T-311). \
                 The wave gate exports it from ensure_gate_db; `cargo xtask db test-it` sets it \
                 to rust_it; by hand: postgres://tbd:tbd@localhost:5434/<name>_it?sslmode=disable"
            )
        });
        assert_safe_test_database(&database_name(&url), &url);
        url
    }

    /// Database name of a Postgres URL (`…/name?sslmode=…` → `name`).
    fn database_name(url: &str) -> String {
        let parsed =
            url::Url::parse(url).unwrap_or_else(|e| panic!("TEST_DATABASE_URL `{url}`: {e}"));
        parsed.path().trim_start_matches('/').to_string()
    }

    /// Mirror of `tests/common::is_safe_test_database_name` (T-381): never the live
    /// `tbd_reforger` — this test DROPs, CREATEs, migrates and seeds.
    fn assert_safe_test_database(name: &str, url: &str) {
        let safe = !name.is_empty()
            && name != "tbd_reforger"
            && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
            && (name == "rust_it"
                || name.starts_with("tbd_gate")
                || name.ends_with("_cold")
                || name.ends_with("_it")
                || name.ends_with("_probe"));
        assert!(
            safe,
            "refusing to target database `{name}` ({url}) — T-381 allow-list: rust_it, \
             tbd_gate*, *_cold, *_it, *_probe"
        );
    }

    /// One throwaway database for this binary, `<base>_leaderboards_lib_it`, dropped and
    /// recreated here so the verdict never depends on a previous run (the T-534 shape), then
    /// migrated and seeded with the content golden. Returns its URL and an open pool.
    async fn provision_golden_database() -> (String, PgPool) {
        let base_url = base_database_url();
        let derived = format!("{}_leaderboards_lib_it", database_name(&base_url));
        assert!(
            derived.len() <= 63
                && derived
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_'),
            "derived database name `{derived}` is not a bare identifier — it is interpolated \
             into DDL below"
        );
        let mut parsed = url::Url::parse(&base_url).expect("base URL parsed above");
        parsed.set_path(&derived);
        let derived_url: String = parsed.into();
        assert_safe_test_database(&derived, &derived_url);

        // Maintenance session on the (allow-listed) base: DDL over the simple protocol, as
        // tests/common does it. The name was asserted bare above, which is what makes
        // `AssertSqlSafe` honest here.
        let mut admin = PgConnection::connect(&base_url)
            .await
            .unwrap_or_else(|e| panic!("connect to `{base_url}` to create `{derived}`: {e}"));
        for ddl in [
            format!("DROP DATABASE IF EXISTS {derived} WITH (FORCE)"),
            format!("CREATE DATABASE {derived}"),
        ] {
            sqlx::raw_sql(AssertSqlSafe(ddl))
                .execute(&mut admin)
                .await
                .unwrap_or_else(|e| panic!("DROP/CREATE `{derived}`: {e}"));
        }
        admin.close().await.expect("close maintenance connection");

        let pool = crate::db::connect(&derived_url)
            .await
            .unwrap_or_else(|e| panic!("connect to `{derived_url}`: {e}"));
        crate::db::migrate(&pool)
            .await
            .unwrap_or_else(|e| panic!("migrate `{derived}`: {e}"));
        sqlx::raw_sql(CONTENT_GOLDEN)
            .execute(&pool)
            .await
            .unwrap_or_else(|e| panic!("apply seeds/content_golden.sql to `{derived}`: {e}"));
        (derived_url, pool)
    }

    /// The handler ignores the bearer beyond requiring one.
    fn bearer() -> AuthUser {
        AuthUser {
            discord_id: GOLDEN_PLAYERS[0].into(),
            role: "admin".into(),
            arma_linked: true,
        }
    }

    /// `GET /api/v1/leaderboards?category=…&limit=…&offset=…` through the real handler.
    async fn board(state: &AppState, category: &str, limit: i64, offset: i64) -> Vec<Value> {
        let query = LeaderboardQuery {
            category: Some(category.into()),
            q: None,
            limit: Some(limit),
            offset: Some(offset),
        };
        let Json(body) = get_leaderboards(State(state.clone()), bearer(), Query(query))
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
        let query = LeaderboardQuery {
            category: Some("bogus".into()),
            q: None,
            limit: Some(PAGE),
            offset: Some(0),
        };
        let rejected = get_leaderboards(State(state.clone()), bearer(), Query(query))
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
}
