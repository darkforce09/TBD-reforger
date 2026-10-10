//! `cargo xtask mcp daemon` — the broker's lifecycle: start, stop, status, restart, stop-all.
//!
//! **Role:** builds, starts, probes and stops the `mcpd` broker of one socket, and stops every
//! broker and orphaned server at once.
//! **Position:** called by `crate::dispatch` for `daemon`, by `crate::call` before a daemon
//! call, by `crate::call_selftest`, and by xtask's mod development bootstrap; builds `mcpd`
//! with `cargo build -p developer_tools --bin mcpd`.
//! **Signals & state:** the broker's state is on disk beside its socket: `<socket>.pid`,
//! `<socket>.log`, and the socket itself.
//! **Invariants:** a broker is running only when its socket accepts a connection; every exit
//! code is the one [`cmd`] documents.
//!
//! Starting builds `mcpd`, hands it the `enfusion-mcp` entry
//! [`crate::server_entrypoint`] resolved, and spawns it in its
//! own session so its argv stays `mcpd --socket` for the process matching that stop-all does.
//! Liveness is a connect probe through `crate::json_rpc::cmd_probe_sock`, not a
//! file check: a socket file outlives the process that bound it.
//!
//! Signalling and file removal are best-effort throughout — a `kill` that finds no process, an
//! already-removed socket and an unreadable npm cache all mean the same thing as success here,
//! so none of them is raised.

use std::env;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use process_runner::Run;

use repository_layout::build_output::{
    MCP_DAEMON_SUBFOLDER, ToolchainEnvironment, build_output_subfolder,
};
use repository_root::find_repository_root;

use crate::server_entrypoint;

const USAGE: &str = "usage: cargo xtask mcp daemon {start|stop|status|restart|stop-all}";

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

/// Socket path: `MCP_SOCK`, else one named for the uid under `XDG_RUNTIME_DIR`. Re-seated
/// under `/tmp` past 100 bytes, because an AF_UNIX path caps at 108 including the terminator.
// `getuid` cannot fail and reads no memory; std offers no safe call for it.
#[allow(unsafe_code)]
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
    crate::json_rpc::cmd_probe_sock(sock) == 0
}

/// Start the broker. When `quiet`, print nothing — the call path starts it opportunistically.
// The `pre_exec` hook below only calls `setsid`, which is async-signal-safe, between fork and
// exec.
#[allow(unsafe_code)]
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

    let root = find_repository_root().unwrap_or_else(|_| PathBuf::from("."));
    let entry = server_entrypoint::resolve(&root).entry_path;
    // The child inherits these; every one has a default so a bare shell can start the daemon.
    let game = env::var("ENFUSION_GAME_PATH").unwrap_or_else(|_| default_game_path(&root));
    let wb = env::var("ENFUSION_WORKBENCH_PATH").unwrap_or_else(|_| default_workbench_path());
    let project = env::var("ENFUSION_PROJECT_PATH").unwrap_or_else(|_| default_project_path());

    let mcpd_target = env::var("MCPD_CARGO_TARGET_DIR")
        .unwrap_or_else(|_| default_mcpd_target_dir(&root).display().to_string());

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

    // Detached: the broker outlives this process, in a new session that keeps its argv
    // `mcpd --socket`, which is what stop-all matches on; init adopts it once xtask exits.
    let mut broker = Run::new(&mcpd_bin)
        .args(["--socket", sock, "--pidfile", &pidfile])
        .env("ENFUSION_GAME_PATH", &game)
        .env("ENFUSION_WORKBENCH_PATH", &wb)
        .env("ENFUSION_PROJECT_PATH", &project)
        .env("MCP_SOCK", sock);
    if let Some(entry) = entry.as_ref() {
        broker = broker.env("ENFUSION_MCP_BIN", entry);
    }
    if let Err(e) = broker.spawn_detached_to_files(log, log_err) {
        if !quiet {
            eprintln!("mcp-daemon: failed to spawn mcpd: {e}");
        }
        return 1;
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
            signal_best_effort(Run::new("kill").arg(pid_s));
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
    let pids = match Run::new("pgrep").args(["-f", "mcpd --socket"]).output() {
        Ok(o) if o.code == 0 => o.stdout,
        _ => String::new(),
    };
    if !pids.trim().is_empty() {
        for pid in pids.split_whitespace() {
            signal_best_effort(Run::new("kill").arg(pid));
        }
        thread::sleep(Duration::from_secs(1));
        for pid in pids.split_whitespace() {
            signal_best_effort(Run::new("kill").args(["-9", pid]));
        }
    }
    // A broker that died without reaping leaves its server child running; it is matched by the
    // installed module's path, which is the one thing every tier of the resolution has in common.
    signal_best_effort(Run::new("pkill").args(["-9", "-f", &server_entrypoint::process_pattern()]));

    let xdg = env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".into());
    rm_tbd_mcp_globs(Path::new(&xdg));
    rm_tbd_mcp_globs(Path::new("/tmp"));

    println!("mcp-daemon: stop-all done");
    0
}

/// Runs one best-effort signal command and passes its output through, as a child sharing this
/// process's stdout and stderr would; its status and a failure to run it are ignored.
fn signal_best_effort(run: Run) {
    if let Ok(o) = run.output() {
        let _ = io::stdout().write_all(o.stdout.as_bytes());
        let _ = io::stderr().write_all(o.stderr.as_bytes());
    }
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

/// The folder `mcpd` builds into when `MCPD_CARGO_TARGET_DIR` is unset:
/// `<checkout>/target/<environment>/dev-mcpd`, private so a gate never rewrites the daemon's
/// binary.
fn default_mcpd_target_dir(root: &Path) -> PathBuf {
    let environment =
        ToolchainEnvironment::from_container_flag(process_runner::host_execution::in_container());
    build_output_subfolder(root, environment, MCP_DAEMON_SUBFOLDER)
}

/// Build `mcpd` quietly into its own target directory, forwarding cargo's stderr.
fn build_mcpd(root: &Path, mcpd_target: &str) -> Result<(), ()> {
    let o = match Run::new("cargo")
        .arg("build")
        .arg("-q")
        .arg("-p")
        .arg("developer_tools")
        .arg("--bin")
        .arg("mcpd")
        .cwd(root)
        .env("CARGO_TARGET_DIR", mcpd_target)
        .output()
    {
        Ok(o) => o,
        Err(_) => return Err(()),
    };
    let _ = io::stderr().write_all(o.stderr.as_bytes());
    if o.code != 0 {
        return Err(());
    }
    Ok(())
}

fn is_executable(path: &Path) -> bool {
    match path.metadata() {
        Ok(m) if m.is_file() => m.permissions().mode() & 0o111 != 0,
        _ => false,
    }
}

/// The game root the MCP server reads when `ENFUSION_GAME_PATH` is unset:
/// `<checkout>/.workstation/enfusion_mcp_game_root`.
fn default_game_path(root: &Path) -> String {
    root.join(repository_layout::ENFUSION_MCP_GAME_ROOT)
        .display()
        .to_string()
}

fn default_workbench_path() -> String {
    let home = env::var("HOME").unwrap_or_else(|_| ".".into());
    format!("{home}/.local/share/Steam/steamapps/common/Arma Reforger Tools")
}

fn default_project_path() -> String {
    let home = env::var("HOME").unwrap_or_else(|_| ".".into());
    format!("{home}/Documents/Games/ArmaReforgerWorkbench/addons")
}
