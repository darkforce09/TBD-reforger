//! The harness's read-only commands: preflight, status (and capacity) and fingerprints.
//!
//! **Role:** declares one module per read-only command.
//!
//! **Position:** called by `staging_dispatch.rs`; host reads run through
//! `remote_observers/host_shell.rs`.
//!
//! **Signals & state:** none.
//!
//! **Invariants:** nothing here sends a command that changes the host.

pub(crate) mod fingerprints;
pub(crate) mod host_capacity;
pub(crate) mod preflight;
pub(crate) mod status;

#[cfg(test)]
#[path = "tests/support_commands.rs"]
mod support_command_tests;
