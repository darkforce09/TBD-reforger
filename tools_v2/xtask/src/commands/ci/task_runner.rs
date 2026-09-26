//! `mk_ci` — the Makefile's CI / composite / map lane, as first-class xtask tasks.
//!
//! The task index. There is no root `Makefile`; this
//! module owns the CI lane: `ci-local`, `ci-local-schema`, `schema-validate`, `schema-codegen`,
//! `verify-citations`, `verify-coding-standards`, `verify-documentation`, `verify-editorconfig`, the
//! three `map-*` composites, `lfs-dem`, `lfs-sat`, `help`, `test`, `build`.
//!
//! ── 1. WHY A TABLE AND NOT SIXTEEN FUNCTIONS ────────────────────────────────────────────────
//!
//! `ci-local`, `ci-local-schema` and `rust-ci` are *sequences of other targets*. Written as
//! sixteen independent functions, a composite would have to re-list what its parts do, and the
//! copy rots — a target that reports success while its
//! recipe had been hollowed to `@true`. Here [`Step::Task`] names another row of [`TASKS`] and
//! the runner recurses into **the same** `run_task` the standalone command calls. A composite
//! therefore cannot drift from its parts or be hollowed independently of them: there is one
//! implementation of "what `verify-documentation` does", and both
//! `cargo xtask ci verify-documentation` and `cargo xtask ci ci-local` reach it through the
//! identical call.
//!
//! The same table is the source for [`help`] (so a task cannot exist and be undiscoverable) and
//! for [`schema_list_gates`] (so the wave driver's drift tripwire keeps an input with no Makefile
//! dies — see §4).
//!
//! ── 2. WHY SOME ROWS BELONG TO OTHER SLICES ─────────────────────────────────────────────────
//!
//! `ci-local` runs `rust-ci` and `ci-local-leptos`; `test` runs `rust-test`; `build` runs
//! `leptos-build`. Those targets are the build lane's and `rust-test-it` is the database lane's, and all
//! three slices are in flight on the same commit. Two options existed:
//!
//!   * stub them, and have `ci-local` report a green it did not earn — the defect this program
//!     exists to kill; or
//!   * carry the recipe here, marked [`Lane::Borrowed`], so the composite genuinely runs.
//!
//! The second, with a guard: the task tests parse the recipe text and assert every row's steps
//! reproduce that target's recipe **verbatim**, borrowed rows included. While the Makefile lives
//! they cannot drift; the lanes are proven equal, so composing them
//! is a deletion of duplicates, not a reconciliation of two guesses.
//!
//! [`Lane::Alias`] rows are different: `verify-no-python` and friends were *already* one-line
//! aliases for an existing `cargo xtask verify …`, so nothing is borrowed — the row just records
//! that the task name maps onto a command that exists.
//!
//! ── 3. MAKEFILE ODDITIES PRESERVED ON PURPOSE ───────────────────────────────────────────────
//!
//! * **The PATH prepend** (`Makefile:7`) is load-bearing, not decoration: `editorconfig-checker`
//!   lives in `~/go/bin`, which is on no default PATH. `apply_env` reproduces the prepend for
//!   every child. Dropping it would turn `verify-editorconfig` into "command not found" on a
//!   correct machine.
//! * **`CARGO_TARGET_DIR ?=`** points every linked worktree at the primary
//!   checkout's warm `target/`, and `?=` lets an operator/wave export win. Not reproducing it
//!   would silently split the 52 GB cache per worktree. The build lane owns the *assertion* half
//!   (`verify-cargo-target`); this is the derivation half, and the two should become one helper
//!   when the lanes merge.
//! * **`-podman …`** in `rust-test-it` (`Makefile:205`) ignores failure; [`Step::Shell`] keeps the
//!   `ignore_err` flag rather than "fixing" a deliberate tolerance.
//!
//! **make's own framing is NOT reproduced**, deliberately, and it is the one place where output
//! differs. GNU make prints `make[1]: Entering directory …` around every sub-make and collapses
//! **every** recipe failure to its own exit status 2 — the Makefile itself complains about that
//! flattening at `mod-compile-selftest` (`Makefile:286`), where a `case` had to be written to
//! recover the 1-vs-3 distinction make destroyed. This runner returns the **leaf's raw code**.
//! Acceptance diffs therefore normalise make's framing away and reconstruct the true rc from the
//! `make: *** [Makefile:NNN: t] Error N` line, which states it.
//!
//! ── 4. `xtask schema list-gates` ────────────────────────────────────────────────────────────
//!
//! `tools_v2/xtask/src/commands/platform/wave_execution/schema.rs` **parses the recipe's
//! `schema-validate` recipe** to cross-check `GATE_SCHEMA_VALIDATE_GATES`; that tripwire refuses
//! to report PASS when the parse comes back empty. Deleting the Makefile removes its input, so
//! the tripwire would go permanently red — or, worse, be quietly loosened. [`schema_list_gates`]
//! prints the set derived from the `schema-validate` row of [`TASKS`], i.e. from the code that
//! actually runs the gates, so the cross-check keeps a source that cannot be a stale second copy.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use verification_core::verdict::NotRun;

use crate::core::repository_root::find_repo_root;

/// Which lane a row belongs to. `cargo xtask help` prints the tag beside the row so an operator
/// can tell a gate step from a one-line wrapper at a glance.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lane {
    /// A CI row: this table owns both the name and the steps.
    Ci,
    /// A one-line wrapper on an existing `cargo xtask verify …` command.
    Alias,
    /// A build or database row, carried here so the CI composites genuinely run it.
    Borrowed,
}

/// One recipe line.
///
/// `silent` suppresses the echo: without it the runner prints the expanded line before running
/// it, and that echo is the operator's progress trace through an eleven-step gate.
pub enum Step {
    /// Recurse into the same [`TASKS`] row the standalone command runs.
    Task(&'static str),
    /// A child process. ONE datum: the make-expanded recipe line. It is both what gets echoed and
    /// what gets spawned ([`split_cmd`]), so the trace and the execution cannot disagree — a
    /// separate `argv` field is exactly the kind of second copy this module exists to avoid.
    /// No shell: none of this lane's own lines carry metacharacters or quoting, and
    /// `cmd_lines_are_shell_free` pins that.
    Cmd { line: &'static str, silent: bool },
    /// An **in-process** xtask leaf: the same function `cargo xtask <group> <cmd>` dispatches to.
    /// Each leaf runs as a function call, not a `cargo run` subprocess, and because the call
    /// targets the CLI's own function, a composite's sub-gate list cannot drift from the CLI's.
    Xtask {
        echo: &'static str,
        silent: bool,
        run: fn() -> anyhow::Result<u8>,
    },
    /// An in-process step that prints its own progress and returns its own exit status
    /// (`verify-editorconfig`, `verify-codegen-fresh`, `ci-chrome`, `editor-api-boot`). Never
    /// echoed: it has no command line of its own.
    Native { run: fn() -> i32 },
    /// A recipe line handed to `/bin/sh -c` verbatim — **only** for [`Lane::Borrowed`] rows.
    /// Not a port and not claimed as one: it is the same shell make ran, kept byte-faithful
    /// (including `sh: podman: not found`) until the owning slice ports it properly.
    Shell {
        script: &'static str,
        silent: bool,
        ignore_err: bool,
    },
}

pub struct Task {
    pub name: &'static str,
    /// The Makefile's `## ` text, verbatim — [`help`] reprints it in make's own column format.
    pub help: &'static str,
    pub group: &'static str,
    pub lane: Lane,
    pub steps: &'static [Step],
}

// The table lives next door, split at the data/behaviour seam to keep both files inside SIZE-1.
// It is pure data: `run_task` below is its only interpreter.
#[path = "task_definitions.rs"]
mod tasks;
pub use tasks::TASKS;

/* ─────────────────────────────────── the runner ─────────────────────────────────── */

/// Environment `cargo run` injects into this process and which must NOT reach a nested `cargo`.
///
/// MEASURED, this worktree, warm `target-ctr` (`/proc/<pid>/environ` of the inner cargo, make-side
/// vs xtask-side):
/// ```text
///   make test                            Compiling=0
///   cargo run -q -p xtask -- ci test     Compiling=13   <- hyper-rustls reqwest ring rustls
///   make test                            Compiling=13      rustls-platform-verifier rustls-webpki
///   ./target-ctr/debug/xtask ci test     Compiling=0       sqlx* tokio-rustls
///   make test                            Compiling=0
/// ```
/// The same binary launched directly costs nothing; launched through `cargo run` it makes the
/// backend's whole TLS stack thrash, and then `make` rebuilds it back — for as long as anyone
/// alternates the two. The env diff explains it: cargo exports `SSL_CERT_FILE`/`SSL_CERT_DIR`
/// (its own probed paths) and an `LD_LIBRARY_PATH` pointing into `target/debug`, and those are in
/// the build-script fingerprints of exactly that crate set.
///
/// The same hazard in a new coat — a shared 52 GB target dir where two invocations
/// disagree — and it arrives with the Makefile's replacement, so it is closed here rather than
/// left to be rediscovered as "xtask is slow". The `CARGO_PKG_*` block is stripped for the same
/// reason plus an obvious one: `CARGO_PKG_NAME=xtask` is a lie to the child.
const CARGO_RUN_INJECTED: &[&str] = &[
    "CARGO",
    "CARGO_BIN_NAME",
    "CARGO_CRATE_NAME",
    "CARGO_MANIFEST_DIR",
    "CARGO_MANIFEST_PATH",
    "CARGO_PRIMARY_PACKAGE",
    "DYLD_FALLBACK_LIBRARY_PATH",
    "LD_LIBRARY_PATH",
    "SSL_CERT_DIR",
    "SSL_CERT_FILE",
];

/* ──────────────────────────────── help / list-gates ──────────────────────────────── */

#[cfg(test)]
#[path = "tests/task_runner.rs"]
mod tests;

mod split_cmd;
pub use split_cmd::find;
pub use split_cmd::help;
pub use split_cmd::invoked_tasks;
pub use split_cmd::run;
pub use split_cmd::schema_list_gates;
pub use split_cmd::step_echo;
pub use split_cmd::validate_gate_names;

#[cfg(test)]
use split_cmd::{run_task_in, split_cmd};
