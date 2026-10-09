//! The harness's read-only commands: preflight and status (and capacity).
//!
//! **Role:** declares one module per read-only command.
//!
//! **Position:** called by `staging_dispatch.rs`; host reads run through
//! `remote_observers/host_shell.rs`.
//!
//! **Signals & state:** none.
//!
//! **Invariants:** nothing here sends a command that changes the host.

pub(crate) mod host_capacity;
pub(crate) mod preflight;
pub(crate) mod status;
