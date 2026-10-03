//! The `cargo xtask debug` group and the `mod remote-logs` verdict.
//!
//! **Role:** the module tree of the server-join probes ([`probes`]), the direct-join report
//! ([`direct_join`]), the fleet instance selection (`staging_fleet_instance`) and the remote
//! console log verdict ([`remote_logs`]); [`DebugCmd`] and [`run`] are the group's command line.
//! **Position:** under the crate root; the `debug` and `mod` groups of `xtask` call it.
//! **Signals & state:** none held here.
//! **Invariants:** every command answers with its exit code; an error means it could not run.

mod debug_command;
mod debug_dispatch;
pub mod debug_log_ids;
pub mod direct_join;
pub mod probes;
pub mod remote_logs;
mod staging_fleet_instance;

pub use debug_command::DebugCmd;
pub use debug_dispatch::run;
