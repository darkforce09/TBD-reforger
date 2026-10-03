//! Why the API's database cannot be opened or migrated.
//!
//! **Role:** the crate's error type and its `Result` alias.
//! **Position:** [`crate::connect`] and [`crate::connect_lazy`] refuse with a [`sqlx::Error`],
//! [`crate::migrate`] with a [`sqlx::migrate::MigrateError`] and the pool settings with an
//! `api_configuration` `ConfigError`; a caller that gathers them converts each into an [`Error`]
//! with `?`.
//! **Signals & state:** none; plain data.
//! **Invariants:** every refusal of this crate is an [`Error`] variant carrying its cause.

use api_configuration::configuration::ConfigError;

/// Why the API's database cannot be opened or migrated.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The pool could not be opened.
    #[error(transparent)]
    Connection(#[from] sqlx::Error),
    /// A migration could not be applied.
    #[error(transparent)]
    Migration(#[from] sqlx::migrate::MigrateError),
    /// A `TBD_DB_POOL_*` variable was refused.
    #[error(transparent)]
    PoolConfiguration(#[from] ConfigError),
}

/// The result of a fallible call of this crate.
pub type Result<T> = std::result::Result<T, Error>;
