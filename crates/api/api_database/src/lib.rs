//! The API's Postgres connection lifecycle and its schema.
//!
//! **Role:** opens the tuned connection pool with its startup retry budget ([`connect`],
//! [`connect_lazy`]), applies the migrations embedded from `migrations/` ([`migrate`]), reads the
//! pool tuning from `TBD_DB_POOL_*` ([`connection_pool`]), and classifies a failed query by its
//! SQLSTATE ([`postgres_errors`]). The crate folder also holds the development seeds
//! (`seeds/`) that `cargo xtask db seed` applies.
//! **Position:** above `api_configuration`, whose `ConfigError` names a malformed pool variable;
//! the API binary, the registry import tool, the services that map constraint violations and the
//! integration suites use it.
//! **Signals & state:** none held here; a pool owns its connections.
//! **Invariants:** a migration file is never edited once committed, since sqlx records its
//! checksum on every database that applied it; a malformed pool variable fails before any
//! connection attempt; a unique or foreign-key violation is recognised by SQLSTATE, never by
//! message text.

mod connection;
pub mod connection_pool;
mod error;
pub mod postgres_errors;
pub mod prelude;

pub use connection::{connect, connect_lazy, migrate};
pub use error::{Error, Result};
