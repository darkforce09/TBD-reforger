//! Running programs without losing the reason they stopped.
//!
//! Gates, deployments and the wave driver spawn `cargo`, `git`, `ssh`, `rsync`, `podman`, `npm`
//! and the game server binaries. Five things happen to every one of those children — it runs, it
//! reports a status, it writes output, it may take too long, it may need another attempt — and
//! three of them have a failure mode that turns a verification into a false result. Correcting
//! those three is the whole content of this module.
//!
//! ── 1. A SIGNAL IS NOT AN EXIT CODE ──────────────────────────────────────────────────────────
//!
//! A child killed by SIGKILL has no exit code at all; a caller that synthesises `128+n` from the
//! signal number hands the next `match` arm a 137 that reads as an ordinary numeric failure.
//! Under eight parallel worktrees the OOM killer is a routine visitor, so "the kernel shot the
//! gate" would regularly be reported as "the gate found a problem". Here that is
//! [`NotRun::Signalled`](crate::verdict::NotRun::Signalled), never a `Failed`.
//!
//! ── 2. A DEADLINE MUST KILL THE TREE, NOT THE DIRECT CHILD ───────────────────────────────────
//!
//! `ArmaReforgerServer`, `cargo`, `ssh` and `podman` all fork. Killing only the process that was
//! spawned leaves the grandchildren alive, holding the log file and the listening port, and the
//! next run then fails for reasons that have nothing to do with the code under test. Every child
//! spawned here is put in **its own process group** via `setsid`, and a timeout kills the group.
//!
//! ── 3. A FULL PIPE DEADLOCKS A CAPTURED CHILD ────────────────────────────────────────────────
//!
//! Capturing stdout while the child also writes stderr deadlocks once either pipe buffer fills —
//! about 64 KiB, which a world-boot log clears comfortably. Both streams are drained by dedicated
//! threads for the child's whole life, so neither can wedge.
//!
//! Statuses are otherwise passed through **raw**: [`Run::status`] hands back the real code and
//! never collapses it, because `cargo xtask mod compile --selftest` passes only on exactly 1 and
//! `cargo xtask map export-terrain` exits 2 to mean "stage the Workbench export first".
//!
//! ── THE MODULE TREE ──────────────────────────────────────────────────────────────────────────
//!
//! This file holds the vocabulary: what a run is ([`Run`]) and what it produced ([`Output`],
//! [`Merged`]). `runner.rs` spawns, isolates and reaps; `stream.rs` drains the pipes; `lookup.rs`
//! resolves programs on `PATH` and waits on conditions.

mod lookup;
mod runner;
mod stream;

pub use lookup::{retry, wait_for, which};

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// What a finished process produced.
#[derive(Debug)]
pub struct Output {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
    pub duration: Duration,
}

/// What a finished process produced on ONE shared pipe — see [`Run::merged_output`].
///
/// Deliberately a separate type rather than an [`Output`] with an always-empty `stderr`: the whole
/// point is that the two streams are no longer separable, and a field that is always empty is an
/// invitation to read it and conclude the child wrote nothing to stderr.
#[derive(Debug)]
pub struct Merged {
    pub code: i32,
    /// stdout and stderr interleaved exactly as the child emitted them.
    pub text: String,
    pub duration: Duration,
}

/// A command to run.
pub struct Run {
    program: String,
    args: Vec<String>,
    cwd: Option<PathBuf>,
    envs: Vec<(String, String)>,
    env_removes: Vec<String>,
    timeout: Option<Duration>,
    stdin: Option<String>,
}

impl Run {
    pub fn new(program: impl AsRef<OsStr>) -> Run {
        Run {
            program: program.as_ref().to_string_lossy().into_owned(),
            args: Vec::new(),
            cwd: None,
            envs: Vec::new(),
            env_removes: Vec::new(),
            timeout: None,
            stdin: None,
        }
    }

    pub fn arg(mut self, a: impl AsRef<OsStr>) -> Run {
        self.args.push(a.as_ref().to_string_lossy().into_owned());
        self
    }

    pub fn args<I, S>(mut self, args: I) -> Run
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        for a in args {
            self.args.push(a.as_ref().to_string_lossy().into_owned());
        }
        self
    }

    pub fn cwd(mut self, dir: impl AsRef<Path>) -> Run {
        self.cwd = Some(dir.as_ref().to_path_buf());
        self
    }

    pub fn env(mut self, k: impl Into<String>, v: impl Into<String>) -> Run {
        self.envs.push((k.into(), v.into()));
        self
    }

    pub fn env_remove(mut self, k: impl Into<String>) -> Run {
        self.env_removes.push(k.into());
        self
    }

    pub fn timeout(mut self, d: Duration) -> Run {
        self.timeout = Some(d);
        self
    }

    pub fn stdin(mut self, body: impl Into<String>) -> Run {
        self.stdin = Some(body.into());
        self
    }

    /// A human-readable rendering of the command, for diagnostics.
    fn display(&self) -> String {
        if self.args.is_empty() {
            self.program.clone()
        } else {
            format!("{} {}", self.program, self.args.join(" "))
        }
    }
}

#[cfg(test)]
#[path = "../tests/proc_tests.rs"]
mod tests;
