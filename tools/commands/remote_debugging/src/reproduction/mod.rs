//! The `cargo xtask repro` group: the mission-version upload reproduction.
//!
//! **Role:** the module tree of the upload orchestrator ([`mission_version_upload`]) and the two
//! helpers it is built from ([`mission_request_bodies`]); [`ReproCmd`] and [`run`] are the
//! group's command line.
//! **Position:** under the crate root; the `repro` group of `xtask` calls it.
//! **Signals & state:** none held here.
//! **Invariants:** every command answers with its exit code; an error means it could not run.

pub mod mission_request_bodies;
pub mod mission_version_upload;
mod reproduction_command;
mod reproduction_dispatch;

pub use reproduction_command::ReproCmd;
pub use reproduction_dispatch::run;
