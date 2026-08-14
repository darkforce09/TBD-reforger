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
mod tests {
    use super::*;
    use crate::testutil::Scratch;
    use std::fs;

    /// Executable-enough for resolution tests: the resolver checks `is_file`.
    fn touch(path: &Path) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, "#!/bin/sh\n").unwrap();
    }

    #[test]
    fn cargo_env_wins_when_it_exists() {
        let s = Scratch::new("cargo-env");
        let cargo_bin = s.path().join("toolchain/cargo");
        touch(&cargo_bin);
        let path_cargo = s.path().join("onpath/cargo");
        touch(&path_cargo);
        let got = resolve_cargo_from(
            Some(cargo_bin.as_os_str()),
            Some(path_cargo.parent().unwrap().as_os_str()),
            Some(s.path()),
        );
        assert_eq!(got, cargo_bin);
    }

    #[test]
    fn stale_cargo_env_falls_through_to_path() {
        let s = Scratch::new("cargo-stale");
        let path_cargo = s.path().join("onpath/cargo");
        touch(&path_cargo);
        let missing = s.path().join("gone/cargo");
        let got = resolve_cargo_from(
            Some(missing.as_os_str()),
            Some(path_cargo.parent().unwrap().as_os_str()),
            Some(s.path()),
        );
        assert_eq!(got, path_cargo);
    }

    #[test]
    fn empty_cargo_env_falls_through_to_path() {
        let s = Scratch::new("cargo-empty");
        let path_cargo = s.path().join("onpath/cargo");
        touch(&path_cargo);
        let got = resolve_cargo_from(
            Some(OsStr::new("")),
            Some(path_cargo.parent().unwrap().as_os_str()),
            None,
        );
        assert_eq!(got, path_cargo);
    }

    #[test]
    fn path_scan_takes_the_first_hit_in_order() {
        let s = Scratch::new("cargo-path-order");
        let first = s.path().join("a/cargo");
        let second = s.path().join("b/cargo");
        touch(&first);
        touch(&second);
        let joined = std::env::join_paths([
            s.path().join("empty-has-no-cargo"),
            first.parent().unwrap().to_path_buf(),
            second.parent().unwrap().to_path_buf(),
        ])
        .unwrap();
        let got = resolve_cargo_from(None, Some(&joined), None);
        assert_eq!(got, first);
    }

    #[test]
    fn bare_gui_path_falls_back_to_home_cargo_bin() {
        let s = Scratch::new("cargo-home");
        let home_cargo = s.path().join(".cargo/bin/cargo");
        touch(&home_cargo);
        // A bare PATH (no rustup shims anywhere on it).
        let bare = s.path().join("usr-bin-without-cargo");
        fs::create_dir_all(&bare).unwrap();
        let got = resolve_cargo_from(None, Some(bare.as_os_str()), Some(s.path()));
        assert_eq!(got, home_cargo);
    }

    #[test]
    fn nothing_found_yields_literal_cargo() {
        let s = Scratch::new("cargo-none");
        let empty = s.path().join("empty");
        fs::create_dir_all(&empty).unwrap();
        let got = resolve_cargo_from(None, Some(empty.as_os_str()), Some(s.path()));
        assert_eq!(got, PathBuf::from("cargo"));
        assert_eq!(resolve_cargo_from(None, None, None), PathBuf::from("cargo"));
    }

    #[test]
    fn log_ring_keeps_the_last_cap_lines_and_counts_drops() {
        let mut ring = LogRing::new(3);
        assert!(ring.is_empty());
        for i in 0..5 {
            ring.push(format!("line {i}"));
        }
        assert_eq!(ring.len(), 3);
        assert_eq!(ring.dropped(), 2);
        let kept: Vec<&str> = ring.lines().collect();
        assert_eq!(kept, vec!["line 2", "line 3", "line 4"]);
        ring.clear();
        assert!(ring.is_empty());
        assert_eq!(ring.dropped(), 0);
    }

    #[cfg(unix)]
    #[test]
    fn streams_merged_lines_and_delivers_the_exit_code() {
        let s = Scratch::new("spawn-stream");
        let handle = spawn_streaming(
            "/bin/sh",
            &["-c", "echo out1; echo err1 >&2; echo out2; exit 3"],
            s.path(),
            || {},
        );
        let mut lines = Vec::new();
        let mut code = None;
        for ev in handle.rx.iter() {
            match ev {
                ProcEvent::Line(l) => lines.push(l),
                ProcEvent::Exited { code: c } => {
                    code = Some(c);
                    break;
                }
                ProcEvent::SpawnFailed(e) => panic!("spawn failed: {e}"),
            }
        }
        assert_eq!(code, Some(Some(3)));
        lines.sort();
        assert_eq!(lines, vec!["err1", "out1", "out2"]);
    }

    #[cfg(unix)]
    #[test]
    fn kill_delivers_a_signal_exit() {
        let s = Scratch::new("spawn-kill");
        let handle = spawn_streaming("/bin/sh", &["-c", "sleep 30"], s.path(), || {});
        handle.kill();
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        loop {
            let left = deadline.saturating_duration_since(std::time::Instant::now());
            match handle.rx.recv_timeout(left) {
                Ok(ProcEvent::Exited { code }) => {
                    assert_eq!(code, None, "SIGKILL has no exit code");
                    break;
                }
                Ok(_) => {}
                Err(e) => panic!("no Exited event after kill: {e}"),
            }
        }
    }

    #[test]
    fn spawn_failure_is_an_event_not_a_panic() {
        let s = Scratch::new("spawn-enoent");
        let handle = spawn_streaming(
            s.path().join("no-such-binary-anywhere"),
            &[],
            s.path(),
            || {},
        );
        match handle.rx.recv_timeout(Duration::from_secs(5)) {
            Ok(ProcEvent::SpawnFailed(e)) => assert!(!e.is_empty()),
            other => panic!("expected SpawnFailed, got {other:?}"),
        }
    }
}
