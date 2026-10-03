//! The deployment source checks: `cargo xtask verify staging-compose-paths`.
//!
//! **Role:** declares the gate that holds the staging compose file, the Caddy site and the
//! staging deploy sources to one set of host paths.
//! **Position:** inside `deployment`; the xtask binary's `verify` and `ci` groups call it.
//! **Signals & state:** none.
//! **Invariants:** a gate's verdict is its exit code, and a gate that examined nothing fails.

pub mod staging_compose_paths;
