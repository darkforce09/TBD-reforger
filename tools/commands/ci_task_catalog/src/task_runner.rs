//! The CI task table's runner: `cargo xtask ci <task>`, `cargo xtask help` and
//! `cargo xtask schema list-gates`.
//!
//! **Role:** interprets [`TASKS`] (the rows of `task_definitions.rs`): [`run`] runs one row's
//! steps, recursing into named rows, spawning command lines and calling in-process leaves;
//! [`help`] and [`schema_list_gates`] render the same table.
//! **Position:** the crate's centre; the xtask binary's `ci`, `help` and `schema list-gates` verbs
//! call it, and the wave driver reads the `schema-validate` row.
//! **Signals & state:** none held; each step's child inherits this process's stdio.
//! **Invariants:** a composite runs the very rows it names; the runner stops at the first red step
//! and returns that leaf's raw exit code; a step naming no row is refused, never skipped.
//!
//! A composite (`ci-local`, `ci-local-schema`, `rust-ci`) names its parts as [`Step::Task`] rows,
//! and the runner recurses into the same `run_task` the standalone command calls, so a composite
//! cannot drift from its parts. [`Lane::Borrowed`] rows carry the build and database lanes'
//! recipes so the composites run them; [`Lane::Alias`] rows wrap an existing
//! `cargo xtask verify …` command. Every child gets the `PATH` prepend that puts
//! `editorconfig-checker` (`~/go/bin`) on the path and, unless the caller exported one, the shared
//! `CARGO_TARGET_DIR` of the primary checkout, so linked worktrees share one warm cache.
//! [`schema_list_gates`] prints the `schema-validate` row's sub-gates, the list the wave driver's
//! schema step cross-checks its own against.

use std::io::Write;
use std::path::{Path, PathBuf};

use verification_core::verdict::NotRun;

use repository_root::find_repository_root;

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
    /// what gets spawned (`split_cmd`), so the trace and the execution cannot disagree — a
    /// separate `argv` field is exactly the kind of second copy this module exists to avoid.
    /// No shell: none of this lane's own lines carry metacharacters or quoting.
    Cmd {
        /// The recipe line, echoed and spawned.
        line: &'static str,
        /// Whether the echo is suppressed.
        silent: bool,
    },
    /// An **in-process** xtask leaf: the same function `cargo xtask <group> <cmd>` dispatches to.
    /// Each leaf runs as a function call, not a `cargo run` subprocess, and because the call
    /// targets the CLI's own function, a composite's sub-gate list cannot drift from the CLI's.
    Xtask {
        /// The command line the leaf's own verb is, echoed before the call.
        echo: &'static str,
        /// Whether the echo is suppressed.
        silent: bool,
        /// The leaf, which `cargo xtask <group> <cmd>` dispatches to as well.
        run: fn() -> crate::Result<u8>,
    },
    /// An in-process step that prints its own progress and returns its own exit status
    /// (`verify-editorconfig`, `verify-codegen-fresh`, `ci-chrome`, `editor-api-boot`). Never
    /// echoed: it has no command line of its own.
    Native {
        /// The step, returning its exit status.
        run: fn() -> i32,
    },
}

/// One row of [`TASKS`]: a named task, its help line, its `help` group, its lane and its steps.
pub struct Task {
    /// The task name `cargo xtask ci <name>` takes.
    pub name: &'static str,
    /// The Makefile's `## ` text, verbatim — [`help`] reprints it in make's own column format.
    pub help: &'static str,
    /// The `help` group the row is listed under.
    pub group: &'static str,
    /// Which lane the row belongs to.
    pub lane: Lane,
    /// The steps, run in order until the first red one.
    pub steps: &'static [Step],
}

/// The task table: every row of [`TASKS`] with its steps.
///
/// **Role:** the data half of the runner, split at the data/behaviour seam to keep both files
/// inside the file-length limit; `run_task` is its only interpreter.
/// **Position:** pulled in from `task_definitions.rs` by `#[path]`, with its step lists and the
/// zero-argument adapters of the in-process verifications in `task_definitions/`.
/// **Signals & state:** none; constant data.
/// **Invariants:** a composite names its parts as [`Step::Task`] rows, never as copies of their
/// steps; every echo is the exact command line the leaf's own verb is.
#[path = "task_definitions.rs"]
mod tasks;
pub use tasks::TASKS;
pub(crate) use tasks::run_database_test_suite;

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

mod split_cmd;
pub use split_cmd::find;
pub use split_cmd::help;
pub use split_cmd::run;
pub(crate) use split_cmd::run_derived_line;
pub use split_cmd::schema_list_gates;
pub use split_cmd::validate_gate_names;
