//! Running programs without losing the reason they stopped: the command builder and its results.
//!
//! **Role:** what a run is ([`Run`]) and what it produced ([`Output`], [`Merged`]); the spawning,
//! reaping and draining live in the sibling modules `runner`, `stream` and `lookup`.
//! **Position:** the crate's vocabulary, re-exported at the crate root; every xtask command that
//! runs an external program builds a [`Run`].
//! **Signals & state:** none; a [`Run`] is a plain value consumed by the call that spawns it.
//! **Invariants:** the three corrections below hold for every child; a status is passed through
//! raw.
//!
//! Gates, deployments and the wave gate steps spawn `cargo`, `git`, `ssh`, `rsync`, `podman`, `npm`
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
//! [`NotRun::Signalled`](verification_core::NotRun::Signalled), never a `Failed`.
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
//! This file holds the vocabulary. `runner.rs` spawns, isolates and reaps; `stream.rs` drains the
//! pipes; `lookup.rs` resolves programs on `PATH` and waits on conditions; `run_modes/` holds the
//! runs that do not capture text: the inherited terminal, byte pipes, output files, a detached
//! child, a line stream and the process replacement.

use std::ffi::OsStr;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// What a finished process produced.
#[derive(Debug)]
pub struct Output {
    /// The raw exit code.
    pub code: i32,
    /// Everything the child wrote to stdout, decoded lossily.
    pub stdout: String,
    /// Everything the child wrote to stderr, decoded lossily.
    pub stderr: String,
    /// The wall time from spawn to exit.
    pub duration: Duration,
}

/// What a finished process produced on ONE shared pipe — see [`Run::merged_output`].
///
/// Deliberately a separate type rather than an [`Output`] with an always-empty `stderr`: the whole
/// point is that the two streams are no longer separable, and a field that is always empty is an
/// invitation to read it and conclude the child wrote nothing to stderr.
#[derive(Debug)]
pub struct Merged {
    /// The raw exit code.
    pub code: i32,
    /// stdout and stderr interleaved exactly as the child emitted them.
    pub text: String,
    /// The wall time from spawn to exit.
    pub duration: Duration,
}

/// A command to run.
pub struct Run {
    pub(crate) program: String,
    pub(crate) args: Vec<String>,
    pub(crate) cwd: Option<PathBuf>,
    pub(crate) envs: Vec<(String, String)>,
    pub(crate) env_removes: Vec<String>,
    pub(crate) timeout: Option<Duration>,
    pub(crate) stdin: StdinSource,
}

/// What a child reads on stdin.
pub(crate) enum StdinSource {
    /// Nothing chosen: `/dev/null` for every run except [`Run::terminal`] and
    /// [`Run::replace_process`], which inherit this process's stdin.
    Unset,
    /// `/dev/null`, whatever the run.
    Null,
    /// Bytes written to a pipe by a writer thread, which then closes it.
    Bytes(Vec<u8>),
    /// An open file the child reads directly, as a shell's `< file`.
    File(File),
}

impl Run {
    /// A run of `program`, found on `PATH` when it names no folder, with no argument.
    pub fn new(program: impl AsRef<OsStr>) -> Run {
        Run {
            program: program.as_ref().to_string_lossy().into_owned(),
            args: Vec::new(),
            cwd: None,
            envs: Vec::new(),
            env_removes: Vec::new(),
            timeout: None,
            stdin: StdinSource::Unset,
        }
    }

    /// Appends one argument.
    pub fn arg(mut self, a: impl AsRef<OsStr>) -> Run {
        self.args.push(a.as_ref().to_string_lossy().into_owned());
        self
    }

    /// Appends every argument of `args`, in order.
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

    /// Runs the child in the folder `dir`.
    pub fn cwd(mut self, dir: impl AsRef<Path>) -> Run {
        self.cwd = Some(dir.as_ref().to_path_buf());
        self
    }

    /// Sets the variable `k` to `v` in the child's environment.
    pub fn env(mut self, k: impl Into<String>, v: impl Into<String>) -> Run {
        self.envs.push((k.into(), v.into()));
        self
    }

    /// Removes the variable `k` from the child's environment.
    pub fn env_remove(mut self, k: impl Into<String>) -> Run {
        self.env_removes.push(k.into());
        self
    }

    /// Kills the child's whole process group when it runs longer than `d`.
    pub fn timeout(mut self, d: Duration) -> Run {
        self.timeout = Some(d);
        self
    }

    /// Writes `body` to the child's stdin and closes it; without a body stdin is `/dev/null`.
    pub fn stdin(mut self, body: impl Into<String>) -> Run {
        self.stdin = StdinSource::Bytes(body.into().into_bytes());
        self
    }

    /// Writes the raw bytes `body` to the child's stdin and closes it — a binary archive, say.
    pub fn stdin_bytes(mut self, body: impl Into<Vec<u8>>) -> Run {
        self.stdin = StdinSource::Bytes(body.into());
        self
    }

    /// Hands the child the open `file` as its stdin, as a shell's `< file` does.
    pub fn stdin_file(mut self, file: File) -> Run {
        self.stdin = StdinSource::File(file);
        self
    }

    /// Gives the child `/dev/null` as its stdin even where a run would inherit this process's
    /// stdin ([`Run::terminal`], [`Run::replace_process`]).
    pub fn stdin_null(mut self) -> Run {
        self.stdin = StdinSource::Null;
        self
    }

    /// A human-readable rendering of the command, for diagnostics.
    pub(crate) fn display(&self) -> String {
        if self.args.is_empty() {
            self.program.clone()
        } else {
            format!("{} {}", self.program, self.args.join(" "))
        }
    }
}

#[cfg(test)]
#[path = "tests/run_tests.rs"]
mod tests;
