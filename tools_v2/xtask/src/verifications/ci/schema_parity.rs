//! T-468 / T-471 / T-472 / T-476 / T-486 / T-489 — the CI schema job must stay on the FULL gate
//! set, and the Class-R verify tasks must stay real (T-853 / T-881 port of
//! `scripts/mod/verify-t468-ci-schema-parity.sh`).
//!
//! Without this tripwire, someone can revert the CI schema job to bare
//! `cargo run -p xtask -- schema validate` + citations and reopen the map-object-enums
//! hole while CI stays green. The task pins close hollow `@true` / `echo PASS` /
//! `#fake` smuggles on `ci-local-schema`, `verify-t456`, and this gate's own invocation.
//!
//! ── T-853 MUTUAL PIN ─────────────────────────────────────────────────────────────────────────
//!
//! Pre-port, `bash_pins` required `^\t@?bash <exact-script-path>` for both verify-t456 and
//! verify-t468. Porting either alone fails the other on a CORRECT tree. Both bash pins were
//! dropped and re-pinned at the cargo spelling in ONE atomic change with the Makefile /
//! wave.sh / ci.yml call sites. `VERIFY_T456` / `VERIFY_T468` are the single source; test
//! fixtures derive from them (gate_t440 precedent).
//!
//! ── T-897: THE RECIPE BODIES MOVED TO `crate::commands::ci::task_runner::TASKS` ─────────────────────────────────────────
//!
//! Three pins read the root `Makefile`. T-897 deleted it — and this gate was FAIL-CLOSED on that
//! file, so it would have gone RED rather than quiet. Their successor is [`crate::commands::ci::task_runner::TASKS`],
//! the table `cargo xtask ci <task>` executes:
//!
//! | pinned in `Makefile` until T-897 | pinned in `TASKS` now |
//! |---|---|
//! | `ci-local-schema:` invokes `schema-validate` + `verify-citations` | that row's `Step::Task`s |
//! | `verify-t456:` recipe is exactly the cargo verify line | that row's step echo |
//! | `verify-t468:` recipe is exactly the cargo verify line (self-pin) | `ci-local`'s DIRECT `verify t468` step |
//!
//! The self-pin's subject MOVED rather than vanishing, and the T-489 circularity it exists for is
//! now preserved by construction: there is deliberately no `verify-t468` row in `TASKS`, so
//! `ci-local` reaches this gate through a direct `Step::Xtask` and never through
//! `Step::Task("verify-t468")`. A hollowed dispatcher therefore cannot green the check that
//! polices dispatch, and that one step is what the third pin reads.
//!
//! The wave gate facade and its linked `checkrun.rs` / `gate_dispatch.rs` implementations are
//! examined: both `gate_slice` and `cmd_gate`
//! iterate `VERIFY_STEPS`, which must carry the t456 and t468 rows (T-478 dual-path discipline).
//! A hollow loop body (checkrun argv gone) must RED this gate. T-902 deleted `wave.sh`.
//!
//! ── WHAT THE PORT REMOVES ────────────────────────────────────────────────────────────────────
//!
//! 1. **`python3`, entirely.** The script was a heredoc owning YAML comment strip + recipe
//!    pins. Ported in-process; the `scripts/python-inventory.txt` line dies with it.
//! 2. **`pin_out="$(…)" || pin_rc=$?` swallow.** A Python crash still surfaced via non-zero
//!    status, but stderr was discarded when the capture only kept stdout. Failures print
//!    directly here.
//!
//! T-489 circularity (preserved): CI / ci-local / wave invoke this gate *directly* (cargo),
//! never through a dispatch table row named after it. A hollowed indirection must not green
//! those callers.

use std::collections::HashSet;
use std::path::Path;

use anyhow::Result;
use regex::Regex;

/// Cargo spelling pinned into TASKS echoes / historical checkrun lines for t456. Const + call sites are one
/// atomic change — see module docs.
#[cfg(test)]
const VERIFY_T456: &str = "cargo run -q -p xtask -- verify t456";
/// Cargo spelling pinned into TASKS echoes / historical checkrun lines for t468 (self-pin).
#[cfg(test)]
const VERIFY_T468: &str = "cargo run -q -p xtask -- verify t468";
/// The `TASKS` step echo `verify-t456` must carry — the in-table spelling of `VERIFY_T456`.
const TASK_ECHO_T456: &str = "cargo xtask verify t456";
/// The `TASKS` step echo `ci-local` must carry for t468. THE SELF-PIN (T-486/T-489): `ci-local`
/// must reach this gate directly, not via a `verify-t468` row that could be hollowed.
const TASK_ECHO_T468: &str = "cargo xtask verify t468";

const CI_REL: &str = ".github/workflows/ci.yml";
const WAVE_REL: &str = "tools_v2/xtask/src/commands/platform/wave_execution/gate.rs";
/// T-902: both gate paths iterate this table. Const + call sites are one atomic change.
const ROW_T456: &str = r#"("T-456 REST size gate", "t456")"#;
const ROW_T468: &str = r#"("T-468 CI schema parity", "t468")"#;
const VERIFY_LOOP: &str = "for (label, name) in VERIFY_STEPS";
const CHECKRUN_ARGV: &str = r#"&["cargo", "run", "-q", "-p", "xtask", "--", "verify", name]"#;
/// What the ci.yml `schema` job must run. `cargo xtask` is the `.cargo/config.toml` alias for
/// `cargo run --package xtask --`; `ci_run_is_good` accepts either spelling.
const GOOD_RUN: &str = "cargo xtask ci ci-local-schema";

#[cfg(test)]
#[path = "tests/schema_parity/tests.rs"]
mod tests;

mod source_audit;
pub use source_audit::verify_t468;

mod strip_yaml_hash_comments;
use strip_yaml_hash_comments::strip_yaml_hash_comments;

#[cfg(test)]
use source_audit::{ci_run_is_good, run_pins, task_pins};
