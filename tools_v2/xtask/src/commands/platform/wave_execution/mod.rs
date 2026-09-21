//! `cargo xtask platform wave` — the platform factory.
//!
//! **This module IS the platform factory.** It creates worktrees, runs gates, merges slices to
//! main and pushes. A defect here does not fail a check — it merges dirty work or deletes
//! somebody's slice. Its comments record the incidents each rule exists for; a comment that says
//! MEASURED with a date is a measurement, not an opinion, and stays as written.
//!
//! ── THE THREE RULES THAT DEFINE THIS FILE ────────────────────────────────────────────────────
//!
//!   1. SHARED CARGO TARGET DIR. These slices are Rust, so without CARGO_TARGET_DIR every
//!      worktree starts a COLD build of a 609-crate workspace; the repo's own target/ is 52 GB.
//!      See [`Ctx::cargo_target_dir`].
//!   2. PER-SLICE LANDING, NO WAVE BARRIER. "Merge only when all three complete" costs most of a
//!      wave's wall clock for nothing. See [`land::cmd_land`].
//!   3. TIERED GATES. A slice pays only the cheap gate (~10 s); the expensive suite runs once per
//!      wave on merged main. See [`gate::gate_slice`] and [`gate::cmd_gate`].
//!
//! ── WHY `platform wave` AND NOT `wave` ───────────────────────────────────────────────────────
//!
//! `cargo xtask mod wave` is the MOD wave driver ([`crate::commands::mod_ops::wave_execution`]).
//! Two different programs with the same shape and different physics, so they get sibling names
//! under their own program groups rather than one of them squatting the bare verb. `platform`
//! already exists as a group (`platform slice-worktree`, `platform preflight`), which is where a
//! reader already looks.
//!
//! ── DELIBERATE BEHAVIOURS (pinned by tests — do not "fix") ───────────────────────────────────
//!
//! - A failing command mid-function does not abort it. Every place that matters is called out at
//!   its site.
//! - `is_shipped` answers "not shipped" for a registry it cannot read, parse, or that holds a
//!   ticket without an `id`. `wave_ledger_unshipped_at` exists precisely because that answer is
//!   wrong for ITS caller — it must distinguish "not shipped" from "cannot speak" (rc 3).
//!   See [`ledger::Registry`].
//! - `plan_rows` filters comment rows, `wave` header rows and blank lines. The engine note lives
//!   on [`base::prev_wave_close`].
//! - `current_wave` skips wave `0` outright: landed generations live in the lock's wave 0, so
//!   waves 1+ are open work only. See [`ledger::current_wave`].
//! - `status` exits 0 whether or not anything is ready.
//! - An unknown command prints the usage header and exits 1.
//!
//! ── STREAM ORDERING ──────────────────────────────────────────────────────────────────────────
//!
//! Rust block-buffers stdout when it is a pipe, which would flush every stdout line AFTER the
//! stderr ones and reorder a `2>&1` capture. `werr` and [`flush()`] keep the emitted order:
//! flush stdout before writing to stderr, and before spawning any child that inherits our stdout.

use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::Result;

pub mod archived_wave_plans;
pub mod base;
pub mod changed;
pub mod db;
pub mod gate;
pub mod host;
pub mod land;
pub mod ledger;
pub mod lock;
pub mod migrate;
pub mod push;
pub mod reclaim;
pub mod schema;
pub mod status;
pub mod test_cmd;
pub mod touch;
pub mod trunk;
/// The gate verdict receipt `land` refuses to merge without.
pub mod verdict;

/// The help an unknown `platform wave` subcommand prints: what the lifecycle is, the three
/// decisions that shape it, and every command it accepts.
pub fn unknown_help() -> String {
    format!(
        "# Platform wave lifecycle — the programmatic form of {}.\n{UNKNOWN_HELP_BODY}",
        crate::core::repository_layout::documentation::PLATFORM_FACTORY_RUNBOOK
    )
}

/// Everything the help prints after its first line.
const UNKNOWN_HELP_BODY: &str = r##"#
# THREE DECISIONS THIS LIFECYCLE IS BUILT ON
# ------------------------------------------
# Slices here are Rust, gated on cargo and trunk. Each decision below is a measured
# consequence of that, not a preference:
#
#   1. SHARED CARGO TARGET DIR.  Without CARGO_TARGET_DIR every worktree starts a COLD build of
#      a 609-crate workspace, and this repository's own target/ is 52 GB. Eight cold worktrees
#      is a dead afternoon. Pointing every tree at one target dir makes cargo's lock serialise
#      builds instead — a warm `cargo check --workspace` is 6.8 s measured, so the wait is cheap
#      and the cache stays hot for every slice.
#
#   2. PER-SLICE LANDING, NO WAVE BARRIER.  A barrier that merges only when every slice is done
#      leaves finished slices blocked behind unfinished ones: measured at 89% of one wave's wall
#      clock, a mean of 64 minutes between merges that themselves take zero seconds. `land`
#      merges ANY slice that is committed, clean and gate-green, the moment it is ready.
#      `land --wave` restores the barrier when a wave genuinely needs one.
#
#   3. TIERED GATES.  A slice pays only the cheap gate (~10 s). The expensive suite runs once
#      per wave on merged main. `cargo xtask ci ci-local` is deliberately NOT a wave step: it
#      takes 15-40 minutes. The language gates it carries are wave steps in their own right
#      below, where they cost nothing.
#
#   cargo xtask platform wave status      # where are we? what is blocking?
#   cargo xtask platform wave prep        # create worktrees for the next disjoint set
#   cargo xtask platform wave gate        # full wave gate; base DERIVED from the last
#                                         # `wave N CLOSED` commit — pass one only to widen,
#                                         # a narrowing base is refused
#   cargo xtask platform wave gate --slice <ticket-id>   # cheap per-slice gate
#   cargo xtask platform wave test --slice <ticket-id> -p website-frontend
#                                         # ad-hoc cargo test into a PER-SLICE private
#                                         # CARGO_TARGET_DIR. Never bare cargo test against
#                                         # the shared cache — that is how one worktree runs
#                                         # another's binary.
#   cargo xtask platform wave run -p website-api --bin api
#                                         # build AND LAUNCH into $CARGO_TARGET_DIR/run-main,
#                                         # stamped `tbd-built-from <sha> <checkout>`.
#                                         # Refuses from a worktree: run-main is main's.
#   cargo xtask platform wave land        # merge every ready slice (no barrier)"##;

/// The disjoint-set command `status` and `prep` both name in their output, so an operator can
/// re-run the collision analysis by itself.
pub const COLLIDE: &str = "cargo run -q -p xtask -- slice-collisions";

// ── THE STEP CAPTURE, AND WHY IT HAS TO EXIST ───────────────────────────────────────────────────
//
// Both step runners capture a step's ENTIRE output — stdout AND stderr, whether the step is an
// external command or an in-process function (`wasm_changed`, `fmt_changed`, `gate_schema`, …) —
// then DISCARD it on PASS and print its last 15 lines, indented six spaces, on FAIL.
//
// A step that wrote straight to the process's stdout would therefore leak on every green step and
// print unindented on every red one. Every emit in this module goes through [`emit`], which routes
// into the active capture buffer when there is one. [`werr`] routes there too: inside a step,
// stderr is part of the captured text, not a separate stream.
thread_local! {
    static SINK: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// `echo …` — stdout, or the active step capture.
#[macro_export]
macro_rules! wprintln {
    () => { $crate::commands::platform::wave_execution::emit("\n") };
    ($($arg:tt)*) => { $crate::commands::platform::wave_execution::emit(&format!("{}\n", format_args!($($arg)*))) };
}

/// `printf '%s' …` — no trailing newline.
#[macro_export]
macro_rules! wprint {
    ($($arg:tt)*) => { $crate::commands::platform::wave_execution::emit(&format!("{}", format_args!($($arg)*))) };
}

/// `echo … >&2`, with stdout flushed first so a `2>&1` capture keeps bash's ordering. Inside a
/// step capture it lands in the same buffer as stdout, which is what `2>&1` means.
#[macro_export]
macro_rules! werr {
    ($($arg:tt)*) => { $crate::commands::platform::wave_execution::emit_err(&format!("{}\n", format_args!($($arg)*))) };
}

/// Everything the driver resolves once, at entry.
///
/// [`Ctx::enter`] `set_current_dir`s to the repository root, which is what lets the relative
/// paths below (`Cargo.toml`, `tools_v2/xtask/src`, `.ai/tickets`) mean the same thing at every
/// call site without each one re-deriving a join.
pub struct Ctx {
    /// `ROOT` — this checkout, which inside a worktree IS the worktree.
    pub root: PathBuf,
    /// The wave plan — `.ai/tickets/wave.lock`. Kept as a string because `status` prints it and
    /// `wave_plan_tickets_at` feeds it to `git show`; there is deliberately no env override —
    /// one committed lock, one writer.
    pub plan: String,
    /// The ticket ledger directory — `.ai/tickets`.
    pub registry: String,
    /// `WORKTREES` — `.ai/artifacts/worktrees`.
    pub worktrees: String,
    /// `MAIN_ROOT` — the PRIMARY checkout, from `git rev-parse --git-common-dir`.
    ///
    /// `root` above is this checkout, which inside a worktree IS the worktree, so deriving the
    /// shared target from it would point each slice at its own target and defeat the sharing
    /// entirely. `--git-common-dir` is shared by every worktree and points at the main repo's
    /// `.git`, so its parent is the main working tree.
    pub main_root: PathBuf,
    /// `CARGO_TARGET_DIR` — see rule 1. Exported into the environment for every child process.
    pub cargo_target_dir: String,
    /// The ONE extra target dir run-style lanes build into. Formula and rationale:
    /// `run_target_dir_for`.
    pub run_target_dir: String,
    /// `GATE_TIMEOUT` — applied by `host::Host::hostrun`, not by the step runner. Two reasons:
    /// `command -v` matches shell functions, so a run()-level wrapper tried to `timeout hostrun`
    /// and failed outright; and wrapping on this side kills the actual host process rather than
    /// just severing the bridge and orphaning a cargo build.
    pub gate_timeout: u64,
    /// The gate's PRIVATE trunk working set. Named here rather than buried in the call site,
    /// because [`trunk::gate_trunk_build`] asserts against them and the whole point is that these
    /// two are never the paths `trunk serve` owns.
    pub gate_trunk_target: String,
    pub gate_trunk_dist: String,
    /// The gate's PRIVATE dir for the ANALYSIS steps — `cargo check` (native + wasm32) and every
    /// clippy. Half of a two-part cure; the other half is `changed::touch_workspace`. Neither
    /// works alone.
    pub gate_check_target: String,
    /// The schema step's own dir, content-stamped. See [`schema::gate_schema`].
    pub gate_schema_target: String,
    /// `GATE_LOCK` / `GATE_LOCK_POLL` / `GATE_LOCK_MAX`.
    pub gate_lock: PathBuf,
    pub gate_lock_poll: u64,
    pub gate_lock_max: u64,
    /// `VERIFY_DEBT_NAG` — nag at 8, which is one wave's width.
    pub verify_debt_nag: i64,
    /// The host bridge, detected once.
    pub host: host::Host,
    /// The ticket ledger, parsed once with `is_shipped`'s exact failure semantics.
    pub registry_view: ledger::Registry,
}

impl Ctx {
    /// Resolve everything the driver needs, and `cd` to the repo root.
    ///
    /// REFUSE RATHER THAN GUESS A ROOT. A driver that guesses describes a directory that is not
    /// the repository and reports `open: 0 / 0 tickets` about it —
    /// [`crate::core::repository_root::find_repo_root`] walks up for the ticket ledger and errors
    /// instead.
    pub fn enter() -> Result<Ctx> {
        let root = crate::core::repository_root::find_repo_root()?;
        std::env::set_current_dir(&root)?;

        // The committed lock IS the plan: one file, one writer. There is no env override, and
        // with the TSVs; the generation floor died with them — landed generations live in the
        // lock's wave 0, so waves 1+ are open work only.
        let plan = ticket_engine::repository::WAVE_LOCK.to_string();

        // `git rev-parse --path-format=absolute --git-common-dir`, falling back to
        // `<root>/.git` when git cannot answer.
        let git_common = git_stdout(&["rev-parse", "--path-format=absolute", "--git-common-dir"])
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| root.join(".git").display().to_string());
        let main_root = Path::new(&git_common)
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| root.clone());

        let cargo_target_dir = std::env::var("CARGO_TARGET_DIR")
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| main_root.join("target").display().to_string());
        // Exported, and every hostrun forwards it explicitly because distrobox-host-exec does
        // NOT forward the environment (measured; see host.rs).
        unsafe { std::env::set_var("CARGO_TARGET_DIR", &cargo_target_dir) };

        // Resolved AFTER the export above so [`resolve_run_target_dir`] reads the value
        // this driver just chose, and EXPORTED so a lane that never builds a `Ctx` (`platform
        // preflight`, `mk`'s api lane, the editor smoke) reads the same directory rather than
        // re-deriving a second formula that drifts from this one.
        let run_target_dir = resolve_run_target_dir(&main_root);
        // SAFETY: as the `CARGO_TARGET_DIR` export above — single-threaded entry, no child yet.
        unsafe { std::env::set_var("TBD_RUN_TARGET_DIR", &run_target_dir) };

        let envd = |k: &str, dflt: String| -> String {
            std::env::var(k)
                .ok()
                .filter(|s| !s.is_empty())
                .unwrap_or(dflt)
        };
        let envn = |k: &str, dflt: i64| -> i64 {
            std::env::var(k)
                .ok()
                .and_then(|s| s.trim().parse().ok())
                .unwrap_or(dflt)
        };

        let host = host::Host::detect(envn("TBD_GATE_TIMEOUT", 1200).max(0) as u64);

        Ok(Ctx {
            plan,
            registry: ticket_engine::repository::TICKETS_DIR.into(),
            worktrees: crate::core::repository_layout::WORKTREES_DIR.into(),
            gate_timeout: host.timeout_secs,
            gate_trunk_target: envd(
                "TBD_GATE_TRUNK_TARGET",
                main_root.join("target-gate-trunk").display().to_string(),
            ),
            gate_trunk_dist: envd(
                "TBD_GATE_TRUNK_DIST",
                main_root.join("dist-gate-frontend").display().to_string(),
            ),
            gate_check_target: envd(
                "TBD_GATE_CHECK_TARGET",
                main_root.join("target-gate-check").display().to_string(),
            ),
            gate_schema_target: envd(
                "TBD_GATE_SCHEMA_TARGET",
                main_root.join("target-gate-schema").display().to_string(),
            ),
            gate_lock: PathBuf::from(envd(
                "TBD_GATE_LOCK",
                main_root
                    .join(verification_core::lock::GATE_LOCK_RELPATH)
                    .display()
                    .to_string(),
            )),
            gate_lock_poll: envn("TBD_GATE_LOCK_POLL", 30).max(1) as u64,
            gate_lock_max: envn("TBD_GATE_LOCK_MAX", 3600).max(0) as u64,
            verify_debt_nag: envn("TBD_VERIFY_DEBT_NAG", 8),
            registry_view: ledger::Registry::load_repo(Path::new(".")),
            host,
            cargo_target_dir,
            run_target_dir,
            main_root,
            root,
        })
    }
}

// ── THE RUN TARGET, AND THE PROVENANCE CARGO DOES NOT RECORD ─────────────────────────────────
//
// Rule 1 stays: one shared `CARGO_TARGET_DIR` for check, test and clippy, because a per-worktree
// target is ~44 GB. What it does NOT survive is a lane that LAUNCHES a binary.
//
// MEASURED 2026-09-06, two checkouts of one package into one target dir:
//
//     A. slice worktree builds first     Compiling tbdprobe v0.1.0 (…/slice-worktree)
//     B. main checkout builds second     Finished `dev` profile … in 0.00s     <- no Compiling
//     C. $CARGO_TARGET_DIR/debug/<bin>   UNMERGED-SLICE-CODE built_from=…/slice-worktree
//     D. `cargo run` from main           UNMERGED-SLICE-CODE built_from=…/slice-worktree
//     E. main's own source says          MERGED-MAIN-CODE
//
// Cargo's `-C metadata` hash omits the manifest path, so both checkouts write the SAME
// `deps/<name>-<hash>` and the same uplifted `<target>/<profile>/<bin>`; freshness is mtime-keyed,
// so main's build is satisfied by the worktree's artifact and never recompiles. That is how a dev
// server on :8080 comes to serve unmerged slice code — the signature defect: success over an input
// never examined. The cure is two-sided, because
// either half alone fails open: a SEPARATE target, so worktree check/test/clippy traffic into the
// shared cache can never satisfy a run lane's fingerprint (ONE extra directory, not one per
// worktree); and a STAMP naming the sha and the checkout, because a directory is trustworthy only
// if something recorded who wrote it — a `cargo clean -p <pkg>` cures it, and without a stamp
// nobody knows to run it. [`cmd_run`] is the lane; [`crate::commands::platform::preflight`] is
// the enforcement.

/// The one extra target dir, under the shared cache. Named for its owner: the MAIN checkout.
pub const RUN_TARGET_SUBDIR: &str = "run-main";

/// The provenance file written beside a run binary. Contents are exactly `<sha> <path>`.
pub const RUN_STAMP_FILE: &str = "tbd-built-from";

/// Who built the binaries in a profile directory: the commit, and the checkout it was built from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunStamp {
    /// `git rev-parse HEAD` in [`RunStamp::checkout`] at build time — full 40 hex, never short.
    pub sha: String,
    /// The absolute path of the checkout whose source was compiled.
    pub checkout: String,
}

impl RunStamp {
    /// `<sha> <path>` plus a newline. One line: this file is read by a human as often as by
    /// preflight.
    pub fn render(&self) -> String {
        format!("{} {}\n", self.sha, self.checkout)
    }

    /// The inverse, REFUSING anything it does not fully understand. Fail-closed: a stamp that
    /// parses loosely becomes a stamp that says "fine" about a file it did not read, which is
    /// the defect this module is about. Absent is a separate answer ([`read_run_stamp`] -> None)
    /// and preflight blocks on that too, so no shape of this file reads as green by accident.
    pub fn parse(text: &str) -> Option<RunStamp> {
        let line = text.lines().next()?.trim();
        let (sha, path) = line.split_once(' ')?;
        let path = path.trim();
        if path.is_empty() || sha.len() != 40 || !sha.chars().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }
        Some(RunStamp {
            sha: sha.to_ascii_lowercase(),
            checkout: path.to_string(),
        })
    }
}

#[cfg(test)]
#[path = "tests/module/run_target_tests.rs"]
mod run_target_tests;

/// Test support — PROCESS-GLOBAL cwd serialisation for tests that must chdir.
///
/// The close-ceremony tests run the marker authority ([`base::wave_close_number`],
/// [`base::wave_close_is_newest_wave`]) against fabricated scratch repos, and those functions
/// are cwd-bound by design (the driver chdirs once at [`Ctx::enter`]). `cargo test` is
/// multi-threaded and the cwd is process state, so every test that moves it must hold ONE lock —
/// otherwise a concurrent `find_repo_root()` (the scratch repos carry `.ai/tickets/ROOT`, which
/// is exactly what that function looks for) resolves inside somebody's scratch tree and a test
/// fails an assertion about a repo it was never meant to read.
#[cfg(test)]
#[path = "tests/module/testcwd.rs"]
pub(crate) mod testcwd;

mod flush;
pub use flush::capture_step;
pub use flush::emit;
pub use flush::emit_err;
pub use flush::flush;
pub use flush::git_stdout;
pub use flush::git_stdout_lossy;
pub use flush::read_run_stamp;
pub use flush::resolve_run_target_dir;
pub use flush::run;
pub use flush::short;
pub use flush::subject;

#[cfg(test)]
use flush::{run_lane_refusal, split_run_args};
#[cfg(test)]
pub(crate) use flush::{run_stamp_path, run_target_dir_for, write_run_stamp};
