//! Remote debugging: why a client cannot join the staging game server, what the staging server's
//! console log says, and how a large mission-version upload behaves against a local API.
//!
//! **Role:** the `cargo xtask debug` group ([`debug`]: the A2S, NDJSON and direct-join probes and
//! the fleet instance selection), the `mod remote-logs` verdict ([`debug::remote_logs`]) and the
//! `cargo xtask repro` group ([`reproduction`]: the mission-version upload reproduction).
//! **Position:** a command crate of `tools/commands`, over `deploy_settings` (the staging host
//! and its folders), `deployment` (the staging fleet's instances, ports and units),
//! `process_runner` (ssh, sshpass, ping and curl) and `repository_root` (the checkout root).
//! The `debug`, `repro` and `mod` groups of `xtask` call it.
//! **Signals & state:** none held; a command appends to the log file it names, writes and removes
//! its own temp files and reads the process environment.
//! **Invariants:** every command returns its exit code and never exits the process; a probe that
//! cannot reach the staging host is reported in its row, never as an error; no message echoes a
//! secret setting.

pub mod debug;
mod error;
pub mod prelude;
pub mod reproduction;

pub use debug::DebugCmd;
pub use error::{Error, Result};
pub use reproduction::ReproCmd;
