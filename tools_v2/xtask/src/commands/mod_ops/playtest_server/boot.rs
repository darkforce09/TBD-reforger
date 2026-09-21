//! Launching the engine, waiting for a verdict, and shutting down.
//!
//! Log reading lives in [`super::logread`]; this module owns operations that need a live process.
//!
//! ── NEVER TAIL A POSSIBLY-HANGING STREAM ─────────────────────────────────────────────────────
//!
//! The launcher's merged output goes to a FILE (`$RUN_DIR/server.out`) and the wait loop polls that
//! file. Three outcomes are polled for: the room registration, a config refusal, and an engine fatal.
//!
//! HOW LONG THIS TAKES IS NOT A RELIABLE NUMBER, and the bash comment here used to claim one
//! ("~95 s measured, 300 s generous"). Measured on this machine, two boots minutes apart:
//!
//! ```text
//!   passing boot   `Server registered with address:` landed 13 s after `Starting RPL server`
//!   failing boot   never, across the full 300 s — world long up, mission in LOBBY, vehicles
//!                  spawned, and the room simply never registered
//! ```
//!
//! Same binary, same config. So the wait is variable and the 300 s below is a bound on our patience,
//! not an estimate of the engine's. What is worth printing is not a countdown against a fictional
//! average but WHICH PHASE the boot is in, which [`boot_phase`] reads out of the log — "still
//! compiling" and "world up, registration pending" are different problems and the old loop printed
//! the same sentence for both, thirty times.
//!
//! ── LIVENESS IS CHECKED ON THE PROCESS GROUP, NOT ON THE LAUNCHER ────────────────────────────
//!
//! Under the host bridge the local launcher returns almost immediately (the world-boot gate records
//! the same trap), so `kill -0 $LAUNCHER` reports "died" while the engine is still compiling
//! scripts — measured here as a FAILED verdict 9 KB into a boot that was going fine. And it is
//! checked every 10 s rather than every tick because each probe spawns a bridge process.

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
/// bash used `trap '…' INT TERM`, which runs the handler body between commands. A Rust signal handler
/// may only do async-signal-safe work, so it stores a flag and the loops act on it — the same
/// "between commands" granularity, made explicit. Poll slices are 100 ms so Ctrl-C still feels
/// immediate inside the 5 s steady-state sleep.
static STOP_REQUESTED: AtomicBool = AtomicBool::new(false);

/// Everything the boot half needs from the caller.
pub struct BootCtx<'a> {
    pub host: &'a Host,
    pub paths: &'a RunPaths,
    pub opts: &'a Opts,
    pub server_dir: &'a str,
    pub cmd_display: &'a str,
    pub addon_guid: &'a str,
    pub lan_ip: &'a str,
    pub scenario: &'a str,
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
    /// Ctrl-C / SIGTERM.
    Interrupted,
}

#[cfg(test)]
#[path = "tests/boot/tests.rs"]
mod tests;

#[path = "boot/on_stop_signal.rs"]
mod on_stop_signal;
pub use on_stop_signal::boot_and_wait;

#[cfg(test)]
use on_stop_signal::launcher_script;
