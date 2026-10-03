//! The repository's structural, language-ban, licensing and registry checks.
//!
//! **Role:** each public check reads a checkout, prints its report and returns its exit status:
//! the engine-layer and workspace-law gates over [`repository_laws`], the `@route` tag gate over
//! the API route tables and the Mission Creator's ORBAT coherency gate ([`architecture`]); the
//! shell, Python and Node bans and the file-length gate ([`language_bans`]); the upstream
//! code-leak gate over the licensed reference lanes ([`licensing`]); and the object registry
//! alias gate ([`registry`]). Its tests hold every tool crate, found by folder, to the tooling
//! dependency, structure and prose rules.
//!
//! **Position:** tier 2 of `tools/checks`, over `verification_core` (verdicts, scans, patterns),
//! `process_runner` (`git` and `cargo` children), `repository_laws` (the rules the gates print)
//! and `repository_layout` (the checkout root and the shared paths). The xtask binary's
//! `cargo xtask verify` verbs and its `ci` task table call these checks.
//!
//! **Signals & state:** none; every check reads the checkout and writes its report to stdout.
//!
//! **Invariants:** a check that could not read an input never reports a pass — it exits 2, or
//! returns an [`Error`]; the exit status and every printed line are the check's contract with
//! the CI logs and the wave driver that scrape them.

pub mod architecture;
mod error;
pub mod language_bans;
pub mod licensing;
pub mod prelude;
pub mod registry;

pub use error::{Error, Result};

#[cfg(test)]
#[path = "tests/tooling_dependency_boundaries.rs"]
mod tooling_dependency_boundaries;

#[cfg(test)]
#[path = "tests/tooling_prose_rules.rs"]
mod tooling_prose_rules;
