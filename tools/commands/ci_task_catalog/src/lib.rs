//! The CI and build lanes of the repository tooling: `cargo xtask ci`, `cargo xtask help` and
//! `cargo xtask mk`.
//!
//! **Role:** [`task_runner`] holds the CI task table ([`task_runner::TASKS`]) and the runner that
//! interprets it, composites recursing into the same rows the standalone commands run;
//! [`build_lane`] holds the `mk` recipes; [`cargo_target_pin`] is the shared `CARGO_TARGET_DIR`
//! pin with its glibc stamp guard, policed by the `mk` targets of `cargo_target_verification`;
//! [`workflow_checks`] holds the `verify ci-shell` and `verify ci-schema-parity` gates;
//! [`workspace_member_tests`] tests every member no dedicated task tests; [`wasm32_lint_lane`]
//! derives the packages the wasm32 lint covers.
//! **Position:** tier 8 of `tools/commands`, over the check crates (`repository_checks`,
//! `mod_script_checks`, `documentation_checks`), `schema_tooling`, `database_operations`,
//! `deployment`, `process_runner`, `repository_layout` and `verification_core`, and over
//! `map_asset_verification` for the map asset steps of the task table. The xtask binary's `ci`,
//! `help` and `mk` groups and its wave driver call it; the binary hands [`task_runner::run`]
//! its clap command tree for the in-process link check, so this crate never reads the command line.
//! **Signals & state:** one process-wide cell holds the command tree the binary handed the runner;
//! every task reads the checkout afresh and spawns its children with inherited stdio.
//! **Invariants:** a composite runs exactly the rows it names, never a copy of them; a step that
//! could not run is never reported as a pass; the runner returns the leaf's raw exit code.

pub mod build_lane;
pub mod cargo_target_pin;
mod cargo_target_verification;
mod chromium_install;
mod editor_api;
mod error;
pub mod prelude;
pub mod task_runner;
pub mod wasm32_lint_lane;
pub mod workflow_checks;
pub mod workspace_member_tests;

pub use error::{Error, Result, cause_chain};
