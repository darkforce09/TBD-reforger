//! Launching the engine, waiting for a verdict, and shutting down.
//!
//! **Role:** launches the dedicated server, waits for its boot verdict, prints the join banner and
//! stops the server on Ctrl-C, SIGTERM or the run deadline; the signal handlers and the tail live
//! in `boot/on_stop_signal.rs`, the `--timeout` deadline in `boot/run_deadline.rs`.
//! **Position:** under [`crate::playtest_server`]; the run order in `usage_fail` calls it with a
//! [`BootCtx`]; log reading lives in [`super::logread`], liveness and kills in
//! [`super::lifecycle`]; this module owns the operations that need a live process.
//! **Signals & state:** `STOP_REQUESTED`, the process-wide flag the SIGINT/SIGTERM handlers set
//! and both wait loops poll in 100 ms slices.
//! **Invariants:** never tail a possibly-hanging stream: the launcher's merged output goes to a
//! FILE (`$RUN_DIR/server.out`) and the wait loop polls that file for three outcomes (the room
//! registration, a config refusal, an engine fatal). Boot time is not a reliable number: on one
//! machine a passing boot registered 13 s after `Starting RPL server` while a failing one never
//! registered across the full 300 s with the world up and the mission in LOBBY, so the 300 s bound
//! is the operator's patience, not an estimate, and the loop prints WHICH PHASE the boot is in
//! ([`boot_phase`]) instead of a countdown. Liveness is checked on the process group, never on the
//! launcher: under the host bridge the local launcher returns almost immediately while the engine
//! is still compiling scripts; the probe runs every 10 s because each one spawns a bridge process.

use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::sleep;
use std::time::Duration;

use super::Opts;
use super::host::Host;
use super::lifecycle::{self, Probe, RunPaths};
use super::logread::{
    assert_local_addon_won, boot_phase, console_log_path, dump_engine_errors, grep_n, has, has_re,
    loaded_addon_line, log, world_is_up,
};

/// Set by the SIGINT/SIGTERM handler; polled by both wait loops.
///
/// A signal handler may only do async-signal-safe work, so it stores this flag and the loops act
/// on it between their steps. Poll slices are 100 ms so Ctrl-C still feels immediate inside the
/// 5 s steady-state sleep.
static STOP_REQUESTED: AtomicBool = AtomicBool::new(false);

/// Everything the boot half needs from the caller.
pub(super) struct BootCtx<'a> {
    pub host: &'a Host,
    pub paths: &'a RunPaths,
    pub opts: &'a Opts,
    pub server_dir: &'a str,
    pub cmd_display: &'a str,
    pub addon_guid: &'a str,
    pub lan_ip: &'a str,
    pub scenario: &'a str,
    /// What the server runs, for the boot banner: `mission <uuid>` or `artifact file <path>`.
    pub running: &'a str,
    /// Runs once the server is up and its banner printed, before the wait for its exit.
    pub after_ready: &'a dyn Fn(),
    /// Runs on Ctrl-C / SIGTERM or an expired `--timeout` after the server was ready, while it is
    /// still up, before the stop.
    pub before_stop: &'a dyn Fn(),
}

/// Which of the four ways the wait loop can end.
enum Verdict {
    Registered,
    /// The engine refused the config or could not start.
    Fatal,
    /// The process group is CONFIRMED gone.
    Died,
    /// 300 s elapsed with none of the above.
    NeverRegistered,
    /// `--timeout` expired before the room registered.
    DeadlineBeforeRegistration,
    /// Ctrl-C / SIGTERM.
    Interrupted,
}

#[cfg(test)]
#[path = "tests/boot/tests.rs"]
mod tests;

#[path = "boot/on_stop_signal.rs"]
mod on_stop_signal;
#[path = "boot/run_deadline.rs"]
mod run_deadline;
pub(super) use on_stop_signal::boot_and_wait;

#[cfg(test)]
use on_stop_signal::launcher_script;
