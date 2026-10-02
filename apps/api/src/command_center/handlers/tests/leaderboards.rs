//! Every whitelisted ORDER BY arm carries the `lt.discord_id ASC` tie-breaker, and nothing off
//! the whitelist ever reaches ORDER BY. Both are pure string pins.
//!
//! The DB-observable half — LIMIT/OFFSET paging over tied scores yielding every row exactly once,
//! in the one order the whitelist specifies — is `tests/leaderboards_paging.rs`. It belongs there
//! because it needs a provisioned database, and only `tests/common::require_test_database_url`
//! may read `TEST_DATABASE_URL`; `tests/common` is not reachable from a lib test.

use super::*;

/// Every category `order_clause` whitelists. Keep in step with its `match` — this pin checks
/// every arm's shape and `tests/leaderboards_paging.rs` pages every one of them.
const CATEGORIES: [&str; 5] = [
    "kd",
    "command_win",
    "missions",
    "longest_kill",
    "team_kills",
];
/// The tie-breaker. `leaderboard_totals` is built `WHERE discord_id IS NOT NULL` with a UNIQUE
/// index on it (migration 0014), so the column is a total order on its own.
const TIEBREAK: &str = ", lt.discord_id ASC";

#[test]
fn every_whitelist_arm_ends_with_the_tiebreaker() {
    for category in CATEGORIES {
        let clause = order_clause(category)
            .unwrap_or_else(|| panic!("`{category}` is a whitelisted category"));
        assert!(
            clause.ends_with(TIEBREAK),
            "ORDER BY arm for `{category}` has no tie-breaker: {clause:?}"
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
