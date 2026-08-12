//! T-888 — port of `scripts/mod/mcp-daemon.sh` → `cargo xtask mcp daemon`.
//!
//! setsid + AF_UNIX socket lifecycle: start / stop / status / restart / stop-all.
//! Builds `mcpd` in-process (former `lib/mcpd-bin.sh`) and probes via
//! [`crate::mcp::cmd_probe_sock`] (former `lib/xtask-run.sh mcp probe-sock`).
//! Lib scripts themselves stay on disk for T-879 — this module is their last
//! live caller removal for the daemon path.
//!
//! Fail-opens pinned (bash parity):
//! - `pgrep` / `pkill` / `kill` errors discarded (`2>/dev/null || true`)
//! - `rm -f` of socket globs ignores missing paths
//! - `resolve_bin` find under `~/.npm/_npx` swallows walk errors (`2>/dev/null`)
//!
//! Usage string keeps the historical script name for byte parity.

use std::env;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

use tbd_gate::proc::Run;

use crate::mcp;
use crate::root::find_repo_root;

const USAGE: &str = "usage: mcp-daemon.sh {start|stop|status|restart|stop-all}";

/// CLI entry: `cargo xtask mcp daemon [ACTION]` (default `status`).
pub fn cmd(action: Option<&str>) -> i32 {
    match action.unwrap_or("status") {
        "start" => start_at(&resolve_sock(), false),
        "stop" => stop_at(&resolve_sock(), false),
        "status" => status_at(&resolve_sock(), false),
        "restart" => {
            let sock = resolve_sock();
            let _ = stop_at(&sock, false);
            start_at(&sock, false)
        }
        "stop-all" => stop_all(),
        _ => {
            eprintln!("{USAGE}");
            2
        }
    }
}

/// Socket path — mirrors bash `MCP_SOCK` / `XDG_RUNTIME_DIR` / AF_UNIX 108-byte cap.
pub fn resolve_sock() -> String {
    let uid = unsafe { libc::getuid() };
    let mut sock = env::var("MCP_SOCK").unwrap_or_else(|_| {
        let base = env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".into());
        format!("{base}/tbd-mcp-{uid}.sock")
    });
    if sock.len() > 100 {
        sock = format!("/tmp/tbd-mcp-{uid}.sock");
    }
    sock
}

/// Live check: socket file must exist as a socket AND be connectable.
pub fn is_running_at(sock: &str) -> bool {
    let meta = match fs::symlink_metadata(sock) {
        Ok(m) => m,
        Err(_) => return false,
    };
    use std::os::unix::fs::FileTypeExt;
    if !meta.file_type().is_socket() {
        return false;
    }
    mcp::cmd_probe_sock(sock) == 0
}

/// Start the broker. When `quiet`, suppress messages (bash `>/dev/null 2>&1`).
pub fn start_at(sock: &str, quiet: bool) -> i32 {
    if is_running_at(sock) {
        if !quiet {
            println!("mcp-daemon: already running ({sock})");
        }
        return 0;
    }
    if Path::new(sock).exists() {
        let _ = fs::remove_file(sock); // stale socket
    }

    let script_dir = scripts_mod();
    let root = script_dir
        .parent()
        .and_then(|p| p.parent())
        .map(Path::to_path_buf)
        .unwrap_or_else(|| find_repo_root().unwrap_or_else(|_| script_dir.clone()));

    let bin = resolve_bin(&script_dir);
    // Match bash `export` defaults before spawn (child inherits).
    let game = env::var("ENFUSION_GAME_PATH").unwrap_or_else(|_| default_game_path());
    let wb = env::var("ENFUSION_WORKBENCH_PATH").unwrap_or_else(|_| default_workbench_path());
    let project = env::var("ENFUSION_PROJECT_PATH").unwrap_or_else(|_| default_project_path());

    let mcpd_target = env::var("MCPD_CARGO_TARGET_DIR")
        .unwrap_or_else(|_| root.join("target-dev-mcpd").display().to_string());

    // Former mcpd-bin.sh: cargo build -q; stdout (path echo) discarded; stderr passes.
    match build_mcpd(&root, &mcpd_target) {
        Ok(()) => {}
        Err(()) => {
            if !quiet {
                eprintln!("mcp-daemon: mcpd build failed");
            }
            return 1;
        }
    }
    let mcpd_bin = format!("{mcpd_target}/debug/mcpd");
    if !is_executable(Path::new(&mcpd_bin)) {
        if !quiet {
            eprintln!("mcp-daemon: mcpd binary missing at {mcpd_bin}");
        }
        return 1;
    }

    let pidfile = format!("{sock}.pid");
    let log_path = format!("{sock}.log");
    let log = match OpenOptions::new().create(true).append(true).open(&log_path) {
        Ok(f) => f,
        Err(e) => {
            if !quiet {
                eprintln!("mcp-daemon: failed to open log {log_path}: {e}");
            }
            return 1;
        }
    };
    let log_err = match log.try_clone() {
        Ok(f) => f,
        Err(e) => {
            if !quiet {
                eprintln!("mcp-daemon: failed to clone log: {e}");
            }
            return 1;
        }
    };

    let mut cmd = Command::new(&mcpd_bin);
    cmd.args(["--socket", sock, "--pidfile", &pidfile])
        .stdout(Stdio::from(log))
        .stderr(Stdio::from(log_err))
        .env("ENFUSION_GAME_PATH", &game)
        .env("ENFUSION_WORKBENCH_PATH", &wb)
        .env("ENFUSION_PROJECT_PATH", &project)
        .env("MCP_SOCK", sock);
    if let Some(b) = bin.as_ref() {
        cmd.env("ENFUSION_MCP_BIN", b);
    }
    // bash: `setsid "$mcpd_bin" … &` — new session so argv stays `mcpd --socket`.
    unsafe {
        cmd.pre_exec(|| {
            if libc::setsid() == -1 {
                Err(io::Error::last_os_error())
            } else {
                Ok(())
            }
        });
    }
    match cmd.spawn() {
        Ok(child) => {
            // Detach: parent exits after poll; init reaps. Dropping Child would
            // briefly zombie until xtask exits — forget to match bash `&`.
            std::mem::forget(child);
        }
        Err(e) => {
            if !quiet {
                eprintln!("mcp-daemon: failed to spawn mcpd: {e}");
            }
            return 1;
        }
    }

    for _ in 0..120 {
        if is_running_at(sock) {
            if !quiet {
                println!("mcp-daemon: started ({sock})");
            }
            return 0;
        }
        thread::sleep(Duration::from_millis(500));
    }
    if !quiet {
        eprintln!("mcp-daemon: failed to start (see {log_path})");
    }
    1
}

/// Stop the broker for `sock`.
pub fn stop_at(sock: &str, quiet: bool) -> i32 {
    let pidfile = format!("{sock}.pid");
    if let Ok(pid_s) = fs::read_to_string(&pidfile) {
        let pid_s = pid_s.trim();
        if !pid_s.is_empty() {
            let _ = Command::new("kill").arg(pid_s).status(); // 2>/dev/null
        }
    }
    let _ = fs::remove_file(sock);
    let _ = fs::remove_file(&pidfile);
    if !quiet {
        println!("mcp-daemon: stopped");
    }
    0
}

/// Status for `sock` — rc 0 running, 1 stopped.
pub fn status_at(sock: &str, quiet: bool) -> i32 {
    if is_running_at(sock) {
        let pid = fs::read_to_string(format!("{sock}.pid"))
            .unwrap_or_default()
            .trim()
            .to_string();
        if !quiet {
            println!("running ({sock}, pid {pid})");
        }
        0
    } else {
        if !quiet {
            println!("stopped");
        }
        1
    }
}

/// Nuke every tbd MCP broker (any socket) + orphaned enfusion-mcp servers.
pub fn stop_all() -> i32 {
    // pids="$(pgrep -f 'mcpd --socket' 2>/dev/null || true)"
    let pids = match Command::new("pgrep").args(["-f", "mcpd --socket"]).output() {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).into_owned(),
        _ => String::new(),
    };
    if !pids.trim().is_empty() {
        for pid in pids.split_whitespace() {
            let _ = Command::new("kill").arg(pid).status();
        }
        thread::sleep(Duration::from_secs(1));
        for pid in pids.split_whitespace() {
            let _ = Command::new("kill").args(["-9", pid]).status();
        }
    }
    // pkill -9 -f 'node_modules/enfusion-mcp/dist/index\.js' 2>/dev/null || true
    let _ = Command::new("pkill")
        .args(["-9", "-f", r"node_modules/enfusion-mcp/dist/index\.js"])
        .status();

    let xdg = env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".into());
    rm_tbd_mcp_globs(Path::new(&xdg));
    rm_tbd_mcp_globs(Path::new("/tmp"));

    println!("mcp-daemon: stop-all done");
    0
}

fn rm_tbd_mcp_globs(dir: &Path) {
    let rd = match fs::read_dir(dir) {
        Ok(rd) => rd,
        Err(_) => return,
    };
    for ent in rd.flatten() {
        let name = ent.file_name();
        let n = name.to_string_lossy();
        if n.starts_with("tbd-mcp-") {
            let _ = fs::remove_file(ent.path());
        }
    }
}

/// 4-tier resolve of the enfusion-mcp entry (mirrors mcp-call / bash resolve_bin).
fn resolve_bin(script_dir: &Path) -> Option<String> {
    if let Ok(bin) = env::var("ENFUSION_MCP_BIN") {
        if Path::new(&bin).is_file() {
            return Some(bin);
        }
    }
    let pinned = script_dir.join("node_modules/enfusion-mcp/dist/index.js");
    if pinned.is_file() {
        return Some(pinned.to_string_lossy().into_owned());
    }
    // PINNED fail-open: bash `find … 2>/dev/null | head -1` (readdir order).
    find_npx_enfusion_first()
}

fn find_npx_enfusion_first() -> Option<String> {
    let home = env::var("HOME").ok()?;
    let root = PathBuf::from(home).join(".npm/_npx");
    if !root.is_dir() {
        return None;
    }
    let mut hits = Vec::new();
    let _ = visit_depth(&root, 0, 4, &mut hits);
    hits.into_iter().next()
}

fn visit_depth(dir: &Path, depth: u32, max: u32, out: &mut Vec<String>) -> io::Result<()> {
    if depth > max {
        return Ok(());
    }
    let rd = match fs::read_dir(dir) {
        Ok(rd) => rd,
        Err(_) => return Ok(()),
    };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            visit_depth(&p, depth + 1, max, out)?;
        } else if p.is_file() {
            let s = p.to_string_lossy();
            if s.contains("enfusion-mcp/dist/index.js") {
                out.push(s.into_owned());
            }
        }
    }
    Ok(())
}

/// Former `lib/mcpd-bin.sh`: quiet build, honor `CARGO_TARGET_DIR` (= mcpd_target here).
/// Forwards cargo stderr; discards stdout (path echo).
fn build_mcpd(root: &Path, mcpd_target: &str) -> Result<(), ()> {
    let o = match Run::new("cargo")
        .arg("build")
        .arg("-q")
        .arg("-p")
        .arg("tbd-tools")
        .arg("--bin")
        .arg("mcpd")
        .cwd(root)
        .env("CARGO_TARGET_DIR", mcpd_target)
        .output()
    {
        Ok(o) => o,
        Err(_) => return Err(()),
    };
    // bash: mcpd-bin stdout → /dev/null; stderr inherits.
    let _ = io::stderr().write_all(o.stderr.as_bytes());
    if o.code != 0 {
        return Err(());
    }
    Ok(())
}

fn scripts_mod() -> PathBuf {
    find_repo_root()
        .map(|r| r.join("scripts/mod"))
        .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../scripts/mod"))
}

fn is_executable(path: &Path) -> bool {
    match path.metadata() {
        Ok(m) if m.is_file() => m.permissions().mode() & 0o111 != 0,
        _ => false,
    }
}

fn default_game_path() -> String {
    let home = env::var("HOME").unwrap_or_else(|_| ".".into());
    format!("{home}/.cache/enfusion-mcp-root")
}

fn default_workbench_path() -> String {
    let home = env::var("HOME").unwrap_or_else(|_| ".".into());
    format!("{home}/.local/share/Steam/steamapps/common/Arma Reforger Tools")
}

fn default_project_path() -> String {
    let home = env::var("HOME").unwrap_or_else(|_| ".".into());
    format!("{home}/Documents/Games/ArmaReforgerWorkbench/addons")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usage_rejects_unknown_action() {
        assert_eq!(cmd(Some("bogus")), 2);
    }

    #[test]
    fn status_stopped_when_no_socket() {
        let sock = format!("/tmp/tbd-mcp-t888-ut-status-{}.sock", std::process::id());
        let _ = fs::remove_file(&sock);
        let _ = fs::remove_file(format!("{sock}.pid"));
        // Isolate from ambient MCP_SOCK
        let code = status_at(&sock, true);
        assert_eq!(code, 1);
    }
}
