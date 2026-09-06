//! T-853 — port of `scripts/platform/wave.sh` → `cargo xtask platform wave`.
//!
//! **This module IS the platform factory.** It creates worktrees, runs gates, merges slices to
//! main and pushes. A wrong port here does not fail a check — it merges dirty work or deletes
//! somebody's slice. The bash it replaces encodes 63 commits of measured corrections, and its
//! comments are the record of incidents that already happened, so they are carried over VERBATIM
//! rather than summarised. When a comment says MEASURED with a date, that measurement was taken
//! against the bash and is reproduced here unchanged; do not "modernise" the prose.
//!
//! ── THE THREE CORRECTIONS THAT DEFINE THIS FILE ──────────────────────────────────────────────
//!
//! From the bash header, and each is a measured correction to how T-181 ran — not a preference:
//!
//!   1. SHARED CARGO TARGET DIR. The mod slices were Enfusion `.c`, so worktrees cost nothing.
//!      These slices are Rust. Without CARGO_TARGET_DIR every worktree starts a COLD build of a
//!      609-crate workspace; the repo's own target/ is 52 GB. See [`Ctx::cargo_target_dir`].
//!   2. PER-SLICE LANDING, NO WAVE BARRIER. T-181's rule "merge only when all three complete"
//!      cost 89% of its wall clock. See [`land::cmd_land`].
//!   3. TIERED GATES. A slice pays only the cheap gate (~10 s); the expensive suite runs once per
//!      wave on merged main. See [`gate::gate_slice`] and [`gate::cmd_gate`].
//!
//! ── WHY `platform wave` AND NOT `wave` ───────────────────────────────────────────────────────
//!
//! `cargo xtask mod wave` is the MOD wave driver ([`crate::mod_wave`], T-890 port of
//! `scripts/mod/wave.sh`). Two different programs with the same shape and different physics —
//! the bash says so in its own header — so they get sibling names under their own program
//! groups rather than one of them squatting the bare verb. `platform` already exists as a group
//! (`platform slice-worktree`, `platform preflight`), and `wave.sh` itself shells out to
//! `cargo run -q -p xtask -- platform slice-worktree`, so this is where a reader already looks.
//!
//! ── PRESERVED BASH ODDITIES (reproduce, pin with a test, document — do not "fix") ────────────
//!
//! - `set -uo pipefail` **without `-e`**: a failing command mid-function does not abort it. Every
//!   place that mattered is called out at its site.
//! - `is_shipped` answers "not shipped" for a registry it cannot read, parse, or that holds a
//!   ticket without an `id`. That is a python `KeyError` reaching `2>/dev/null`, and
//!   `wave_ledger_unshipped_at` exists precisely because that answer is wrong for ITS caller.
//!   See [`ledger::Registry`].
//! - `plan_rows` filters `^#` and `^wave[[:space:]]` and blank lines — BRE, which means the same
//!   thing under ugrep and GNU grep. The engine note lives on [`base::prev_wave_close`].
//! - `current_wave` skips wave `0` outright. (The bash also skipped everything below a
//!   generation floor of 76; T-912.2 deleted the floor — landed generations live in the lock's
//!   wave 0 now, so waves 1+ are open work only. See [`ledger::current_wave`].)
//! - `status`'s `[ "$ready" -gt 0 ] && echo …` leaves a non-zero rc when nothing is ready, but
//!   the `echo` after it resets it, so `status` exits 0 either way.
//! - Unknown command prints `sed -n '2,40p' "$0"` — the historical header — and exits 1.
//! - The two `python3` sites (`is_shipped`, `wave_ledger_unshipped_at`) are gone; `serde_json` is
//!   an xtask dep. Their EXIT-CODE semantics are preserved exactly, which is the part that
//!   mattered: one treats unreadable as "not shipped", the other as "cannot speak" (rc 3).
//!
//! ── STREAM ORDERING ──────────────────────────────────────────────────────────────────────────
//!
//! bash writes each `echo` with its own `write(2)`, so stdout and stderr interleave in the order
//! they were emitted. Rust block-buffers stdout when it is a pipe, which would flush every stdout
//! line AFTER the stderr ones and reorder a `2>&1` capture. [`werr`] and [`flush`] exist to keep
//! the byte-for-byte contract: flush stdout before writing to stderr, and before spawning any
//! child that inherits our stdout.

use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::Result;

pub mod base;
pub mod changed;
pub mod db;
pub mod diff;
pub mod diff_arms;
pub mod diff_reclaim;
pub mod gate;
pub mod host;
pub mod land;
pub mod ledger;
pub mod legacy_plan;
pub mod lock;
pub mod migrate;
pub mod push;
pub mod reclaim;
pub mod schema;
pub mod status;
pub mod test_cmd;
pub mod touch;
pub mod trunk;

/// `sed -n '2,40p' "$0"` — the historical header, printed verbatim on an unknown command.
///
/// Embedded rather than read off disk: `wave.sh` is deleted at the end of this port, and a help
/// text that vanishes with its source is a help text that silently becomes empty.
pub const UNKNOWN_HELP: &str = r##"# Platform wave lifecycle — the programmatic form of docs/platform/PLATFORM_FACTORY.md.
#
# WHY THIS EXISTS SEPARATELY FROM scripts/mod/wave.sh
# ---------------------------------------------------
# Same shape, different physics. The mod program gates on the Enfusion compiler and a real
# headless game boot. This program gates on cargo and trunk. Three things had to change, and
# each is a measured correction to how T-181 ran — not a preference:
#
#   1. SHARED CARGO TARGET DIR.  The mod slices were Enfusion `.c`, so worktrees cost nothing.
#      These slices are Rust. Without CARGO_TARGET_DIR every worktree starts a COLD build of a
#      609-crate workspace; the repo's own target/ is 52 GB. Eight cold worktrees is not a slow
#      wave, it is a dead afternoon. Pointing every tree at one target dir means cargo's lock
#      serialises builds instead — and a warm `cargo check --workspace` is 6.8 s measured, so
#      the wait is cheap and the cache is hot for everyone.
#
#   2. PER-SLICE LANDING, NO WAVE BARRIER.  T-181's rule "merge only when all three complete"
#      cost 89% of its wall clock: mean 64 minutes between lands, on merges that take zero
#      seconds. Finished slices sat blocked behind unfinished ones. Here `land` merges ANY slice
#      that is committed, clean and gate-green, the moment it is ready. `land --wave` keeps the
#      old barrier behaviour if you ever actually want it.
#
#   3. TIERED GATES.  A slice pays only the cheap gate (~10 s). The expensive suite runs once per
#      wave on merged main. `cargo xtask ci ci-local` is deliberately NOT used: it is 15-40 minutes, not the
#      22.7 s the docs still claim. (It was ALSO red for weeks because verify-no-python failed on
#      scripts/mod/slice-collisions.py; T-620 ported both .py files to xtask and deleted them, so
#      that half is green now and `verify-no-python` is a wave-gate step in its own right below.)
#
#   cargo xtask platform wave status      # where are we? what is blocking?
#   cargo xtask platform wave prep        # create worktrees for the next disjoint set
#   cargo xtask platform wave gate        # full wave gate; base DERIVED from the last
#                                         # `wave N CLOSED` commit — pass one only to widen,
#                                         # never to narrow (T-602 refuses a narrowing base)
#   cargo xtask platform wave gate --slice T-190   # cheap per-slice gate
#   cargo xtask platform wave test --slice T-190 -p website-frontend
#                                         # ad-hoc cargo test into a PER-SLICE private
#                                         # CARGO_TARGET_DIR (T-742). Never bare cargo test
#                                         # against the shared cache — that is the
#                                         # cross-worktree false-binary class.
#   cargo xtask platform wave run -p website-api --bin api
#                                         # build AND LAUNCH into $CARGO_TARGET_DIR/run-main,
#                                         # stamped `tbd-built-from <sha> <checkout>` (T-300).
#                                         # Refuses from a worktree: run-main is main's.
#   cargo xtask platform wave land        # merge every ready slice (no barrier)
#
#   bash scripts/platform/wave.sh was deleted at T-902."##;

/// `COLLIDE` — the dispatch-set command `status` and `prep` both name.
///
/// T-620: was `scripts/platform/slice-collisions.py`. Ported to xtask byte-identically (default,
/// `--check` and `--repack` all diffed clean against the Python before it was deleted), because
/// the factory's own tooling was the last thing keeping `cargo xtask verify no-python` red.
pub const COLLIDE: &str = "cargo run -q -p xtask -- slice-collisions";

/// Flush stdout. Call before writing to stderr and before spawning an inheriting child — see the
/// STREAM ORDERING note in the module header.
pub fn flush() {
    let _ = std::io::stdout().flush();
}

// ── THE STEP CAPTURE, AND WHY IT HAS TO EXIST ───────────────────────────────────────────────────
//
// Both step runners are `out="$("$@" 2>&1)"`. The argument is sometimes an external command and
// sometimes a SHELL FUNCTION (`wasm_changed`, `fmt_changed`, `gate_schema`, …), and in both cases
// the step's entire output — stdout AND stderr — is captured, then DISCARDED on PASS and printed
// `tail -15 | sed 's/^/      /'` on FAIL.
//
// So a ported step that wrote straight to the process's stdout would leak on every green step and
// print unindented on every red one. Every emit in this module therefore goes through [`emit`],
// which routes into the active capture buffer when there is one. `2>&1` is why [`werr`] routes
// there too: inside a step, stderr is part of the captured text, not a separate stream.
thread_local! {
    static SINK: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// Write to the innermost active step capture, or to real stdout when no step is running.
pub fn emit(s: &str) {
    SINK.with(|c| {
        let mut b = c.borrow_mut();
        match b.last_mut() {
            Some(buf) => buf.push_str(s),
            None => {
                drop(b);
                print!("{s}");
            }
        }
    });
}

/// Write to the innermost active step capture, or to real stderr when no step is running.
pub fn emit_err(s: &str) {
    SINK.with(|c| {
        let mut b = c.borrow_mut();
        match b.last_mut() {
            Some(buf) => buf.push_str(s),
            None => {
                drop(b);
                flush();
                eprint!("{s}");
            }
        }
    });
}

/// `out="$(step 2>&1)"; rc=$?` — run `f` with its output captured.
pub fn capture_step<R>(f: impl FnOnce() -> R) -> (String, R) {
    SINK.with(|c| c.borrow_mut().push(String::new()));
    let r = f();
    let out = SINK.with(|c| c.borrow_mut().pop().unwrap_or_default());
    (out, r)
}

/// `echo …` — stdout, or the active step capture.
#[macro_export]
macro_rules! wprintln {
    () => { $crate::wave::emit("\n") };
    ($($arg:tt)*) => { $crate::wave::emit(&format!("{}\n", format_args!($($arg)*))) };
}

/// `printf '%s' …` — no trailing newline.
#[macro_export]
macro_rules! wprint {
    ($($arg:tt)*) => { $crate::wave::emit(&format!("{}", format_args!($($arg)*))) };
}

/// `echo … >&2`, with stdout flushed first so a `2>&1` capture keeps bash's ordering. Inside a
/// step capture it lands in the same buffer as stdout, which is what `2>&1` means.
#[macro_export]
macro_rules! werr {
    ($($arg:tt)*) => { $crate::wave::emit_err(&format!("{}\n", format_args!($($arg)*))) };
}

/// Everything the bash set up at load time, resolved once.
///
/// The bash `cd "$ROOT"` and then used relative paths (`Cargo.toml`, `Makefile`, `xtask/src`,
/// `.ai/tickets/registry.json`) throughout. [`Ctx::enter`] does the same `set_current_dir`, which
/// is what makes those relative paths mean the same thing here without rewriting every one of
/// them into a join — and rewriting them is exactly where a port drifts.
pub struct Ctx {
    /// `ROOT` — this checkout, which inside a worktree IS the worktree.
    pub root: PathBuf,
    /// The wave plan — `.ai/tickets/wave.lock` since T-912.2 (the TSVs and their plan-path env
    /// override are dead). Kept as a string because `status` prints it and
    /// `wave_plan_tickets_at` feeds it to `git show`; there is deliberately no env override —
    /// one committed lock, one writer.
    pub plan: String,
    /// `REGISTRY` — `.ai/tickets/registry.json`.
    pub registry: String,
    /// `WORKTREES` — `.ai/artifacts/worktrees`.
    pub worktrees: String,
    /// `MAIN_ROOT` — the PRIMARY checkout, from `git rev-parse --git-common-dir`.
    ///
    /// `$ROOT` is this script's own repo — which inside a worktree IS the worktree, so defaulting
    /// to `"$ROOT/target"` pointed each slice at its own target and defeated the entire
    /// mitigation. `--git-common-dir` is shared by every worktree and points at the main repo's
    /// `.git`, so its parent is the main working tree.
    pub main_root: PathBuf,
    /// `CARGO_TARGET_DIR` — see correction 1. Exported into the environment, as the bash did.
    pub cargo_target_dir: String,
    /// T-300 — the ONE extra target dir run-style lanes build into. Formula and rationale:
    /// [`run_target_dir_for`].
    pub run_target_dir: String,
    /// `GATE_TIMEOUT` — applied by [`host::Host::hostrun`], not by the step runner. Two reasons:
    /// `command -v` matches shell functions, so a run()-level wrapper tried to `timeout hostrun`
    /// and failed outright; and wrapping on this side kills the actual host process rather than
    /// just severing the bridge and orphaning a cargo build.
    pub gate_timeout: u64,
    /// The gate's PRIVATE trunk working set (T-396). Named here rather than buried in the call
    /// site, because [`trunk::gate_trunk_build`] asserts against them and the whole cure is that
    /// these two are never the paths `trunk serve` owns.
    pub gate_trunk_target: String,
    pub gate_trunk_dist: String,
    /// The gate's PRIVATE dir for the ANALYSIS steps — `cargo check` (native + wasm32) and every
    /// clippy. T-421. Half of a two-part cure; the other half is
    /// [`changed::touch_workspace`]. Neither works alone.
    pub gate_check_target: String,
    /// T-420/T-422 — the schema step's own dir, content-stamped. See [`schema::gate_schema`].
    pub gate_schema_target: String,
    /// `GATE_LOCK` / `GATE_LOCK_POLL` / `GATE_LOCK_MAX`.
    pub gate_lock: PathBuf,
    pub gate_lock_poll: u64,
    pub gate_lock_max: u64,
    /// `VERIFY_DEBT_NAG` — nag at 8, which is one wave's width.
    pub verify_debt_nag: i64,
    /// The host bridge, detected once (`HOST_BRIDGE` in the bash).
    pub host: host::Host,
    /// `.ai/tickets/registry.json`, parsed once with `is_shipped`'s exact failure semantics.
    pub registry_view: ledger::Registry,
}

impl Ctx {
    /// Resolve everything the bash resolved at load time, and `cd` to the repo root.
    ///
    /// THE `$0` ASSERT, CARRIED OVER. The bash refuses when it cannot locate the repo root,
    /// because `$0` IS THE SHELL when the script is sourced or piped:
    ///
    /// > MEASURED 2026-07-26: `bash -c 'source .../wave.sh status'` from a scratch directory
    /// > printed `open: 0 / 0 tickets` and `ALL WAVES COMPLETE` about a directory that is not the
    /// > repo, because `$0` was `bash`, `dirname` was `.`, and ROOT became `cwd/../..`.
    ///
    /// A compiled binary has no `$BASH_SOURCE`, so the piped/sourced hazard is not reachable in
    /// the same way — but the FAILURE it produced is: a tool describing a directory that is not
    /// the repo. [`crate::root::find_repo_root`] walks up for `.ai/tickets/registry.json` and
    /// errors rather than guessing, which is the same refusal with a different oracle.
    pub fn enter() -> Result<Ctx> {
        let root = crate::root::find_repo_root()?;
        std::env::set_current_dir(&root)?;

        // T-912.2: the compiled lock IS the plan. The old TSV path and its env override died
        // with the TSVs; the generation floor died with them — landed generations live in the
        // lock's wave 0, so waves 1+ are open work only.
        let plan = crate::wave_lock::LOCK_REL.to_string();

        // `git rev-parse --path-format=absolute --git-common-dir`, falling back to `$ROOT/.git`
        // exactly as the bash `|| echo "$ROOT/.git"` did.
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
        // The bash `export`ed it, and every hostrun forwards it explicitly because
        // distrobox-host-exec does NOT forward the environment (measured; see host.rs).
        unsafe { std::env::set_var("CARGO_TARGET_DIR", &cargo_target_dir) };

        // T-300. Resolved AFTER the export above so [`resolve_run_target_dir`] reads the value
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
            registry: ".ai/tickets/registry.json".into(),
            worktrees: ".ai/artifacts/worktrees".into(),
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
                    .join("target/.tbd-gate.lock")
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

/// `git … 2>/dev/null` capturing trimmed stdout, `None` on any failure.
///
/// One helper rather than a `Command` at every site: the bash reached for git roughly ninety
/// times and swallowed stderr at nearly all of them, and the places where it did NOT swallow are
/// the interesting ones ([`ledger::git_porcelain_paths`], T-401).
pub fn git_stdout(args: &[&str]) -> Option<String> {
    let out = std::process::Command::new("git").args(args).output().ok()?;
    if !out.status.success() {
        return None;
    }
    Some(
        String::from_utf8_lossy(&out.stdout)
            .trim_end_matches('\n')
            .to_string(),
    )
}

/// As [`git_stdout`], but keeps the output even when git exited non-zero — the `|| true` shape.
pub fn git_stdout_lossy(args: &[&str]) -> String {
    match std::process::Command::new("git").args(args).output() {
        Ok(out) => String::from_utf8_lossy(&out.stdout)
            .trim_end_matches('\n')
            .to_string(),
        Err(_) => String::new(),
    }
}

/// `git rev-parse --short <rev>`, or the input unchanged when git cannot resolve it.
///
/// The bash interpolated `$(git rev-parse --short "$x")` straight into messages, so a failure
/// rendered as an EMPTY string mid-sentence. Preserved: an unresolvable rev yields `""` here too,
/// because several refusal messages are asserted byte-for-byte by the diff harness.
pub fn short(rev: &str) -> String {
    git_stdout(&["rev-parse", "--short", rev]).unwrap_or_default()
}

/// `git log -1 --format=%s <rev>`, empty when git cannot resolve it (the `2>/dev/null` shape).
pub fn subject(rev: &str) -> String {
    git_stdout(&["log", "-1", "--format=%s", rev]).unwrap_or_default()
}

// ── T-300: THE RUN TARGET, AND THE PROVENANCE CARGO DOES NOT RECORD ──────────────────────────
//
// Correction 1 stays: one shared `CARGO_TARGET_DIR` for check, test and clippy, because a
// per-worktree target is ~44 GB. What it does NOT survive is a lane that LAUNCHES a binary.
//
// MEASURED 2026-09-06, T-300, two checkouts of one package into one target dir:
//
//     A. slice worktree builds first     Compiling tbdprobe v0.1.0 (…/slice-worktree)
//     B. main checkout builds second     Finished `dev` profile … in 0.00s     <- no Compiling
//     C. $CARGO_TARGET_DIR/debug/<bin>   UNMERGED-SLICE-CODE built_from=…/slice-worktree
//     D. `cargo run` from main           UNMERGED-SLICE-CODE built_from=…/slice-worktree
//     E. main's own source says          MERGED-MAIN-CODE
//
// Cargo's `-C metadata` hash omits the manifest path, so both checkouts write the SAME
// `deps/<name>-<hash>` and the same uplifted `<target>/<profile>/<bin>`; freshness is mtime-keyed,
// so main's build is satisfied by the worktree's artifact and never recompiles. That is the wave-1
// T-192 incident (`make api` on :8080 served unmerged slice code) with its mechanism written down
// — the signature defect: success over an input never examined. The cure is two-sided, because
// either half alone fails open: a SEPARATE target, so worktree check/test/clippy traffic into the
// shared cache can never satisfy a run lane's fingerprint (ONE extra directory, not one per
// worktree); and a STAMP naming the sha and the checkout, because a directory is trustworthy only
// if something recorded who wrote it — `cargo clean -p website-api` was the wave-1 fix and nobody
// could have known to run it. [`cmd_run`] is the lane; [`crate::platform_preflight`] is the
// enforcement.

/// The one extra target dir, under the shared cache. Named for its owner: the MAIN checkout.
pub const RUN_TARGET_SUBDIR: &str = "run-main";

/// The provenance file written beside a run binary. Contents are exactly `<sha> <path>`.
pub const RUN_STAMP_FILE: &str = "tbd-built-from";

/// `$K` from the environment, empty treated as unset — `make`'s `?=` semantics, the same rule
/// [`Ctx::enter`] applies to `CARGO_TARGET_DIR`.
fn env_nonempty(k: &str) -> Option<String> {
    std::env::var(k).ok().filter(|s| !s.is_empty())
}

/// `<cargo_target_dir>/run-main` — the whole formula, in one expression, so the driver and
/// `platform preflight` cannot answer this question differently.
///
/// A subdirectory of the shared cache, not a sibling: `platform wave reclaim` already spares the
/// shared dir wholesale, so a run target outside it is a 40 GB tree no sweep knows about. It is
/// also what makes the disk claim checkable — one `run-main`, whatever the worktree count.
pub fn run_target_dir_for(cargo_target_dir: &str) -> String {
    Path::new(cargo_target_dir)
        .join(RUN_TARGET_SUBDIR)
        .display()
        .to_string()
}

/// The run target for a caller with no [`Ctx`] — `platform preflight`, which must not build one
/// (`Ctx::enter` chdirs and prints the host banner).
///
/// Policy, in precedence order, and identical to the one [`Ctx::enter`] applies:
/// `$TBD_RUN_TARGET_DIR` (the driver exports it, so a child inherits the parent's answer), then
/// `$CARGO_TARGET_DIR/run-main`, then `<main checkout>/target/run-main`.
///
/// THE ASYMMETRY, AND WHY IT IS RIGHT. [`disown_ambient_target_dir`] strips `CARGO_TARGET_DIR`
/// before [`Ctx::enter`], so the DRIVER's run target is `<main checkout>/target/run-main` whatever
/// the shell exported; `platform preflight` does not disown, so standalone it answers with the
/// operator's cache — right, because its job is to describe the environment the operator's next
/// `cargo run` will use. Each caller gets the dir its own lanes write, from one formula.
pub fn resolve_run_target_dir(main_root: &Path) -> String {
    if let Some(v) = env_nonempty("TBD_RUN_TARGET_DIR") {
        return v;
    }
    let base = env_nonempty("CARGO_TARGET_DIR")
        .unwrap_or_else(|| main_root.join("target").display().to_string());
    run_target_dir_for(&base)
}

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

/// `<bin_dir>/tbd-built-from` — the stamp sits beside the binaries it describes, so deleting the
/// profile directory deletes the claim with it and cannot leave a stale one behind.
pub fn run_stamp_path(bin_dir: &Path) -> PathBuf {
    bin_dir.join(RUN_STAMP_FILE)
}

/// Write the stamp, creating the directory if cargo has not yet.
pub fn write_run_stamp(bin_dir: &Path, stamp: &RunStamp) -> std::io::Result<()> {
    std::fs::create_dir_all(bin_dir)?;
    std::fs::write(run_stamp_path(bin_dir), stamp.render())
}

/// Read the stamp. `None` covers BOTH "absent" and "unreadable as a stamp" — see
/// [`RunStamp::parse`]; the caller must treat that as unknown provenance, never as agreement.
pub fn read_run_stamp(bin_dir: &Path) -> Option<RunStamp> {
    RunStamp::parse(&std::fs::read_to_string(run_stamp_path(bin_dir)).ok()?)
}

/// `cargo xtask platform wave run <cargo args…> [-- <run args…>]` — the run lane.
///
/// Build, stamp, launch — all into [`Ctx::run_target_dir`], the stamp between the two so it
/// describes a build that succeeded and exists before the server it labels does. Argv left of
/// `--` goes to BOTH cargo verbs (`-p`, `--bin`, `--release` are accepted by each); right of it
/// is the program's and reaches only `run`. That is `ci_editor_api`'s own build-then-run shape,
/// which is the lane this exists to make safe.
///
/// NOT through [`host::Host::hostrun_argv`], and not for style: `hostrun` wraps every command in
/// `timeout $GATE_TIMEOUT` — a run lane's whole point is a process that outlives the gate's
/// 1200 s — and pipes both streams, which would hide a server's log until it exits. Cargo is
/// spawned directly with inherited stdio, as [`crate::mk_build`] does.
pub fn cmd_run(ctx: &Ctx, args: &[String]) -> u8 {
    if let Some(lines) = run_lane_refusal(&ctx.root, &ctx.main_root, &ctx.run_target_dir, args) {
        for l in &lines {
            werr!("{l}");
        }
        return 1;
    }
    // The two-glibc guard, reused rather than re-derived: a container-built run-main read back by
    // host cargo is `GLIBC_2.xx not found`, which reads as a broken checkout (T-853).
    if let Err(msg) = crate::mk_target_dir::abi_guard(Path::new(&ctx.run_target_dir)) {
        werr!("run: {msg}");
        return 1;
    }

    let (build_args, run_args) = split_run_args(args);
    let profile = if build_args.iter().any(|a| a == "--release") {
        "release"
    } else {
        "debug"
    };

    wprintln!("run: CARGO_TARGET_DIR={}", ctx.run_target_dir);
    let rc = spawn_cargo(ctx, "build", &build_args, &[]);
    if rc != 0 {
        werr!("run: build failed (rc {rc}) — nothing stamped, nothing launched.");
        return rc;
    }

    let bin_dir = Path::new(&ctx.run_target_dir).join(profile);
    let stamp = RunStamp {
        sha: git_stdout(&["rev-parse", "HEAD"]).unwrap_or_default(),
        checkout: ctx.root.display().to_string(),
    };
    if stamp.sha.len() != 40 {
        // Refuse rather than write a stamp preflight will reject anyway: an unresolvable HEAD
        // means we cannot say what this binary is, and launching an unlabelled binary is the
        // state this whole ticket exists to end.
        werr!("run: REFUSING — `git rev-parse HEAD` did not resolve; cannot stamp the build.");
        return 1;
    }
    if let Err(e) = write_run_stamp(&bin_dir, &stamp) {
        werr!(
            "run: REFUSING — could not write {}: {e}",
            run_stamp_path(&bin_dir).display()
        );
        return 1;
    }
    wprintln!(
        "run: {} {} {}",
        RUN_STAMP_FILE,
        short(&stamp.sha),
        stamp.checkout
    );

    spawn_cargo(ctx, "run", &build_args, &run_args)
}

/// The three reasons a run lane must not proceed, rendered as the stderr it prints, or `None`.
///
/// A free function over paths rather than a method on [`Ctx`]: building a `Ctx` chdirs and
/// detects the host bridge, so the refusals could only be tested by the thing they prevent.
/// Returning the LINES keeps the message under test — a refusal naming the wrong path is a
/// refusal nobody acts on.
fn run_lane_refusal(
    root: &Path,
    main_root: &Path,
    run_target_dir: &str,
    args: &[String],
) -> Option<Vec<String>> {
    // R1 — no argv builds the whole workspace and stamps it as if a run binary existed.
    if args.is_empty() {
        return Some(vec![
            "run: REFUSING — no cargo arguments.".into(),
            "     usage: cargo xtask platform wave run -p website-api --bin api".into(),
            "            cargo xtask platform wave run -p tbd-tools --bin world -- reclassify"
                .into(),
        ]);
    }
    // R2 — a caller naming its own target dir has opted out and would still get a stamp.
    if let Some(bad) = args
        .iter()
        .find(|a| a.starts_with("--target-dir") || a.starts_with("CARGO_TARGET_DIR="))
    {
        return Some(vec![
            format!("run: REFUSING — `{bad}` overrides the run target this lane exists to pin."),
            "     Drop it, or use `cargo xtask platform wave test --slice <T-xxx>` for a".into(),
            "     private per-slice dir (T-742).".into(),
        ]);
    }
    // R3 — THE MAIN GOAL, at the only place it can be enforced. `run-main` is MAIN's; a
    // worktree writing it is the wave-1 incident again, one layer down. A slice that needs a
    // live server takes a private dir instead: disk, but it cannot poison anybody.
    if root != main_root {
        return Some(vec![
            "run: REFUSING — this is a worktree, and the run target belongs to the main checkout."
                .into(),
            format!("     worktree      = {}", root.display()),
            format!("     main checkout = {}", main_root.display()),
            format!("     run target    = {run_target_dir}"),
            "     A worktree build here would be executed by main's next run lane (T-300).".into(),
            "     Use CARGO_TARGET_DIR=<main_root>/target-<slice> for a private server dir.".into(),
        ]);
    }
    None
}

/// Split a run-lane argv at the first bare `--`: left goes to both cargo verbs, right only to
/// `run`. A missing `--` means everything is a cargo argument.
fn split_run_args(args: &[String]) -> (Vec<String>, Vec<String>) {
    match args.iter().position(|a| a == "--") {
        Some(i) => (args[..i].to_vec(), args[i + 1..].to_vec()),
        None => (args.to_vec(), Vec::new()),
    }
}

/// `cargo <verb> <cargo_args…> [-- <run_args…>]` in the run target, stdio inherited.
fn spawn_cargo(ctx: &Ctx, verb: &str, cargo_args: &[String], run_args: &[String]) -> u8 {
    let mut cmd = std::process::Command::new("cargo");
    cmd.arg(verb)
        .args(cargo_args)
        .current_dir(&ctx.root)
        .env("CARGO_TARGET_DIR", &ctx.run_target_dir);
    if !run_args.is_empty() {
        cmd.arg("--").args(run_args);
    }
    // Flush before the child inherits stdout — the STREAM ORDERING contract in the header.
    flush();
    match cmd.status() {
        // A signalled child has NO exit code; `128+n` is bash's fiction and [`host::capture`]
        // reproduces it deliberately, so this does too. The OOM killer is a routine visitor here.
        Ok(s) => match s.code() {
            Some(code) => (code & 0xff) as u8,
            None => {
                let sig = std::os::unix::process::ExitStatusExt::signal(&s).unwrap_or(0);
                werr!("run: cargo {verb} was killed by signal {sig} — not a build failure.");
                128u8.saturating_add(sig as u8)
            }
        },
        // 127 is "not installed", not "it ran and failed" (`NotRun::ToolAbsent`).
        Err(e) => {
            werr!("run: failed to spawn cargo {verb}: {e}");
            127
        }
    }
}

/// Entry for `cargo xtask platform wave [args…]`.
///
/// The bash tail was a `case "${1:-status}"`, so no argument means `status`. Exit codes are the
/// command's own; the unknown arm prints the header and exits 1.
/// Strip an inherited `CARGO_TARGET_DIR` before anything spawns.
///
/// ── WHY THIS IS AT THE ENTRY POINT AND NOT AT EACH CALL SITE ─────────────────────────────────
///
/// This driver CHOOSES target dirs; it does not take one. It has three, each for a measured
/// reason: `GATE_CHECK_TARGET` so the gate's artifacts are written by the gate alone (that is what
/// makes one fingerprint invalidation hold for every step under it), a per-slice private dir for
/// ad-hoc `cargo test` (T-742, so a slice cannot read a sibling's binary), and the shared warm
/// cache derived from `git rev-parse --git-common-dir` so every linked worktree points at the
/// PRIMARY repo's `target/` instead of cold-building a 609-crate workspace eight times.
///
/// An ambient `CARGO_TARGET_DIR` can only fight all three. MEASURED 2026-08-12, and this is what
/// motivated the guard: the driver was invoked with `CARGO_TARGET_DIR=<repo>/target-container`
/// exported. Steps that cross the bridge run cargo ON THE HOST, so host cargo (glibc 2.43) wrote
/// host binaries into the container's target dir, and the next in-container `cargo run` died with
/// `GLIBC_2.39 not found` — a link error that reads exactly like a broken checkout and is not one.
/// That is the same two-glibc trap `scripts/lib/hostrun.sh` was written for, arriving through an
/// environment variable instead of a compiler.
///
/// Removing it is right rather than merely convenient: there is no value a caller could supply
/// that this driver should honour. Announced, never silent — a tool that quietly edits its own
/// environment is the thing that makes the next failure unexplainable.
fn disown_ambient_target_dir() {
    if let Ok(v) = std::env::var("CARGO_TARGET_DIR") {
        if !v.is_empty() {
            eprintln!("wave: ignoring inherited CARGO_TARGET_DIR={v}");
            eprintln!(
                "      This driver picks its own (gate-check, per-slice private, shared warm cache)."
            );
            eprintln!(
                "      An inherited one crosses the container/host bridge and poisons the cache it names."
            );
            // SAFETY: single-threaded entry, before any child is spawned or thread started.
            unsafe { std::env::remove_var("CARGO_TARGET_DIR") };
        }
    }
}

pub fn run(args: &[String]) -> Result<u8> {
    disown_ambient_target_dir();
    // Internal probe used by the base arm: print the derived wave base and nothing else.
    // Must NOT go through Ctx::enter — Host::detect prints a five-line HOST-shell banner on
    // stderr, and the bash side of that arm is the extracted `prev_wave_close` functions, which
    // never print it. Measured T-902: 11/11 "mismatches" were that banner sitting between the
    // SHA and the disavowal skip; the SHAs themselves were identical.
    if args.first().map(String::as_str) == Some("diff")
        && args.get(1).map(String::as_str) == Some("base-probe")
    {
        return Ok(match base::prev_wave_close() {
            Some(s) => {
                crate::wprintln!("{s}");
                0
            }
            None => 1,
        });
    }
    let ctx = Ctx::enter()?;
    let cmd = args.first().map(String::as_str).unwrap_or("status");
    let rest: Vec<String> = args.iter().skip(1).cloned().collect();

    let rc = match cmd {
        "status" => status::cmd_status(&ctx),
        "prep" => status::cmd_prep(&ctx),
        "test" => test_cmd::cmd_test(&ctx, &rest),
        // T-300. Sibling of `test`, and for the same reason one layer over: `test` keeps a slice's
        // cargo test off the shared cache, `run` keeps a launched binary off it.
        "run" => cmd_run(&ctx, &rest),
        "gate" => match rest.first().map(String::as_str) {
            Some("--slice") => {
                gate::gate_slice(&ctx, rest.get(1).map(String::as_str).unwrap_or(""))
            }
            // `advance` writes the shared persist DB, so it takes the same lock the wave gate
            // holds when it calls this. GATE_LOCK_HELD is deliberately not settable from the
            // environment (it is reset at load), so there is no way to skip this by exporting a
            // variable — here, a `GateLock` has no public constructor at all (T-406).
            Some("--migrate-persist") => {
                let mode = rest.get(1).map(String::as_str).unwrap_or("audit");
                let mut state = lock::GateState::new();
                if mode == "advance" {
                    match state.take(&ctx, "migrate-persist advance") {
                        0 => {}
                        n => return Ok(n),
                    }
                }
                migrate::gate_db_migrate_persist(&ctx, &state, mode)
            }
            other => gate::cmd_gate(&ctx, other.unwrap_or("")),
        },
        "wave" => {
            if rest.first().map(String::as_str) == Some("--close") {
                // T-923: everything after `--close` belongs to the close ceremony
                // (`--summary <text>`, `--dry-run`) and is allowlist-parsed there.
                land::cmd_wave_close(&ctx, &rest[1..])
            } else {
                status::cmd_wave(&ctx)
            }
        }
        "verified" => land::cmd_verified(&ctx, rest.first().map(String::as_str).unwrap_or("")),
        "reclaim" => reclaim::cmd_reclaim(&ctx, &rest),
        "land" => land::cmd_land(&ctx, &rest),
        "revert" => land::cmd_revert(&ctx, rest.first().map(String::as_str).unwrap_or("")),
        "push" => push::cmd_push(&ctx),
        // T-853 addition, not in the bash: the verdict-diff harness. See [`diff`].
        "diff" => diff::cmd_diff(&ctx, &rest),
        _ => {
            println!("{UNKNOWN_HELP}");
            1
        }
    };
    flush();
    Ok(rc)
}
#[cfg(test)]
mod run_target_tests {
    use super::*;

    const SHA: &str = "4b2cca4a5880ee8a0e8fbcbbedd476534db5b0ac";

    fn v(s: &[&str]) -> Vec<String> {
        s.iter().map(|x| (*x).to_string()).collect()
    }

    /// THE FORMULA, and the disk claim with it. `run-main` must be a CHILD of the shared cache
    /// and never the cache itself — collapse the two and every worktree's check/test/clippy
    /// traffic is back in the run lane's fingerprint set with every other test still green. One
    /// answer for every checkout is what makes it one extra target, not one per worktree.
    #[test]
    fn run_target_is_one_child_of_the_shared_cache_and_never_the_cache_itself() {
        let shared = "/home/Samuel/.cache/tbd-target";
        let run = run_target_dir_for(shared);
        assert_eq!(run, format!("{shared}/{RUN_TARGET_SUBDIR}"));
        assert_eq!(run, "/home/Samuel/.cache/tbd-target/run-main");
        assert_ne!(
            run, shared,
            "the run target collapsed onto the shared cache"
        );
        assert!(Path::new(&run).starts_with(shared));
        assert_eq!(
            run,
            run_target_dir_for(shared),
            "not one answer per checkout"
        );
    }
    /// Fail-closed. Every one of these is a shape a "tolerant" parser accepts, and each would
    /// let preflight report agreement about a binary it cannot actually identify.
    #[test]
    fn stamp_parse_refuses_what_it_cannot_read() {
        for bad in [
            "",
            "\n",
            SHA,
            "/run/media/system/Disk_2 \n",
            "4b2cca4a5 /run/media/system/Disk_2",
            "zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz /run/media",
            "4b2cca4a5880ee8a0e8fbcbbedd476534db5b0ac ",
        ] {
            assert_eq!(RunStamp::parse(bad), None, "accepted {bad:?}");
        }
    }

    #[test]
    fn split_run_args_puts_program_arguments_only_on_run() {
        assert_eq!(
            split_run_args(&v(&[
                "-p",
                "tbd-tools",
                "--bin",
                "world",
                "--",
                "reclassify"
            ])),
            (
                v(&["-p", "tbd-tools", "--bin", "world"]),
                v(&["reclassify"])
            )
        );
        assert_eq!(
            split_run_args(&v(&["-p", "website-api", "--bin", "api"])),
            (v(&["-p", "website-api", "--bin", "api"]), v(&[]))
        );
    }

    /// THE MAIN GOAL, as a unit: a worktree may not build into the run target at all, the main
    /// checkout may, and the refusal names both trees so the operator knows which one to look at.
    #[test]
    fn run_lane_refuses_a_worktree_and_names_both_checkouts() {
        let main = Path::new("/repo");
        let wt = Path::new("/repo/.ai/artifacts/worktrees/T-300");
        let args = v(&["-p", "website-api"]);
        let text = run_lane_refusal(wt, main, "/cache/run-main", &args)
            .expect("must refuse")
            .join("\n");
        assert!(text.contains("REFUSING"), "{text}");
        assert!(
            text.contains("/repo/.ai/artifacts/worktrees/T-300"),
            "{text}"
        );
        assert!(text.contains("main checkout = /repo"), "{text}");
        assert!(text.contains("/cache/run-main"), "{text}");
        assert_eq!(run_lane_refusal(main, main, "/cache/run-main", &args), None);
    }

    #[test]
    fn run_lane_refuses_an_empty_argv_and_a_caller_supplied_target_dir() {
        let main = Path::new("/repo");
        assert!(run_lane_refusal(main, main, "/cache/run-main", &[]).is_some());
        for bad in [
            "--target-dir",
            "--target-dir=/tmp/x",
            "CARGO_TARGET_DIR=/tmp/x",
        ] {
            let args = v(&["-p", "website-api", bad]);
            let lines = run_lane_refusal(main, main, "/cache/run-main", &args)
                .unwrap_or_else(|| panic!("accepted {bad}"));
            assert!(lines.join("\n").contains(bad));
        }
    }
}

/// T-923 test support — PROCESS-GLOBAL cwd serialisation for tests that must chdir.
///
/// The close-ceremony tests run the marker authority ([`base::wave_close_number`],
/// [`base::wave_close_is_newest_wave`]) against fabricated scratch repos, and those functions
/// are cwd-bound by design (the driver chdirs once at [`Ctx::enter`]). `cargo test` is
/// multi-threaded and the cwd is process state, so every test that moves it must hold ONE lock —
/// otherwise a concurrent `find_repo_root()` (the scratch repos carry `.ai/tickets/ROOT`, which
/// is exactly what that function looks for) resolves inside somebody's scratch tree and a test
/// fails an assertion about a repo it was never meant to read.
#[cfg(test)]
pub(crate) mod testcwd {
    use std::path::{Path, PathBuf};
    use std::sync::{Mutex, MutexGuard};

    static CWD_LOCK: Mutex<()> = Mutex::new(());

    /// Run `f` with the cwd pinned — the READER half of this module's contract.
    ///
    /// T-946. The module doc above already states the rule ("every test that moves it must hold
    /// ONE lock"), but only the movers held it: a fixture helper resolving `find_repo_root()`
    /// from the cwd took no lock at all, so it could still land inside a scratch tree mid-chdir.
    /// Measured 2026-09-05 in the wave 248 gate, `test xtask+tbd-tools`:
    /// `world_parity_world_column_clears_its_floor_when_the_dem_is_present` failed NotFound on a
    /// committed fixture, 3/4 runs in the gate's cold target dir and never in isolation.
    ///
    /// Resolve UNDER the lock and keep the absolute path: the answer stays valid after the lock
    /// is released, so readers hold it for a path walk and nothing longer. Callers must not
    /// already hold a [`CwdGuard`] — the mutex is not reentrant.
    pub(crate) fn resolve_under_lock<T>(f: impl FnOnce() -> T) -> T {
        // Poisoning ignored for the guard's reason: a panicked sibling restored its cwd on drop.
        let _g = CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        f()
    }

    /// Holds the lock and the previous cwd; restores the cwd on drop (lock released after).
    pub(crate) struct CwdGuard {
        prev: PathBuf,
        _g: MutexGuard<'static, ()>,
    }

    impl CwdGuard {
        /// Take the lock, THEN resolve the target via `f`, then chdir. Resolution happens under
        /// the lock on purpose: `find_repo_root()` reads the cwd, so resolving before locking
        /// would race with whichever test currently owns it. `None` from `f` releases everything
        /// and returns `None` — the caller's skip path.
        pub(crate) fn enter_resolved(f: impl FnOnce() -> Option<PathBuf>) -> Option<CwdGuard> {
            // Poisoning is ignored on purpose: a panicked sibling must not cascade into every
            // later cwd test — the guard's Drop restored its cwd even on that panic.
            let g = CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
            let dir = f()?;
            let prev = std::env::current_dir().expect("cwd");
            std::env::set_current_dir(&dir).expect("chdir");
            Some(CwdGuard { prev, _g: g })
        }

        pub(crate) fn enter(dir: &Path) -> CwdGuard {
            Self::enter_resolved(|| Some(dir.to_path_buf())).expect("enter with Some never skips")
        }
    }

    impl Drop for CwdGuard {
        fn drop(&mut self) {
            let _ = std::env::set_current_dir(&self.prev);
        }
    }
}
