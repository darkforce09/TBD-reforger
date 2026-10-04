//! The transport types most pages name, for `use frontend_transport::prelude::*;`.
//!
//! **Role:** re-exports the error, the settled-fetch wrapper and the token provider seam.
//! **Position:** a re-export list over the crate's own modules.
//! **Signals & state:** none.
//! **Invariants:** re-exports only; every item keeps its home module.

pub use crate::client::Fetched;
pub use crate::error::Error;
pub use crate::token_provider::TokenProvider;
