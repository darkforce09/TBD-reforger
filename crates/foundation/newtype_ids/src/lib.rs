//! Macros that declare newtype identifiers in the calling crate.
//!
//! **Role:** [`string_id!`], [`integer_id!`] and [`uuid_id!`] each declare a tuple struct around
//! one inner value (a `String`, an integer type the caller names, a `Uuid`) with the derives,
//! constructors, accessors and conversions an identifier needs, so an id is a type of its own
//! rather than a bare primitive that any other id converts into silently.
//! **Position:** foundation tier, depending on `serde` and `uuid` only, which the expansions name
//! through the hidden `__private` re-export so a calling crate needs neither dependency. The
//! optional first arm `sqlx,` adds `#[derive(::sqlx::Type)] #[sqlx(transparent)]`, resolved in
//! the calling crate: this crate never depends on sqlx.
//! **Signals & state:** none; the macros expand to plain data types.
//! **Invariants:** an identifier serialises and deserialises exactly as its inner value
//! (`serde(transparent)`), compares, orders and hashes as its inner value, and a string id
//! borrows as `str`, so a `HashMap` keyed by the id is looked up by `&str`. The expansions name
//! serde as `newtype_ids::__private::serde`, so a caller depends on this crate under its own name.

mod integer_ids;
pub mod prelude;
mod string_ids;
mod uuid_ids;

#[doc(hidden)]
pub mod __private;
