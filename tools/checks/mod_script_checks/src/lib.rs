//! The Enfusion mod script checks.
//!
//! **Role:** each public check reads the mod's layouts or drives a Workbench from a checkout,
//! prints its report and returns its exit status: the UI layout gate ([`ui_layouts`]) and the
//! Workbench-driven runs behind `cargo xtask mod` ([`spawn_determinism`],
//! [`spawn_verification`]).
//!
//! **Position:** tier 2 of `tools/checks`, over `verification_core` (verdicts, scans, patterns),
//! `process_runner` (the `cargo xtask mcp`, `ss`, `pkill` and `steam` children), `content_digest`
//! (the run digests), `repository_root` (the checkout root) and `repository_layout` (the shared
//! paths). The xtask binary's `cargo xtask verify` and `cargo xtask mod` verbs call these checks.
//!
//! **Signals & state:** none; every check reads the checkout and writes its report to stdout. The
//! spawn runs drive a running Workbench and read its console log.
//!
//! **Invariants:** a check that could not read an input never reports a pass; the exit status and
//! every printed line are the check's contract with the logs that scrape them.

mod error;
pub mod prelude;
pub mod spawn_determinism;
pub mod spawn_verification;
mod ui_layout_parser;
pub mod ui_layouts;

pub use error::{Error, Result};
