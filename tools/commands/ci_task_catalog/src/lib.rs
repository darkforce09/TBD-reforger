//! The CI and build lanes of the repository tooling: `cargo xtask ci`, `cargo xtask help` and
//! `cargo xtask mk`.
//!
//! **Role:** [`task_runner`] holds the CI task table ([`task_runner::TASKS`]) and the runner that
//! interprets it, composites recursing into the same rows the standalone commands run;
//! [`build_lane`] holds the `mk` recipes; [`cargo_target_pin`] is the shared `CARGO_TARGET_DIR`
//! pin with its glibc stamp guard, and `ci_scratch_reclaim` the `mk reclaim-target-ci` body;
//! [`workspace_member_tests`] tests every member no dedicated task tests in one cargo run;
//! [`wasm32_lint_lane`] derives the packages the wasm32 lint covers; [`frontend_package_lane`] derives the frontend
//! family (the app and every crate under `crates/frontend`) its lines format, lint and test;
//! [`wave_gate_steps`] holds the `mk` helper commands the ticket manager's slice and wave gates run.
//! **Position:** tier 8 of `tools/commands`, over `repository_checks`, `schema_tooling`,
//! `database_operations`, `process_runner`, `repository_layout` and `verification_core`, and over
//! `map_asset_verification` for the map asset steps of the task table. The xtask binary's `ci`,
//! `help` and `mk` groups call it, and through `mk` the ticket manager's gate steps.
//! **Signals & state:** none held; every task reads the checkout afresh and spawns its children
//! with inherited stdio.
//! **Invariants:** a composite runs exactly the rows it names, never a copy of them; a step that
//! could not run is never reported as a pass; the runner returns the leaf's raw exit code.

pub mod api_package_lane;
pub mod build_lane;
pub mod cargo_target_pin;
mod chromium_install;
mod ci_scratch_reclaim;
mod editor_api;
mod error;
pub mod frontend_package_lane;
pub mod prelude;
pub mod task_runner;
pub mod wasm32_lint_lane;
pub mod wave_gate_steps;
pub mod workspace_member_tests;

pub use error::{Error, Result, cause_chain};
