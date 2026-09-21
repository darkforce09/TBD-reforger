//! Subprocess plumbing (T-915.3) — the ONLY way the app runs external commands.
//!
//! Every spawn gets an explicit `current_dir` at the discovered repo root (a
//! GUI-launched process has an arbitrary cwd) and cargo resolves robustly
//! ($CARGO → PATH → `$HOME/.cargo/bin/cargo` — the GUI PATH is bare, without
//! rustup shims). stdout and stderr are merged into ONE line stream over mpsc
//! (per-pipe reader threads, so whole lines never interleave); the exit code is
//! delivered as a terminal event; `kill` works from any thread. The UI retains at
//! most [`LOG_CAP`] lines (`LogRing`) — the stream itself is drained every frame.
//!
//! No egui types here; the resolution order, the ring and the event shapes are
//! pure and unit-tested.

use std::collections::VecDeque;
use std::ffi::{OsStr, OsString};
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// Retained-output bound for the UI ring buffer (the stream is unbounded but
/// drained per frame; only the last ~500 lines are kept for the verbatim pane).
pub const LOG_CAP: usize = 500;

/// Exit-poll interval for the waiter thread. Polling (not `wait()`) is deliberate:
/// killing `cargo run` leaves the grandchild binary holding the pipes open, so an
/// EOF-based wait would hang — `try_wait` sees the death regardless.
const WAIT_POLL: Duration = Duration::from_millis(25);

/// One event from a spawned subprocess, in arrival order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcEvent {
    /// One line of merged stdout+stderr, verbatim (no trailing newline).
    Line(String),
    /// Terminal: the process exited. `code == None` means killed by a signal.
    Exited { code: Option<i32> },
    /// Terminal: the spawn itself failed (binary absent, permission…) — the
    /// verbatim OS error. No `Exited` follows.
    SpawnFailed(String),
}

/// Handle to an in-flight subprocess: drain `rx` non-blocking from the UI;
/// `kill()` from any thread.
pub struct ProcHandle {
    pub rx: Receiver<ProcEvent>,
    shared: Arc<Mutex<Shared>>,
}

struct Shared {
    child: Option<Child>,
    kill_requested: bool,
}

impl ProcHandle {
    /// Kill the subprocess (SIGKILL). Safe before the spawn lands (the worker
    /// honors the request) and after exit (no-op).
    pub fn kill(&self) {
        let mut shared = self.shared.lock().expect("subproc mutex");
        shared.kill_requested = true;
        if let Some(child) = shared.child.as_mut() {
            let _ = child.kill();
        }
    }
}

/// Spawn `program args…` with `current_dir = cwd`, streaming merged
/// stdout+stderr lines plus the exit code over the returned handle. `on_event`
/// fires after every send (the app passes `request_repaint`). All IO — including
/// the spawn itself — happens on worker threads; this returns immediately.
pub fn spawn_streaming(
    program: impl AsRef<OsStr>,
    args: &[&str],
    cwd: &Path,
    on_event: impl Fn() + Send + Sync + 'static,
) -> ProcHandle {
    let (tx, rx) = mpsc::channel();
    let shared = Arc::new(Mutex::new(Shared {
        child: None,
        kill_requested: false,
    }));
    let program: OsString = program.as_ref().to_owned();
    let args: Vec<OsString> = args.iter().map(OsString::from).collect();
    let cwd = cwd.to_path_buf();
    let worker_shared = Arc::clone(&shared);
    let on_event = Arc::new(on_event);
    thread::spawn(move || run_child(&program, &args, &cwd, &worker_shared, &tx, &on_event));
    ProcHandle { rx, shared }
}

fn run_child(
    program: &OsStr,
    args: &[OsString],
    cwd: &Path,
    shared: &Arc<Mutex<Shared>>,
    tx: &Sender<ProcEvent>,
    on_event: &Arc<impl Fn() + Send + Sync + 'static>,
) {
    let spawned = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();
    let mut child = match spawned {
        Ok(child) => child,
        Err(e) => {
            let _ = tx.send(ProcEvent::SpawnFailed(e.to_string()));
            on_event();
            return;
        }
    };
    // Take the pipes BEFORE parking the child in the mutex: the reader threads own
    // them outright, so `try_wait`/`kill` never contend with a blocked read.
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    {
        let mut lock = shared.lock().expect("subproc mutex");
        if lock.kill_requested {
            let _ = child.kill();
        }
        lock.child = Some(child);
    }
    // Two pipe types, one generic reader — spawned separately.
    if let Some(pipe) = stdout {
        let tx = tx.clone();
        let on_event = Arc::clone(on_event);
        thread::spawn(move || stream_lines(pipe, &tx, &on_event));
    }
    if let Some(pipe) = stderr {
        let tx = tx.clone();
        let on_event = Arc::clone(on_event);
        thread::spawn(move || stream_lines(pipe, &tx, &on_event));
    }
    loop {
        let status = {
            let mut lock = shared.lock().expect("subproc mutex");
            match lock.child.as_mut().map(Child::try_wait) {
                Some(Ok(Some(status))) => {
                    lock.child = None;
                    Some(status)
                }
                _ => None,
            }
        };
        if let Some(status) = status {
            // Give the readers a moment to flush buffered tail lines so `Exited`
            // lands after the output it explains (best effort — UI order only).
            thread::sleep(WAIT_POLL);
            let _ = tx.send(ProcEvent::Exited {
                code: status.code(),
            });
            on_event();
            return;
        }
        thread::sleep(WAIT_POLL);
    }
}

/// Read one pipe to EOF, sending whole lines. A failed send means the UI dropped
/// the receiver — stop quietly.
fn stream_lines(pipe: impl Read, tx: &Sender<ProcEvent>, on_event: &Arc<impl Fn()>) {
    let reader = BufReader::new(pipe);
    for line in reader.lines() {
        let Ok(line) = line else { return };
        if tx.send(ProcEvent::Line(line)).is_err() {
            return;
        }
        on_event();
    }
}

// ---- cargo resolution ----

/// Resolve the cargo binary from real process env. Order: `$CARGO` (set by cargo
/// itself when it launched us) → `cargo` on `$PATH` → `$HOME/.cargo/bin/cargo`
/// (the rustup default a bare GUI PATH misses) → literal `cargo`, whose spawn
/// failure surfaces verbatim in the banner.
pub fn resolve_cargo() -> PathBuf {
    resolve_cargo_from(
        std::env::var_os("CARGO").as_deref(),
        std::env::var_os("PATH").as_deref(),
        std::env::var_os("HOME").map(PathBuf::from).as_deref(),
    )
}

/// The pure resolution order — env injected for tests.
pub fn resolve_cargo_from(
    cargo_env: Option<&OsStr>,
    path_env: Option<&OsStr>,
    home: Option<&Path>,
) -> PathBuf {
    if let Some(cargo) = cargo_env
        && !cargo.is_empty()
    {
        let candidate = PathBuf::from(cargo);
        if candidate.is_file() {
            return candidate;
        }
    }
    if let Some(path) = path_env {
        for dir in std::env::split_paths(path) {
            if dir.as_os_str().is_empty() {
                continue;
            }
            let candidate = dir.join("cargo");
            if candidate.is_file() {
                return candidate;
            }
        }
    }
    if let Some(home) = home {
        let candidate = home.join(".cargo").join("bin").join("cargo");
        if candidate.is_file() {
            return candidate;
        }
    }
    PathBuf::from("cargo")
}

// ---- retained output ----

/// Bounded verbatim-output buffer: keeps the LAST `cap` lines and counts what was
/// dropped, so the pane can say "… N earlier lines dropped" instead of lying by
/// omission.
pub struct LogRing {
    lines: VecDeque<String>,
    dropped: usize,
    cap: usize,
}

impl LogRing {
    pub fn new(cap: usize) -> Self {
        Self {
            lines: VecDeque::with_capacity(cap.min(64)),
            dropped: 0,
            cap,
        }
    }

    pub fn push(&mut self, line: String) {
        if self.lines.len() == self.cap {
            self.lines.pop_front();
            self.dropped += 1;
        }
        self.lines.push_back(line);
    }

    pub fn clear(&mut self) {
        self.lines.clear();
        self.dropped = 0;
    }

    pub fn lines(&self) -> impl Iterator<Item = &str> {
        self.lines.iter().map(String::as_str)
    }

    pub fn len(&self) -> usize {
        self.lines.len()
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    pub fn dropped(&self) -> usize {
        self.dropped
    }
}

#[cfg(test)]
#[path = "tests/subproc_tests.rs"]
mod tests;
