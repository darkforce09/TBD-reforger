use std::path::Path;
/// Open a path with the OS handler: `xdg-open` (cfg-gated `start` on Windows).
/// Spawn-and-forget — the UI thread never waits on the child.
pub(crate) fn open_path(path: &Path) {
    #[cfg(target_os = "windows")]
    let spawned = std::process::Command::new("cmd")
        .arg("/C")
        .arg("start")
        .arg("")
        .arg(path)
        .spawn();
    #[cfg(not(target_os = "windows"))]
    let spawned = std::process::Command::new("xdg-open").arg(path).spawn();
    if let Err(e) = spawned {
        eprintln!("ticketboard: opening {} failed: {e}", path.display());
    }
}
