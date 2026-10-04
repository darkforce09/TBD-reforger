//! The session items most pages name, for `use frontend_session::prelude::*;`.
//!
//! **Role:** re-exports the session store and the two content gates.
//! **Position:** a re-export list over the crate's own modules.
//! **Signals & state:** none.
//! **Invariants:** re-exports only; every item keeps its home module.

pub use crate::gates::{AdminGate, AuthGate};
pub use crate::store::AuthStore;
