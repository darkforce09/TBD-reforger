//! The Discord id range reserved for synthetic staging accounts.
//!
//! **Role:** names the reserved range, 9100000000000000000 to 9100000000000099999, decides whether
//! an id lies in it, and counts the accounts that hold one.
//!
//! **Position:** every run prints the census from [`count_reserved_accounts`]; the actor guard in
//! `guarded_context.rs` refuses a reserved id; the load population and fixture event subcommands
//! draw their synthetic ids from the range and delete only inside it.
//!
//! **Signals & state:** none; constants and one read.
//!
//! **Invariants:** a reserved id is the canonical decimal form of a number in the range, always 19
//! digits, so the numeric test [`is_reserved_discord_id`] and the SQL pattern
//! [`RESERVED_DISCORD_ID_PATTERN`] accept exactly the same strings. A Discord snowflake carries its
//! creation time in its top bits, and every id of the range encodes a time after 2080, so no real
//! Discord account holds one.

use sqlx::PgExecutor;

/// The first id of the reserved range.
pub(crate) const FIRST_RESERVED_DISCORD_ID: u64 = 9_100_000_000_000_000_000;
/// The last id of the reserved range.
pub(crate) const LAST_RESERVED_DISCORD_ID: u64 = 9_100_000_000_000_099_999;
/// The PostgreSQL regular expression that matches exactly the reserved ids: the 14 digits every
/// id of the range shares, then any five digits.
pub(crate) const RESERVED_DISCORD_ID_PATTERN: &str = "^91000000000000[0-9]{5}$";

/// Whether `candidate` is the canonical decimal form of an id inside the reserved range. A sign, a
/// leading zero or surrounding whitespace makes it a different string and so not reserved.
pub(crate) fn is_reserved_discord_id(candidate: &str) -> bool {
    let canonical =
        candidate.bytes().all(|byte| byte.is_ascii_digit()) && !candidate.starts_with('0');
    canonical
        && candidate
            .parse::<u64>()
            .is_ok_and(|id| (FIRST_RESERVED_DISCORD_ID..=LAST_RESERVED_DISCORD_ID).contains(&id))
}

/// The number of accounts whose Discord id lies in the reserved range.
pub(crate) async fn count_reserved_accounts<'e>(
    executor: impl PgExecutor<'e>,
) -> sqlx::Result<i64> {
    sqlx::query_scalar("SELECT count(*) FROM users WHERE discord_id ~ $1")
        .bind(RESERVED_DISCORD_ID_PATTERN)
        .fetch_one(executor)
        .await
}
