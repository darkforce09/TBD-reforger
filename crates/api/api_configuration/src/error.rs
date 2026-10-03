//! Why the API's configuration cannot be used.
//!
//! **Role:** the crate's error type and its `Result` alias.
//! **Position:** [`crate::configuration::Config::load`] and the database pool settings refuse a
//! variable with a [`ConfigError`]; a caller that gathers configuration failures with others
//! converts it into an [`Error`] with `?`.
//! **Signals & state:** none; plain data.
//! **Invariants:** every refusal of this crate is an [`Error`] variant naming the variable, never a
//! silent fallback to a default.

use crate::configuration::ConfigError;

/// Why the API's configuration cannot be used.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A configuration variable was refused.
    #[error(transparent)]
    Configuration(#[from] ConfigError),
}

/// The result of a fallible call of this crate.
pub type Result<T> = std::result::Result<T, Error>;
