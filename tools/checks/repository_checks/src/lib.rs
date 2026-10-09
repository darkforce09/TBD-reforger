//! The repository's structural, language-ban, licensing and registry checks.
//!
//! **Role:** each public check reads a checkout, prints its report and returns its exit status:
//! the workspace-law gates over [`repository_laws`] and the wave gate's source readers
//! ([`architecture`]); the shell, Python and Node bans and the file-length advice
//! ([`language_bans`]); the upstream code-leak gate over the licensed reference lanes
//! ([`licensing`]); and the object registry alias gate ([`registry`]).
//!
//! **Position:** tier 2 of `tools/checks`, over `verification_core` (verdicts, scans, patterns),
//! `process_runner` (`git` and `cargo` children), `repository_laws` (the rules the gates print)
//! `repository_root` (the checkout root) and `repository_layout` (the shared paths). The xtask
//! binary's `cargo xtask verify` verbs and its `ci` task table call these checks.
//!
//! **Signals & state:** none; every check reads the checkout and writes its report to stdout.
//!
//! **Invariants:** a check that could not read an input never reports a pass — it exits 2, or
//! returns an [`Error`]; the exit status and every printed line are the check's contract with
//! the logs that scrape them.

pub mod architecture;
mod error;
pub mod language_bans;
pub mod licensing;
pub mod prelude;
pub mod registry;

pub use error::{Error, Result};
