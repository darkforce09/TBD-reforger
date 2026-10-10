//! Remote debugging: why a client cannot join the staging game server and what the staging
//! server's console log says.
//!
//! **Role:** the `cargo xtask debug` group ([`debug`]: the A2S, NDJSON and direct-join probes and
//! the fleet instance selection) and the `mod remote-logs` verdict ([`debug::remote_logs`]).
//! **Position:** a command crate of `tools/commands`, over `deploy_settings` (the staging host
//! and its folders), `deployment` (the staging fleet's instances, ports and units),
//! `process_runner` (ssh, sshpass, ping and curl) and `repository_root` (the checkout root).
//! The `debug` and `mod` groups of `xtask` call it.
//! **Signals & state:** none held; a command appends to the log file it names, writes and removes
//! its own temp files and reads the process environment.
//! **Invariants:** every command returns its exit code and never exits the process; a probe that
//! cannot reach the staging host is reported in its row, never as an error; no message echoes a
//! secret setting.

pub mod debug;
mod error;
pub mod prelude;

pub use debug::DebugCmd;
pub use error::{Error, Result};
