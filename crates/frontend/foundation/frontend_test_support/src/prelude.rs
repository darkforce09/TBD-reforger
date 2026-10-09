//! The helpers most frontend tests reach for, for `use frontend_test_support::prelude::*;`.
//!
//! **Role:** re-exports the captured-response macro and the repository file reads.
//! **Position:** a re-export list over the crate's own modules.
//! **Signals & state:** none.
//! **Invariants:** re-exports only; every item keeps its home module.

pub use crate::golden;
pub use crate::repository_root::{repository_path, repository_text};
