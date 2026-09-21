//! The CI schema job must stay on the FULL gate set, and the Class-R verify tasks must stay real.
//!
//! Without this tripwire, someone can revert the CI schema job to bare
//! `cargo run -p xtask -- schema validate` + citations and reopen the map-object-enums
//! hole while CI stays green. The task pins close hollow `true` / `echo PASS` smuggles on
//! `ci-local-schema`, `verify-mission-rest-size-limits`, and this gate's own invocation.
//!
//! ── WHAT THE PINS READ ───────────────────────────────────────────────────────────────────────
//!
//! | subject | pin |
//! |---|---|
//! | the ci.yml `schema` job | at least one `run:` is exactly `cargo xtask ci ci-local-schema` |
//! | `ci-local-schema` | its steps invoke both `schema-validate` and `verify-citations` |
//! | `verify-mission-rest-size-limits` | its step echoes the cargo verify line |
//! | `ci-local` | it echoes `cargo xtask verify ci-schema-parity` DIRECTLY |
//!
//! The pins read [`crate::commands::ci::task_runner::TASKS`] in process — the very table
//! `cargo xtask ci <task>` executes — so hollowing a row is the only way to hollow a task, and a
//! hollowed row fails here.
//!
//! ── THE SELF-PIN ─────────────────────────────────────────────────────────────────────────────
//!
//! There is deliberately no `verify-ci-schema-parity` row in `TASKS`: `ci-local` reaches this gate
//! through a direct `Step::Xtask` and never through `Step::Task`. A hollowed dispatcher therefore
//! cannot green the check that polices dispatch, and that one direct step is what the third pin
//! reads. CI and the wave gate invoke this gate the same way, directly.
//!
//! The wave gate facade and its linked `checkrun.rs` / `gate_dispatch.rs` implementations are both
//! examined: `gate_slice` and `cmd_gate` must each iterate `VERIFY_STEPS`, which must carry the
//! mission-rest-size-limits and ci-schema-parity rows. A hollow loop body (checkrun argv gone)
//! reds this gate.

use std::collections::HashSet;
use std::path::Path;

use anyhow::Result;
use regex::Regex;

/// The long cargo spelling of the mission REST size gate. This const and every call site change
/// together — see module docs.
#[cfg(test)]
const VERIFY_MISSION_REST_SIZE_LIMITS: &str =
    "cargo run -q -p xtask -- verify mission-rest-size-limits";
/// The long cargo spelling of this gate itself (the self-pin).
#[cfg(test)]
const VERIFY_CI_SCHEMA_PARITY: &str = "cargo run -q -p xtask -- verify ci-schema-parity";
/// The `TASKS` step echo `verify-mission-rest-size-limits` must carry — the in-table spelling of
/// [`VERIFY_MISSION_REST_SIZE_LIMITS`].
const TASK_ECHO_MISSION_REST_SIZE_LIMITS: &str = "cargo xtask verify mission-rest-size-limits";
/// The `TASKS` step echo `ci-local` must carry for this gate. THE SELF-PIN: `ci-local` must reach
/// it directly, not via a `verify-ci-schema-parity` row that could be hollowed.
const TASK_ECHO_CI_SCHEMA_PARITY: &str = "cargo xtask verify ci-schema-parity";

const CI_REL: &str = ".github/workflows/ci.yml";
const WAVE_REL: &str = "tools_v2/xtask/src/commands/platform/wave_execution/gate.rs";
/// Both gate paths iterate `VERIFY_STEPS`; these are the two rows that must be in it. Consts and
/// call sites are one atomic change.
const ROW_MISSION_REST_SIZE_LIMITS: &str =
    r#"("mission REST size limits", "mission-rest-size-limits")"#;
const ROW_CI_SCHEMA_PARITY: &str = r#"("CI schema parity", "ci-schema-parity")"#;
const VERIFY_LOOP: &str = "for (label, name) in VERIFY_STEPS";
const CHECKRUN_ARGV: &str = r#"&["cargo", "run", "-q", "-p", "xtask", "--", "verify", name]"#;
/// What the ci.yml `schema` job must run. `cargo xtask` is the `.cargo/config.toml` alias for
/// `cargo run --package xtask --`; `ci_run_is_good` accepts either spelling.
const GOOD_RUN: &str = "cargo xtask ci ci-local-schema";

#[cfg(test)]
#[path = "tests/schema_parity/tests.rs"]
mod tests;

mod source_audit;
pub use source_audit::verify_ci_schema_parity;

mod strip_yaml_hash_comments;
use strip_yaml_hash_comments::strip_yaml_hash_comments;

#[cfg(test)]
use source_audit::{ci_run_is_good, run_pins, task_pins};
