//! The Enfusion mod script checks.
//!
//! **Role:** each public check reads the mod's scripts or layouts from a checkout, prints its
//! report and returns its exit status: the in-code documentation card over the pinned script roots
//! ([`enfusion_comments`]), the source pins against false comments and the mission size bypass
//! ([`player_identity_comments`], [`results_reporter_identity_comments`],
//! [`destroy_target_diagnostics`], [`mission_rest_size_limits`]), the UI layout gate
//! ([`ui_layouts`]), and the Workbench-driven runs behind `cargo xtask mod`
//! ([`spawn_determinism`], [`spawn_verification`]).
//!
//! **Position:** tier 2 of `tools/checks`, over `verification_core` (verdicts, scans, patterns),
//! `process_runner` (the `cargo xtask mcp`, `ss`, `pkill` and `steam` children), `content_digest`
//! (the run digests), `repository_root` (the checkout root) and `repository_layout` (the shared
//! paths). The xtask binary's `cargo xtask verify` and `cargo xtask mod` verbs and its `ci` task
//! table call these checks.
//!
//! **Signals & state:** none; every check reads the checkout and writes its report to stdout. The
//! spawn runs drive a running Workbench and read its console log.
//!
//! **Invariants:** a check that could not read an input never reports a pass; every ban and pin
//! is proved on a perturbed copy that must fail; the exit status and every printed line are the
//! check's contract with the CI logs and the wave gates that scrape them.

pub mod destroy_target_diagnostics;
pub mod enfusion_comments;
mod enfusion_script_lexer;
mod error;
pub mod mission_rest_size_limits;
pub mod player_identity_comments;
pub mod prelude;
pub mod results_reporter_identity_comments;
pub mod spawn_determinism;
pub mod spawn_verification;
mod ui_layout_parser;
pub mod ui_layouts;

pub use error::{Error, Result};
