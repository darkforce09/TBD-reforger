//! The full validation suite and the one-file mission check.
//!
//! **Role:** declares `validate_all` and `validate_file` and re-exports their entries.
//! **Position:** private to the schema gates; the crate root re-exports both entries.
//! **Signals & state:** none.
//! **Invariants:** none beyond its two submodules'.
use super::*;

mod validate_all;
pub use validate_all::validate_all;

mod validate_file;
pub use validate_file::validate_file;
