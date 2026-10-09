//! The database source check: `cargo xtask verify no-select-star`.
//!
//! **Role:** declares the gate that holds the API's query sources to explicit column lists on
//! tables with nullable columns.
//! **Position:** inside `database_operations`; the xtask binary's `verify` and `ci` groups call
//! it.
//! **Signals & state:** none.
//! **Invariants:** a gate's verdict is its exit code (0 held, 1 findings, 2 did not run), and a
//! gate that examined nothing fails.

pub mod sql_deserialization;
