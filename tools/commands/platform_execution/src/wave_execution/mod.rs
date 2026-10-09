//! `cargo xtask platform wave`: the platform factory's wave driver.
//!
//! **Role:** creates slice worktrees, runs the tiered gates, lands slices on main, closes waves and
//! pushes. Three rules shape it. (1) One shared cargo target folder: every worktree builds into
//! `Ctx::cargo_target_dir`, because a cold per-worktree build of the workspace costs tens of
//! gigabytes. (2) Per-slice landing with no wave barrier: `land::cmd_land` merges each ready slice
//! as soon as its gate is green. (3) Tiered gates: a slice pays only the cheap gate
//! (`gate::gate_slice`); the full suite runs once per wave on merged main (`gate::cmd_gate`).
//!
//! **Position:** reached through `platform_dispatch` from `cargo xtask platform wave <command>`.
//! `cargo xtask mod wave` is the separate mod wave driver; the two share a shape and differ in
//! everything they build, so each lives under its own program group. This module holds `Ctx`, the
//! print macros, the run-lane stamp types and the command table; the submodules hold the commands.
//!
//! **Signals & state:** `Ctx` is resolved once at entry and `chdir`s the process to the repository
//! root, so the relative paths it carries (`Cargo.toml`, `.ai/tickets`, `.ai/artifacts/worktrees`)
//! mean the same thing at every call site; the step capture stack (`SINK`) is thread-local.
//!
//! **Invariants:** a failing command inside a function does not abort it, and each site where that
//! matters says so; an unreadable ticket registry reads as "not shipped" except where a caller must
//! tell "not shipped" from "cannot speak" (`base::wave_ledger_unshipped_at`, rc 3); `current_wave`
//! skips wave `0`, which holds landed generations; `status` exits 0 whether or not anything is
//! ready; an unknown command prints the usage header and exits 1; stdout is flushed before every
//! stderr write and before every child that inherits stdout, so a `2>&1` capture keeps the emitted
//! order.

use std::io::Write;
use std::path::{Path, PathBuf};

use crate::Result;

use repository_layout::build_output::{self, RUN_TARGET_SUBDIR};

pub(crate) mod archived_wave_plans;
pub(crate) mod base;
pub(crate) mod changed;
pub(crate) mod db;
pub(crate) mod gate;
pub(crate) mod host;
pub(crate) mod land;
pub(crate) mod ledger;
pub(crate) mod lock;
pub(crate) mod migrate;
pub(crate) mod push;
pub(crate) mod reclaim;
pub(crate) mod schema;
pub(crate) mod status;
pub(crate) mod test_cmd;
pub(crate) mod touch;
pub(crate) mod trunk;
/// The gate verdict receipt `land` refuses to merge without.
pub(crate) mod verdict;

/// The help an unknown `platform wave` subcommand prints: what the lifecycle is, the three
/// decisions that shape it, and every command it accepts.
pub(crate) fn unknown_help() -> String {
    format!(
        "# Platform wave lifecycle — the programmatic form of {}.\n{UNKNOWN_HELP_BODY}",
        repository_layout::PLATFORM_FACTORY_RUNBOOK
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
#   cargo xtask platform wave test --slice <ticket-id> -p frontend_application
#                                         # ad-hoc cargo test into a PER-SLICE private
#                                         # CARGO_TARGET_DIR. Never bare cargo test against
#                                         # the shared cache — that is how one worktree runs
#                                         # another's binary.
#   cargo xtask platform wave run -p api_server --bin api-server
#                                         # build AND LAUNCH into $CARGO_TARGET_DIR/run-main,
#                                         # stamped `tbd-built-from <sha> <checkout>`.
#                                         # Refuses from a worktree: run-main is main's.
#   cargo xtask platform wave land        # merge every ready slice (no barrier)"##;

/// The disjoint-set command `status` and `prep` both name in their output, so an operator can
/// re-run the collision analysis by itself.
pub(crate) const COLLIDE: &str = "cargo run -q -p xtask -- slice-collisions";

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
macro_rules! wprintln {
    () => { $crate::wave_execution::emit("\n") };
    ($($arg:tt)*) => { $crate::wave_execution::emit(&format!("{}\n", format_args!($($arg)*))) };
}

/// `printf '%s' …` — no trailing newline.
macro_rules! wprint {
    ($($arg:tt)*) => { $crate::wave_execution::emit(&format!("{}", format_args!($($arg)*))) };
}

/// `echo … >&2`, with stdout flushed first so a `2>&1` capture keeps bash's ordering. Inside a
/// step capture it lands in the same buffer as stdout, which is what `2>&1` means.
macro_rules! werr {
    ($($arg:tt)*) => { $crate::wave_execution::emit_err(&format!("{}\n", format_args!($($arg)*))) };
}

pub(crate) use {werr, wprint, wprintln};

/// Everything the driver resolves once, at entry.
///
/// [`Ctx::enter`] `set_current_dir`s to the repository root, which is what lets the relative
/// paths below (`Cargo.toml`, `.ai/tickets`, `.ai/artifacts/worktrees`) mean the same thing at every
/// call site without each one re-deriving a join.
pub(crate) struct Ctx {
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
    /// [`repository_root::find_repository_root`] walks up for the ticket ledger and errors
    /// instead.
    pub(crate) fn enter() -> Result<Ctx> {
        let root = repository_root::find_repository_root()?;
        std::env::set_current_dir(&root)?;

        // The committed lock IS the plan: one file, one writer. There is no env override, and
        // with the TSVs; the generation floor died with them — landed generations live in the
        // lock's wave 0, so waves 1+ are open work only.
        let plan = repository_layout::WAVE_LOCK.to_string();

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
        // SAFETY: single-threaded entry, before any child is spawned or thread started.
        #[allow(unsafe_code)]
        unsafe {
            std::env::set_var("CARGO_TARGET_DIR", &cargo_target_dir)
        };

        // Resolved AFTER the export above so [`resolve_run_target_dir`] reads the value
        // this driver just chose, and EXPORTED so a lane that never builds a `Ctx` (`platform
        // preflight`, `mk`'s api lane, the editor smoke) reads the same directory rather than
        // re-deriving a second formula that drifts from this one.
        let run_target_dir = resolve_run_target_dir(&main_root);
        // SAFETY: as the `CARGO_TARGET_DIR` export above — single-threaded entry, no child yet.
        #[allow(unsafe_code)]
        unsafe {
            std::env::set_var("TBD_RUN_TARGET_DIR", &run_target_dir)
        };

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
            registry: repository_layout::TICKETS_DIR.into(),
            worktrees: repository_layout::WORKTREES_DIR.into(),
            gate_timeout: host.timeout_secs,
            gate_trunk_target: envd(
                "TBD_GATE_TRUNK_TARGET",
                gate_folder(&main_root, build_output::GATE_TRUNK_SUBFOLDER),
            ),
            gate_trunk_dist: envd(
                "TBD_GATE_TRUNK_DIST",
                gate_folder(&main_root, build_output::GATE_FRONTEND_DIST_SUBFOLDER),
            ),
            gate_check_target: envd(
                "TBD_GATE_CHECK_TARGET",
                gate_folder(&main_root, build_output::GATE_CHECK_SUBFOLDER),
            ),
            gate_schema_target: envd(
                "TBD_GATE_SCHEMA_TARGET",
                gate_folder(&main_root, build_output::GATE_SCHEMA_SUBFOLDER),
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
// nobody knows to run it. [`cmd_run`] is the lane; [`crate::preflight`] is
// the enforcement.

// The one extra target dir under the shared cache, named for its owner (the MAIN checkout), is
// `RUN_TARGET_SUBDIR` in `repository_layout::build_output`.

/// `<main checkout>/target/<subfolder>`: the default location of a wave-gate step's private build
/// folder, each its own `CARGO_TARGET_DIR` (or trunk dist folder). The subfolder names live in
/// [`repository_layout::build_output`].
pub(crate) fn gate_folder(main_root: &Path, subfolder: &str) -> String {
    build_output::build_output_subfolder(main_root, subfolder)
        .display()
        .to_string()
}

/// The provenance file written beside a run binary. Contents are exactly `<sha> <path>`.
pub(crate) const RUN_STAMP_FILE: &str = "tbd-built-from";

/// Who built the binaries in a profile directory: the commit, and the checkout it was built from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RunStamp {
    /// `git rev-parse HEAD` in [`RunStamp::checkout`] at build time — full 40 hex, never short.
    pub sha: String,
    /// The absolute path of the checkout whose source was compiled.
    pub checkout: String,
}

impl RunStamp {
    /// `<sha> <path>` plus a newline. One line: this file is read by a human as often as by
    /// preflight.
    pub(crate) fn render(&self) -> String {
        format!("{} {}\n", self.sha, self.checkout)
    }

    /// The inverse, REFUSING anything it does not fully understand. Fail-closed: a stamp that
    /// parses loosely becomes a stamp that says "fine" about a file it did not read, which is
    /// the defect this module is about. Absent is a separate answer ([`read_run_stamp`] -> None)
    /// and preflight blocks on that too, so no shape of this file reads as green by accident.
    pub(crate) fn parse(text: &str) -> Option<RunStamp> {
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

mod flush;
pub(crate) use flush::capture_step;
pub(crate) use flush::emit;
pub(crate) use flush::emit_err;
pub(crate) use flush::flush;
pub(crate) use flush::git_stdout;
pub(crate) use flush::git_stdout_lossy;
pub(crate) use flush::read_run_stamp;
pub(crate) use flush::resolve_run_target_dir;
pub(crate) use flush::run;
pub(crate) use flush::short;
pub(crate) use flush::subject;
