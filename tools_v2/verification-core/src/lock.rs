//! The gate lock — `flock(2)`, with "I failed to lock" made unrepresentable.
//!
//! ── THE DEFECT THIS TYPE EXISTS TO PREVENT ───────────────────────────────────────────────────
//!
//! `cargo xtask platform wave` serialises its expensive steps on one lock file so that two
//! worktrees cannot build into the same paths at once. Tracking that in a separate success flag —
//! a variable the acquiring function sets to 1 when it believes it succeeded — is the same shape
//! as a gate that reports OK over an input it never examined: a failed lock (a full disk, say)
//! leaves the flag set and the gate runs unserialised.
//!
//! Here there is no flag. [`GateLock`] has a private field and no public constructor, so **the
//! only way to hold one is to have acquired it**, and a function that needs serialisation takes
//! `&GateLock` as an argument. Forgetting to check is not something the type system will compile.
//!
//! ── ONE PATH, RESOLVED AGAINST THE PRIMARY REPO ──────────────────────────────────────────────
//!
//! `flock(1)` and `flock(2)` are the same primitive, so any process that names the same path
//! contends correctly. [`GATE_LOCK_RELPATH`] is that path, and it is repo-relative because every
//! linked worktree must resolve it to the PRIMARY repo (`git rev-parse --git-common-dir`), or the
//! lock serialises nothing.
//!
//! ── ON EXHAUSTION, REFUSE ────────────────────────────────────────────────────────────────────
//!
//! [`DEFAULT_MAX`] is 3600s, and reaching it is a refusal, never an unserialised run.
//! [`flock_exclusive`] returns [`NotRun::Timeout`] — a `DidNotRun`, not a `Failed`, because a gate
//! that could not serialise did not examine a tree anyone can name.

use std::fs::{File, OpenOptions};
use std::os::fd::AsRawFd;
use std::path::Path;
use std::time::{Duration, Instant};

use crate::verdict::NotRun;

/// Repo-relative path of the shared gate lock. `TBD_GATE_LOCK` overrides it.
///
/// Resolve it against the **primary** repo root (`git rev-parse --git-common-dir`), never against
/// a linked worktree's own root — a per-worktree lock file serialises nothing.
pub const GATE_LOCK_RELPATH: &str = "target/.repository-verification.lock";

/// Heartbeat interval while blocked; `TBD_GATE_LOCK_POLL` overrides it.
pub const DEFAULT_POLL: Duration = Duration::from_secs(30);
/// Refuse after this long; `TBD_GATE_LOCK_MAX` overrides it.
pub const DEFAULT_MAX: Duration = Duration::from_secs(3600);

/// Proof that an exclusive lock is currently held.
///
/// Released when dropped (the kernel releases on last close). Not `Clone`, and constructible only
/// by [`flock_exclusive`].
#[derive(Debug)]
pub struct GateLock {
    // Held for its Drop. The lock lives on the open file description, so closing releases it.
    _file: File,
    path: std::path::PathBuf,
}

impl GateLock {
    /// The lock file actually held, for logging.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// Acquire the exclusive lock at `path`, blocking up to `max` and calling `heartbeat` every
/// `poll` while it waits.
///
/// A gate that blocks silently for minutes is indistinguishable from a hung one, and this program
/// runs unattended — hence the heartbeat.
pub fn flock_exclusive(
    path: &Path,
    poll: Duration,
    max: Duration,
    mut heartbeat: impl FnMut(Duration),
) -> Result<GateLock, NotRun> {
    if let Some(parent) = path.parent() {
        // A missing parent is not fatal on its own — the open below will report the real cause.
        let _ = std::fs::create_dir_all(parent);
    }

    let file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .open(path)
        .map_err(|source| NotRun::Unreadable {
            path: path.to_path_buf(),
            source,
        })?;

    let fd = file.as_raw_fd();
    let started = Instant::now();
    // Poll in short slices so the deadline is punctual while the heartbeat stays at `poll`.
    let slice = Duration::from_millis(50).min(poll);
    let mut next_beat = poll;

    loop {
        // SAFETY: `fd` is a live descriptor owned by `file` for the whole call.
        let rc = unsafe { libc::flock(fd, libc::LOCK_EX | libc::LOCK_NB) };
        if rc == 0 {
            return Ok(GateLock {
                _file: file,
                path: path.to_path_buf(),
            });
        }

        let err = std::io::Error::last_os_error();
        match err.raw_os_error() {
            // Held by someone else — the one case worth waiting on.
            Some(libc::EWOULDBLOCK) => {}
            // Anything else (EIO, or ENOLCK on a full disk) means the lock
            // was NOT taken. Never fall through to "proceed anyway".
            _ => {
                return Err(NotRun::ToolError {
                    tool: format!("flock {}", path.display()),
                    status: err.raw_os_error().unwrap_or(-1),
                    stderr: format!("could not lock: {err}"),
                });
            }
        }

        let waited = started.elapsed();
        if waited >= max {
            return Err(NotRun::Timeout {
                tool: format!("flock {}", path.display()),
                secs: max.as_secs(),
            });
        }
        if waited >= next_beat {
            heartbeat(waited);
            next_beat = waited + poll;
        }
        std::thread::sleep(slice);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_lock(name: &str) -> std::path::PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!(
            "verification-core-lock-{}-{name}",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&p);
        p
    }

    #[test]
    fn acquires_when_free() {
        let p = tmp_lock("free");
        let got = flock_exclusive(&p, DEFAULT_POLL, Duration::from_secs(5), |_| {});
        assert!(got.is_ok());
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn a_second_holder_is_refused_not_granted() {
        // flock() treats separate open file descriptions independently even within one process,
        // so this is a genuine contention test.
        let p = tmp_lock("contend");
        let first = flock_exclusive(&p, DEFAULT_POLL, Duration::from_secs(5), |_| {}).unwrap();

        let second = flock_exclusive(
            &p,
            Duration::from_millis(20),
            Duration::from_millis(150),
            |_| {},
        );
        match second {
            Err(NotRun::Timeout { .. }) => {}
            Ok(_) => panic!("TWO HOLDERS AT ONCE — the lock serialised nothing"),
            Err(other) => panic!("expected Timeout, got {other:?}"),
        }
        drop(first);
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn dropping_releases_for_the_next_holder() {
        let p = tmp_lock("release");
        let first = flock_exclusive(&p, DEFAULT_POLL, Duration::from_secs(5), |_| {}).unwrap();
        drop(first);
        let second = flock_exclusive(
            &p,
            Duration::from_millis(20),
            Duration::from_secs(2),
            |_| {},
        );
        assert!(second.is_ok(), "a released lock must be re-acquirable");
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn exhaustion_is_did_not_run_never_a_pass() {
        // Give up rather than run unserialised.
        let p = tmp_lock("refuse");
        let _held = flock_exclusive(&p, DEFAULT_POLL, Duration::from_secs(5), |_| {}).unwrap();
        let got = flock_exclusive(
            &p,
            Duration::from_millis(10),
            Duration::from_millis(60),
            |_| {},
        );
        let cause = got.expect_err("must not succeed");
        assert!(matches!(cause, NotRun::Timeout { .. }));
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn heartbeat_fires_while_blocked() {
        let p = tmp_lock("beat");
        let _held = flock_exclusive(&p, DEFAULT_POLL, Duration::from_secs(5), |_| {}).unwrap();
        let mut beats = 0;
        let _ = flock_exclusive(
            &p,
            Duration::from_millis(20),
            Duration::from_millis(150),
            |_| {
                beats += 1;
            },
        );
        assert!(
            beats >= 2,
            "a silent block is indistinguishable from a hang; got {beats} beats"
        );
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn interops_with_the_flock_command() {
        // Any process naming this path must contend with this lock. If `flock(1)` and this do not
        // contend, both "serialise" against nothing and the whole lock is decorative.
        let p = tmp_lock("interop");
        if crate::proc::which("flock").is_err() {
            eprintln!("skip: flock(1) not installed");
            return;
        }
        // Hold the lock from a shell process the ordinary way (fd 9 + flock 9).
        let script = format!("exec 9>{}; flock 9; sleep 3", p.display());
        let mut child = std::process::Command::new("sh")
            .arg("-c")
            .arg(&script)
            .spawn()
            .unwrap();
        // Give the child time to actually take it before asserting contention.
        std::thread::sleep(Duration::from_millis(400));

        let got = flock_exclusive(
            &p,
            Duration::from_millis(20),
            Duration::from_millis(200),
            |_| {},
        );
        let refused = matches!(got, Err(NotRun::Timeout { .. }));

        let _ = child.kill();
        let _ = child.wait();
        let _ = std::fs::remove_file(&p);
        assert!(
            refused,
            "Rust did not contend with bash's flock — the shared lock is decorative"
        );
    }
}
