//! Why a map coordinate cannot be read.
//!
//! **Role:** the crate's error type and its `Result` alias.
//! **Position:** [`crate::grid_reference::parse_grid`] refuses a typed reference with a
//! [`GridParseError`]; a caller that gathers several coordinate failures converts it into an
//! [`Error`] with `?`.
//! **Signals & state:** none; plain data.
//! **Invariants:** every refusal of this crate is an [`Error`] variant, never a sentinel value.

use crate::grid_reference::GridParseError;

/// Why a map coordinate cannot be read.
#[derive(Clone, Copy, PartialEq, Eq, Debug, thiserror::Error)]
pub enum Error {
    /// A typed grid reference was refused.
    #[error(transparent)]
    GridReference(#[from] GridParseError),
}

/// The result of a fallible call of this crate.
pub type Result<T> = std::result::Result<T, Error>;
