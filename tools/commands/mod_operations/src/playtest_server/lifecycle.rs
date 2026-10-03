//! Kill discipline, the liveness probe and the run lock of the playtest server.
//!
//! **Role:** names the run folder's paths ([`RunPaths`]), the four probe answers ([`Probe`]) and
//! the far-side probe script, and holds the run lock ([`LockGuard`]); `lifecycle/probe_group.rs`
//! probes a process group, kills a run, claims the lock and refuses to start over a live server;
//! `lifecycle/selftest.rs` runs `--selftest` against real process groups.
//! **Position:** under [`crate::playtest_server`]; reached before every other check on purpose:
//! `--selftest` gets here without a mission id, and `assert_no_live_server` runs BEFORE staging
//! rewrites `server.json` underneath a server that is still running; [`super::boot`] polls it.
//! **Signals & state:** the lock folder `$RUN_DIR/.run.lock` for as long as its guard lives, and
//! the pidfile naming the server's process group.
//! **Invariants:** `Unknown` is never `Dead`: only a confirmed dead or zombie group lets a run
//! proceed; the run lock is released by `Drop`, so nothing under the lock calls
//! `process::exit`; two guards answer different questions (another copy of this program on the
//! run folder, and a server left by a previous run, which a killed program leaves without a lock).

use std::path::Path;
use std::thread::sleep;
use std::time::Duration;

use super::Opts;
use super::host::Host;

/// The three run-dir paths the lifecycle owns.
pub(super) struct RunPaths {
    pub run_dir: String,
    pub pidfile: String,
    /// The launcher's merged stdout+stderr. The boot loop polls this FILE rather than tailing a
    /// stream — see [`super::boot`].
    pub srv_out: String,
    pub lockdir: String,
}

impl RunPaths {
    pub(super) fn new(run_dir: &str) -> RunPaths {
        RunPaths {
            run_dir: run_dir.to_string(),
            pidfile: format!("{run_dir}/server.pid"),
            srv_out: format!("{run_dir}/server.out"),
            lockdir: format!("{run_dir}/.run.lock"),
        }
    }
}

/// What the probe found. **Four states, and `Unknown` is NOT `Dead`.**
///
/// A three-valued answer is enough to make the bug unrepresentable, and `Zombie`
/// has to be separate too — see [`probe_group()`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Probe {
    Alive,
    Zombie,
    Dead,
    Unknown,
}

impl Probe {
    /// The four words the bash `printf`ed, used verbatim in the escalation message.
    pub(super) fn word(self) -> &'static str {
        match self {
            Probe::Alive => "alive",
            Probe::Zombie => "zombie",
            Probe::Dead => "dead",
            Probe::Unknown => "unknown",
        }
    }

    /// Is the group CONFIRMED not to be holding sockets? `Unknown` is deliberately not included and
    /// there is no `bool` conversion that could let it slip in.
    fn confirmed_gone(self) -> bool {
        matches!(self, Probe::Dead | Probe::Zombie)
    }
}

/// The far side of the probe, run on the host, verbatim from the bash.
///
/// The sentinel is printed by the FAR SIDE. See [`probe_group()`] for why that is the whole design.
const PROBE_SH: &str = r#"
    p=$1
    if kill -0 -- "-$p" 2>/dev/null; then
      live=0; seen=0
      for q in $(pgrep -g "$p" 2>/dev/null); do
        seen=$((seen + 1))
        st=$(sed -n "s/^State:[[:space:]]*\([A-Z]\).*/\1/p" "/proc/$q/status" 2>/dev/null)
        [ "$st" = "Z" ] || live=$((live + 1))
      done
      if [ "$seen" -gt 0 ] && [ "$live" -eq 0 ]; then echo "TBDPROBE=zombie"; else echo "TBDPROBE=alive"; fi
    else
      echo "TBDPROBE=dead"
    fi
  "#;

/// Whether `kill_run` may print its escalation line.
///
/// bash called it as `kill_run >/dev/null 2>&1` at every one of the FOUR selftest call sites and
/// bare everywhere else. Without this distinction the port's `--selftest` output gains two
/// `TERM did not settle …` lines the baseline does not have — caught by the byte diff, which is
/// exactly what that diff is for.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Volume {
    Loud,
    Quiet,
}

// ── the run lock ─────────────────────────────────────────────────────────────────────────────
// The run folder is fixed, so a second invocation started before the first server stops would
// delete the only handle on the running group (its pidfile), the first instance's `kill_run`
// would read no pidfile and report "server exited" while its engine was still up, and the new
// boot would die on the port. Two guards, because they answer different questions:
//   claim_lock            — is another copy of THIS PROGRAM using this run dir?
//   assert_no_live_server — is a SERVER still running from a previous invocation of it?
// The second one matters on its own: a program that was killed leaves no lock but can very much
// leave a server.

/// Holds the lock dir for as long as it is alive.
///
/// The lock is released by `Drop`, which is why nothing under the lock may call `process::exit` —
/// see the note at the `claim_lock` call site.
pub(super) struct LockGuard {
    dir: String,
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// The verdict of [`check_no_live_server`], so the selftest can read the refusal text without
/// capturing a terminal — and so the `Unknown` arm is reachable in a unit test at all.
pub(super) enum LiveVerdict {
    /// Nothing is running from a previous invocation. The pidfile has been cleared if it named a
    /// confirmed-dead group.
    Clear,
    Refuse {
        code: u8,
        message: Vec<String>,
    },
}

// ── --selftest ───────────────────────────────────────────────────────────────────────────────

/// Spawn a real `setsid` process group on the host and echo its pgid. `code` is shell the group
/// leader runs; that is how the TERM-ignoring case is built.
const SPAWN_SH: &str = r#"
      f=$(mktemp /tmp/tbd-rps-pg.XXXXXX)
      setsid sh -c "echo \$\$ > $f; $1" >/dev/null 2>&1 &
      n=0
      while [ ! -s "$f" ] && [ "$n" -lt 50 ]; do n=$((n+1)); sleep 0.1; done
      cat "$f"; rm -f "$f"
    "#;

/// Tallies the ok/FAIL lines exactly as bash's `st_pass` / `st_fail` did.
struct Tally {
    rc: u8,
}

impl Tally {
    fn pass(&self, msg: &str) {
        println!("  ok    {msg}");
    }
    fn fail(&mut self, msg: &str) {
        println!("  FAIL  {msg}");
        self.rc = 1;
    }
    /// `cond ? pass : fail` — the `[ … ] && st_pass … || st_fail …` idiom, made unable to run both
    /// arms (which the bash form does whenever `st_pass` itself returns non-zero).
    fn check(&mut self, cond: bool, ok: &str, bad: &str) {
        if cond {
            self.pass(ok);
        } else {
            self.fail(bad);
        }
    }
}

#[cfg(test)]
#[path = "tests/lifecycle/tests.rs"]
mod tests;

mod probe_group;
pub(super) use probe_group::assert_no_live_server;
pub(super) use probe_group::check_no_live_server;
pub(super) use probe_group::claim_lock;
pub(super) use probe_group::kill_run;
use probe_group::kill_run_inner;
pub(super) use probe_group::print_stray_warning;
pub(super) use probe_group::probe_group;
pub(super) use probe_group::read_pgid;
use probe_group::st_spawn;

mod selftest;
pub(super) use selftest::selftest;

#[cfg(test)]
use probe_group::{local_pid_is_alive, stray_warning};
