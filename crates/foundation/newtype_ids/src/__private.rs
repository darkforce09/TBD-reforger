//! The external crates the macro expansions name, re-exported for them.
//!
//! **Role:** lets an expansion write `$crate::__private::serde::Serialize` and
//! `$crate::__private::uuid::Uuid`, so the calling crate needs no `serde` or `uuid` dependency.
//! **Position:** reached only from the expansions of [`crate::string_id!`],
//! [`crate::integer_id!`] and [`crate::uuid_id!`]; hidden from the documentation.
//! **Signals & state:** none; two re-exports.
//! **Invariants:** not a public interface: a caller never names this module itself.

pub use serde;
pub use uuid;
