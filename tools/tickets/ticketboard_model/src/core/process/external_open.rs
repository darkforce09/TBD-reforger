//! Opening a path with the operating system's handler.
//!
//! **Role:** `open_path`, which hands a path to `xdg-open` (or `start` on Windows).
//! **Position:** called by the desktop application for "open externally" in the ticket details and
//! the document column.
//! **Signals & state:** none here; `process_runner`'s reaper thread waits on the detached child.
//! **Invariants:** spawn-and-forget: the UI thread never waits on the child, which runs detached
//! in its own session, and a failed spawn is reported on standard error, never as a panic.

use std::path::Path;

use process_runner::Run;

/// Open a path with the OS handler: `xdg-open` (cfg-gated `start` on Windows).
/// Spawn-and-forget — the UI thread never waits on the child.
pub fn open_path(path: &Path) {
    #[cfg(target_os = "windows")]
    let spawned = Run::new("cmd")
        .arg("/C")
        .arg("start")
        .arg("")
        .arg(path)
        .spawn_detached();
    #[cfg(not(target_os = "windows"))]
    let spawned = Run::new("xdg-open").arg(path).spawn_detached();
    if let Err(e) = spawned {
        eprintln!("ticketboard: opening {} failed: {e}", path.display());
    }
}
