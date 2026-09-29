//! `seed-load-population` and `clean-load-population`: the synthetic member accounts a staging
//! load run signs in with.
//!
//! **Role:** the two subcommands' parsers, and what they share: the run of reserved Discord ids a
//! population occupies and the synthetic profile of each account.
//!
//! **Position:** rows of the subcommand table in `main.rs`. `population_seeding` writes through the
//! services the API's own sign-in uses and hands the refresh tokens to `account_file`, whose file
//! the `staging load` harness hands to the developer-tools load engine; `population_cleanup`
//! deletes what the population and a load run leave behind, and undoes a failed seeding.
//!
//! **Signals & state:** none; plain values.
//!
//! **Invariants:** account `k` of a population holds Discord id `first + k`, and every id of a
//! [`SyntheticIdRange`] lies inside the reserved range of `reserved_accounts`, so nothing here
//! creates or deletes an account outside it.

mod account_file;
mod population_cleanup;
mod population_seeding;

pub(crate) use population_cleanup::parse as parse_clean;
pub(crate) use population_seeding::parse as parse_seed;

use crate::reserved_accounts::{FIRST_RESERVED_DISCORD_ID, LAST_RESERVED_DISCORD_ID};
use crate::tool_failure::ToolFailure;

/// A run of consecutive reserved Discord ids: account `k` holds `first + k`, for `k` below
/// `count`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SyntheticIdRange {
    first: u64,
    count: u32,
}

impl SyntheticIdRange {
    /// The `count` ids from `first`, refused unless all of them lie in the reserved range.
    pub(crate) fn new(first: u64, count: u32) -> Result<Self, ToolFailure> {
        if count == 0 {
            return Err(ToolFailure::refused("--accounts takes at least 1"));
        }
        let last = first.checked_add(u64::from(count) - 1);
        let inside = (FIRST_RESERVED_DISCORD_ID..=LAST_RESERVED_DISCORD_ID).contains(&first)
            && last.is_some_and(|last| last <= LAST_RESERVED_DISCORD_ID);
        if !inside {
            return Err(ToolFailure::refused(format!(
                "{count} accounts from id {first} leave the reserved range \
                 {FIRST_RESERVED_DISCORD_ID} to {LAST_RESERVED_DISCORD_ID}"
            )));
        }
        Ok(Self { first, count })
    }

    /// How many accounts the range holds.
    pub(crate) fn count(self) -> u32 {
        self.count
    }

    /// The first id.
    pub(crate) fn first(self) -> u64 {
        self.first
    }

    /// The last id.
    pub(crate) fn last(self) -> u64 {
        self.first + u64::from(self.count) - 1
    }

    /// The Discord id of account `index`, in its canonical decimal form.
    pub(crate) fn discord_id(self, index: u32) -> String {
        (self.first + u64::from(index)).to_string()
    }
}

/// The display name of synthetic account `index`, such as `Load Member 00042`.
pub(crate) fn synthetic_username(index: u32) -> String {
    format!("Load Member {index:05}")
}

/// The Discord handle of synthetic account `index`, such as `load-member-00042`.
pub(crate) fn synthetic_handle(index: u32) -> String {
    format!("load-member-{index:05}")
}

#[cfg(test)]
#[path = "tests/synthetic_id_range.rs"]
mod tests;
