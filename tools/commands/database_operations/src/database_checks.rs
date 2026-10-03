//! The database source checks: `cargo xtask verify wiki-seeds`, `faction-library-seeds` and
//! `no-select-star`.
//!
//! **Role:** declares the three gates; each pins a seed file or the API's query sources to the
//! contract it must keep.
//! **Position:** inside `database_operations`; the xtask binary's `verify` and `ci` groups call
//! them; [`wiki_seeds`] and [`faction_library_seeds`] read the seed order in
//! `crate::local_database::SEEDS`.
//! **Signals & state:** none.
//! **Invariants:** a gate's verdict is its exit code (0 held, 1 findings, 2 did not run), and a
//! gate that examined nothing fails.

pub mod faction_library_seeds;
pub mod sql_deserialization;
pub mod wiki_seeds;
