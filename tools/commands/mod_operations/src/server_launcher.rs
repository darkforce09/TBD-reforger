//! A dedicated server launcher the caller polls through the server's own logs, then stops.
//!
//! **Role:** [`start`] runs a launcher (the host bridge in front of `env -C … setsid sh -c …`) with
//! its output lines written to a log file or dropped on a thread of their own; [`stop`] gives the
//! launcher two seconds to exit after the caller has killed the server's process group, then kills
//! the launcher's own group and reaps it.
//! **Position:** the compile gate's server and calibration runs and the world boot's server use it;
//! the server's process group is stopped by the caller's `kill_run` from the pidfile the launch
//! script writes, never through this handle.
//! **Signals & state:** one thread per launcher forwards its lines until both pipes close.
//! **Invariants:** a full pipe never blocks the launcher (its lines are always read); stopping a
//! launcher that already exited is no error.

use std::fs::File;
use std::io::Write;
use std::thread;
use std::time::Duration;

use process_runner::{Run, StreamingChild};
use verification_core::NotRun;

/// Start `launcher`, writing each of its output lines to `log` (or dropping it when `log` is
/// `None`), and return its handle.
pub(crate) fn start(launcher: Run, log: Option<File>) -> Result<StreamingChild, NotRun> {
    let (child, lines) = launcher.stream_lines()?;
    thread::spawn(move || {
        let mut log = log;
        for line in lines {
            if let Some(file) = log.as_mut() {
                let _ = writeln!(file, "{line}");
            }
        }
    });
    Ok(child)
}

/// Wait up to ten 200 ms polls for `launcher` to exit, then kill its process group and reap it.
pub(crate) fn stop(mut launcher: StreamingChild) {
    for _ in 0..10 {
        // An exit code or a signal death both mean the launcher is gone.
        if !matches!(launcher.try_wait(), Ok(None)) {
            break;
        }
        thread::sleep(Duration::from_millis(200));
    }
    launcher.kill();
    let _ = launcher.wait();
}
