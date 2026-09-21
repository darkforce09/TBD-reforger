//! `mk_ci` — the Makefile's CI / composite / map lane, as first-class xtask tasks.
//!
//! The task index. There is no root `Makefile`; this
//! module owns the CI lane: `ci-local`, `ci-local-schema`, `schema-validate`, `schema-codegen`,
//! `verify-citations`, `verify-coding-standards`, `verify-doc-layout`, `verify-editorconfig`, the
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
//! implementation of "what `verify-doc-layout` does", and both `cargo xtask ci verify-doc-layout`
//! and `cargo xtask ci ci-local` reach it through the identical call.
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
//! **Fail-opens closed, and named:** `verify-doc-layout`'s recipe is
//! `! find … 2>/dev/null | grep -q .`. That has two holes — `2>/dev/null` hides an unreadable
//! directory, and a missing `grep` (exit 127) reads as "no match", i.e. as a pass. The Rust port
//! walks the tree with [`verification_core::scan::walk_files`], which returns `NotRun` for an unreadable
//! directory, and matches in-process so there is no `grep` to be absent. A tree it could not read
//! is reported, not swallowed.
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
    /// Nine subprocesses become nine calls, and the sub-gate list cannot drift from the CLI's.
    Xtask {
        echo: &'static str,
        silent: bool,
        run: fn() -> anyhow::Result<u8>,
    },
    /// A Rust port of a shell recipe (`verify-doc-layout`). Always silent: the only recipe with
    /// this shape is `@`-prefixed, and the parity test pins that.
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

/// `verify-doc-layout`'s failure text. Names the trees the walk actually covers (`apps`,
/// `contracts_v2`, `assets_v2` — see `split_cmd::verify_doc_layout`), so the message and the
/// behaviour cannot disagree about where a `docs/` subtree is forbidden.
/// What the document-layout refusal prints: the trees markdown may not be committed under, and
/// where it belongs instead.
pub fn doc_layout_refusal() -> String {
    format!(
        "FORBIDDEN: markdown under apps/**/docs/, contracts_v2/**/docs/ or assets_v2/**/docs/ — \
         use {} instead",
        crate::core::repository_layout::documentation::LAYOUT_TARGET_DIR
    )
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

/* ───────────────────────────── verify-doc-layout (native) ───────────────────────────── */

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
use split_cmd::verify_doc_layout;

#[cfg(test)]
use split_cmd::{is_forbidden_doc, run_task_in, split_cmd};
