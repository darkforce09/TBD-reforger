//! The types most callers name, for `use frontend_api_dtos::prelude::*;`.
//!
//! **Role:** re-exports every typed identifier and the role ladder.
//! **Position:** a re-export list over the crate's own modules.
//! **Signals & state:** none.
//! **Invariants:** re-exports only; every item keeps its home module.

pub use crate::identifiers::*;
pub use crate::role::Role;
