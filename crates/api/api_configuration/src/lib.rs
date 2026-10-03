//! The API's runtime configuration and its process-wide shutdown signal.
//!
//! **Role:** [`configuration::Config`], every setting the API reads from the environment at boot
//! with its development defaults and boot-time validation, the trusted proxy networks it parses
//! ([`configuration::proxy_network`]), and [`process_lifecycle::process_shutdown`], the flag the
//! server raises when it is asked to stop.
//! **Position:** above `api_identifiers` (the Discord client and guild ids); the API's database,
//! middleware, observability and composition code read the configuration, and its event streams
//! wait on the shutdown signal.
//! **Signals & state:** [`configuration::Config`] is a plain value loaded once; the shutdown signal
//! is one process-global `tokio::sync::watch` channel.
//! **Invariants:** a required variable that is empty or malformed fails the boot with a
//! [`configuration::ConfigError`] naming it, never at first use; a begun shutdown never ends.

pub mod configuration;
mod error;
pub mod prelude;
pub mod process_lifecycle;

pub use error::{Error, Result};
