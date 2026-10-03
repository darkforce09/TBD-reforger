//! Running a program off the UI thread and streaming its output.
//!
//! **Role:** `spawn_streaming`, `ProcessHandle` and `ProcessEvent`: merged output lines, then one
//! terminal event (exit code or spawn failure).
//! **Position:** part of `crate::core::process`; the desktop application runs the strict check,
//! `git status` and every ticket command through it.
//! **Signals & state:** reader and watcher threads per child, reporting over an `mpsc` channel; the
//! handle can kill the child from any thread.
//! **Invariants:** the UI thread never blocks; each process ends in exactly one terminal event; the
//! child runs through `process_runner`'s line stream, in its own process group, so a kill reaches
//! every process it started.

use super::*;
/// Exit-poll interval for the waiter thread. Polling (not `wait()`) is deliberate: a
/// grandchild that left the process group can hold the pipes open past the exit, so an
/// EOF-based wait could hang — `try_wait` sees the death regardless.
const WAIT_POLL: Duration = Duration::from_millis(25);

/// One event from a spawned subprocess, in arrival order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcessEvent {
    /// One line of merged stdout+stderr, verbatim (no trailing newline).
    Line(String),
    /// Terminal: the process exited.
    Exited {
        /// The exit code; `None` means killed by a signal.
        code: Option<i32>,
    },
    /// Terminal: the spawn itself failed (binary absent, permission…) — the cause
    /// `process_runner` reports. No `Exited` follows.
    SpawnFailed(String),
}

/// Handle to an in-flight subprocess: drain `rx` non-blocking from the UI;
/// `kill()` from any thread.
pub struct ProcessHandle {
    /// The events in arrival order; drain it without blocking.
    pub rx: Receiver<ProcessEvent>,
    shared: Arc<Mutex<Shared>>,
}

struct Shared {
    child: Option<StreamingChild>,
    kill_requested: bool,
}

impl ProcessHandle {
    /// Kill the subprocess and every process in its group (SIGKILL). Safe before the
    /// spawn lands (the worker honors the request) and after exit (no-op).
    pub fn kill(&self) {
        let mut shared = self.shared.lock().expect("subproc mutex");
        shared.kill_requested = true;
        if let Some(child) = shared.child.as_mut() {
            child.kill();
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
) -> ProcessHandle {
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
    ProcessHandle { rx, shared }
}

fn run_child(
    program: &OsStr,
    args: &[OsString],
    cwd: &Path,
    shared: &Arc<Mutex<Shared>>,
    tx: &Sender<ProcessEvent>,
    on_event: &Arc<impl Fn() + Send + Sync + 'static>,
) {
    let spawned = Run::new(program).args(args).cwd(cwd).stream_lines();
    let (mut child, lines) = match spawned {
        Ok(started) => started,
        Err(cause) => {
            let _ = tx.send(ProcessEvent::SpawnFailed(cause.to_string()));
            on_event();
            return;
        }
    };
    {
        let mut lock = shared.lock().expect("subproc mutex");
        if lock.kill_requested {
            child.kill();
        }
        lock.child = Some(child);
    }
    // The line stream's own readers own the pipes, so `try_wait`/`kill` never contend with a
    // blocked read; this thread forwards each line and wakes the UI.
    {
        let tx = tx.clone();
        let on_event = Arc::clone(on_event);
        thread::spawn(move || forward_lines(&lines, &tx, &on_event));
    }
    loop {
        let code = {
            let mut lock = shared.lock().expect("subproc mutex");
            let exited = match lock.child.as_mut().map(StreamingChild::try_wait) {
                Some(Ok(Some(code))) => Some(Some(code)),
                Some(Err(NotRun::Signalled { .. })) => Some(None),
                _ => None,
            };
            if exited.is_some() {
                lock.child = None;
            }
            exited
        };
        if let Some(code) = code {
            // Give the readers a moment to flush buffered tail lines so `Exited`
            // lands after the output it explains (best effort — UI order only).
            thread::sleep(WAIT_POLL);
            let _ = tx.send(ProcessEvent::Exited { code });
            on_event();
            return;
        }
        thread::sleep(WAIT_POLL);
    }
}

/// Forward each streamed line until both pipes reach EOF. A failed send means the UI
/// dropped the receiver — stop quietly.
fn forward_lines(lines: &Receiver<String>, tx: &Sender<ProcessEvent>, on_event: &Arc<impl Fn()>) {
    for line in lines {
        if tx.send(ProcessEvent::Line(line)).is_err() {
            return;
        }
        on_event();
    }
}
