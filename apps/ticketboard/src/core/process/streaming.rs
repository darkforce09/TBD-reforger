use super::*;
/// Exit-poll interval for the waiter thread. Polling (not `wait()`) is deliberate:
/// killing `cargo run` leaves the grandchild binary holding the pipes open, so an
/// EOF-based wait would hang — `try_wait` sees the death regardless.
const WAIT_POLL: Duration = Duration::from_millis(25);

/// One event from a spawned subprocess, in arrival order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcessEvent {
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
pub struct ProcessHandle {
    pub rx: Receiver<ProcessEvent>,
    shared: Arc<Mutex<Shared>>,
}

struct Shared {
    child: Option<Child>,
    kill_requested: bool,
}

impl ProcessHandle {
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
            let _ = tx.send(ProcessEvent::SpawnFailed(e.to_string()));
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
            let _ = tx.send(ProcessEvent::Exited {
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
fn stream_lines(pipe: impl Read, tx: &Sender<ProcessEvent>, on_event: &Arc<impl Fn()>) {
    let reader = BufReader::new(pipe);
    for line in reader.lines() {
        let Ok(line) = line else { return };
        if tx.send(ProcessEvent::Line(line)).is_err() {
            return;
        }
        on_event();
    }
}
