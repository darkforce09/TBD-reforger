//! The compile gate's host bridge, its process-group kill and its signal cleanup.
//!
//! **Role:** [`hostrun`] runs a command on the real machine (through `distrobox-host-exec` or
//! `host-spawn` inside a container), [`kill_run`] stops a server's whole process group, and
//! [`Session`] arms the SIGINT and SIGTERM cleanup that stops the server and removes the run folder.
//! **Position:** used by [`crate::compile`]; every child runs through `process_runner`.
//! **Signals & state:** one process-wide cleanup record and its `CLEANED` flag, set by
//! [`Session::install`] and taken once by whichever of the session's drop and a signal comes first.
//! **Invariants:** the cleanup runs at most once; a signal ends the process with 130 (SIGINT) or
//! 143
//! (SIGTERM) after it; the container test also honours a `container` variable, the marker the
//! ticket manager's host bridge reads (`ticket_manager_execution.toml`).

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use process_runner::Run;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

pub(crate) fn in_container() -> bool {
    Path::new("/run/.containerenv").is_file()
        || Path::new("/.dockerenv").is_file()
        || std::env::var_os("container").is_some()
}

fn host_bridge() -> Option<&'static str> {
    for b in ["distrobox-host-exec", "host-spawn"] {
        if Run::new("sh")
            .args(["-c", &format!("command -v {b} >/dev/null 2>&1")])
            .status()
            .is_ok_and(|code| code == 0)
        {
            return Some(b);
        }
    }
    None
}

pub(crate) fn require_host() -> Result<(), ()> {
    if !in_container() || host_bridge().is_some() {
        Ok(())
    } else {
        Err(())
    }
}

/// `program` with `args` as a run on the real machine: behind the host bridge inside a container,
/// directly on the host.
pub(crate) fn hostrun(program: &str, args: &[&str]) -> Run {
    if in_container()
        && let Some(bridge) = host_bridge()
    {
        return Run::new(bridge).arg(program).args(args);
    }
    Run::new(program).args(args)
}

pub(crate) fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    match path.metadata() {
        Ok(m) if m.is_file() => m.permissions().mode() & 0o111 != 0,
        _ => false,
    }
}

pub(crate) fn mktemp_dir(prefix: &str) -> io::Result<PathBuf> {
    let base = PathBuf::from(std::env::var_os("TMPDIR").unwrap_or_else(|| "/tmp".into()));
    for i in 0..32u32 {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let p = base.join(format!("{prefix}.{}.{nanos}.{i}", std::process::id()));
        match fs::create_dir(&p) {
            Ok(()) => return Ok(p),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        }
    }
    Err(io::Error::other("mktemp exhausted"))
}

pub(crate) fn kill_run(run_dir: &Path) {
    let Ok(pgid) = fs::read_to_string(run_dir.join("server.pid")) else {
        return;
    };
    let pgid = pgid.trim();
    if pgid.is_empty() {
        return;
    }
    let neg = format!("-{pgid}");
    let _ = hostrun("kill", &["-TERM", "--", &neg]).status();
    for _ in 0..10 {
        let alive = hostrun("kill", &["-0", "--", &neg])
            .status()
            .is_ok_and(|code| code == 0);
        if !alive {
            return;
        }
        thread::sleep(Duration::from_millis(200));
    }
    let _ = hostrun("kill", &["-9", "--", &neg]).status();
}

struct CleanupBits {
    run_dir: PathBuf,
    keep_logs: bool,
    cal_dir: Option<PathBuf>,
}

pub(crate) struct Session;

static CLEANED: AtomicBool = AtomicBool::new(false);
static BITS: Mutex<Option<CleanupBits>> = Mutex::new(None);

impl Session {
    // `signal` is a raw libc call; the handlers run the cleanup and `_exit`.
    #[allow(unsafe_code)]
    pub(crate) fn install(run_dir: PathBuf, keep_logs: bool) -> Session {
        CLEANED.store(false, Ordering::SeqCst);
        *BITS
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(CleanupBits {
            run_dir,
            keep_logs,
            cal_dir: None,
        });
        unsafe {
            libc::signal(libc::SIGINT, on_int as *const () as libc::sighandler_t);
            libc::signal(libc::SIGTERM, on_term as *const () as libc::sighandler_t);
        }
        Session
    }

    pub(crate) fn set_cal(cal: Option<PathBuf>) {
        if let Ok(mut g) = BITS.lock()
            && let Some(b) = g.as_mut()
        {
            b.cal_dir = cal;
        }
    }

    pub(crate) fn finish(self) {
        cleanup_now();
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        cleanup_now();
    }
}

fn cleanup_now() {
    if CLEANED.swap(true, Ordering::SeqCst) {
        return;
    }
    let Some(bits) = BITS.lock().ok().and_then(|mut g| g.take()) else {
        return;
    };
    kill_run(&bits.run_dir);
    if let Some(cal) = &bits.cal_dir
        && let Ok(pgid) = fs::read_to_string(cal.join("server.pid"))
    {
        let pgid = pgid.trim();
        if !pgid.is_empty() {
            let _ = hostrun("kill", &["-9", "--", &format!("-{pgid}")]).status();
        }
    }
    if bits.keep_logs {
        println!("  (logs kept: {})", bits.run_dir.display());
        let _ = io::stdout().flush();
    } else {
        let _ = fs::remove_dir_all(&bits.run_dir);
    }
}

// `_exit` is a raw libc call: it ends the process without running Rust destructors twice.
#[allow(unsafe_code)]
extern "C" fn on_int(_: libc::c_int) {
    cleanup_now();
    unsafe { libc::_exit(130) };
}

// `_exit` is a raw libc call: it ends the process without running Rust destructors twice.
#[allow(unsafe_code)]
extern "C" fn on_term(_: libc::c_int) {
    cleanup_now();
    unsafe { libc::_exit(143) };
}
