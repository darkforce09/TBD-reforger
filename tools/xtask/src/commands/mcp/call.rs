//! `cargo xtask mcp call` — one Workbench tool call, through the daemon when it is up.
//!
//! Exit codes, pinned by `cargo xtask mcp selftest`: 0 success · 1 usage or empty after every
//! retry · 2 initialize failed · 3 JSON-RPC tool error · 4 timeout. Internal 9 means "the daemon
//! is unavailable, run the call one-shot".
//!
//! Three failures deliberately do not stop the call, because none of them says the tool call
//! cannot succeed:
//! - a lock that cannot be taken within 65 s — the lock only serialises daemon startup;
//! - an absent or unreadable npx download cache — the pinned package usually answers first;
//! - a daemon `status` or `start` that fails — the one-shot path still reaches the server.
//!
//! One failure does stop the attempt: a missing `timeout(1)`. The alternative is a call that
//! hangs for the whole `MCP_CALL_TIMEOUT` wall clock behind a racy kill thread, so the attempt
//! fails closed instead and maps to the empty-result code.

use std::env;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use developer_tools::enfusion_tooling::enfusion_mcp_entrypoint;
use verification_core::lock::flock_exclusive;
use verification_core::proc;

/// The usage line `cargo xtask mcp selftest` greps for.
const USAGE: &str = "usage: cargo xtask mcp call <tool> '<json-args>'";

/// Entry for `xtask mcp call [tool] [args-json]`.
pub fn run(tool: Option<String>, args_json: Option<String>) -> i32 {
    let Some(tool) = tool.filter(|t| !t.is_empty()) else {
        eprintln!("{USAGE}");
        return 1;
    };
    // An absent or empty argument object is the empty JSON object.
    let args = match args_json {
        None => "{}".to_string(),
        Some(s) if s.is_empty() => "{}".to_string(),
        Some(s) => s,
    };

    export_enfusion_defaults();
    let sock = resolve_sock();
    // SAFETY: set before any thread is spawned. The daemon and the `socket-send` helper read
    // MCP_SOCK from the environment they inherit.
    unsafe { env::set_var("MCP_SOCK", &sock) };

    let mut rc = daemon_try(&tool, &args, &sock);
    if rc == 9 {
        rc = oneshot(&tool, &args);
    }
    rc
}

fn export_enfusion_defaults() {
    let home = env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    set_default(
        "ENFUSION_GAME_PATH",
        &format!("{home}/.cache/enfusion-mcp-root"),
    );
    set_default(
        "ENFUSION_WORKBENCH_PATH",
        &format!("{home}/.local/share/Steam/steamapps/common/Arma Reforger Tools"),
    );
    set_default(
        "ENFUSION_PROJECT_PATH",
        &format!("{home}/Documents/Games/ArmaReforgerWorkbench/addons"),
    );
}

fn set_default(key: &str, val: &str) {
    if env::var_os(key).is_none() {
        unsafe { env::set_var(key, val) };
    }
}

fn resolve_sock() -> String {
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

fn xtask_bin() -> PathBuf {
    env::current_exe().unwrap_or_else(|_| PathBuf::from("xtask"))
}

fn dbg(msg: &str) {
    if env::var("MCP_DEBUG").ok().as_deref() == Some("1") {
        eprintln!("[mcp-call] {msg}");
    }
}

fn timeout_secs() -> u64 {
    env::var("MCP_CALL_TIMEOUT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(180)
}

fn retries() -> u32 {
    env::var("MCP_CALL_RETRIES")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1)
}

fn mktmp() -> PathBuf {
    let mut p = env::temp_dir();
    p.push(format!(
        "tbd-mcp-call-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    let _ = File::create(&p);
    p
}

fn emit_requests(tool: &str, args: &str) -> String {
    let call = format!(
        r#"{{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{{"name":"{tool}","arguments":{args}}}}}"#
    );
    format!(
        "{}\n{}\n{}\n",
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"cc","version":"1.0"}}}"#,
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
        call
    )
}

/// The argv that starts an `enfusion-mcp` server for the one-shot path.
fn resolve_runner() -> Vec<String> {
    let command = enfusion_mcp_entrypoint::resolve(&repository_root());
    dbg(&format!("runner={}", command.source.label()));
    command.argv()
}

/// The checkout to resolve the pinned server package against.
///
/// The cwd walk answers for the worktree the call is being made from. When it cannot (the call
/// was made from outside a checkout), the crate's own manifest directory locates the repository
/// that built this binary.
fn repository_root() -> PathBuf {
    crate::core::repository_root::find_repo_root()
        .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."))
}

fn ensure_daemon(sock: &str) -> bool {
    if env::var("MCP_NO_DAEMON").ok().as_deref() == Some("1") {
        return false;
    }
    if crate::commands::mcp::daemon::is_running_at(sock) {
        return true;
    }
    // One lock file per socket serialises the start, so two concurrent calls do not each spawn a
    // broker for the same socket.
    let lock_path = format!("{sock}.lock");
    // A lock that cannot be taken must not block the call: the daemon may already be starting.
    let _held = flock_exclusive(
        Path::new(&lock_path),
        Duration::from_secs(1),
        Duration::from_secs(65),
        |_| {},
    )
    .ok();
    if !crate::commands::mcp::daemon::is_running_at(sock) {
        let _ = crate::commands::mcp::daemon::start_at(sock, true);
    }
    drop(_held);
    crate::commands::mcp::daemon::is_running_at(sock)
}

/// 0 success · 3 tool error · 9 fall-back-to-oneshot
fn daemon_try(tool: &str, args: &str, sock: &str) -> i32 {
    if !ensure_daemon(sock) {
        dbg("daemon unavailable");
        return 9;
    }
    dbg(&format!("daemon_try TOOL=[{tool}] ARGS=[{args}]"));
    let outf = mktmp();
    let errf = mktmp();
    let xtask = xtask_bin();

    let mut send = match Command::new(&xtask)
        .args(["mcp", "socket-send", sock, tool, args])
        .stdout(Stdio::piped())
        .stderr(
            File::create(&errf)
                .ok()
                .map(Stdio::from)
                .unwrap_or_else(Stdio::null),
        )
        .spawn()
    {
        Ok(c) => c,
        Err(_) => {
            cleanup_tmps(&outf, &errf);
            return 9;
        }
    };
    let send_stdout = match send.stdout.take() {
        Some(s) => s,
        None => {
            let _ = send.kill();
            cleanup_tmps(&outf, &errf);
            return 9;
        }
    };
    let mut consume = match Command::new(&xtask)
        .args(["mcp", "consume"])
        .stdin(send_stdout)
        .stdout(
            File::create(&outf)
                .ok()
                .map(Stdio::from)
                .unwrap_or_else(Stdio::null),
        )
        .spawn()
    {
        Ok(c) => c,
        Err(_) => {
            let _ = send.kill();
            cleanup_tmps(&outf, &errf);
            return 9;
        }
    };
    let consume_rc = consume.wait().ok().and_then(|s| s.code()).unwrap_or(1);
    let send_rc = send.wait().ok().and_then(|s| s.code()).unwrap_or(1);
    dbg(&format!("daemon send_rc={send_rc} consume_rc={consume_rc}"));

    let result = if send_rc == 0 && consume_rc == 0 {
        cat_stdout(&outf);
        0
    } else if send_rc == 0 && consume_rc == 3 {
        3
    } else {
        if env::var("MCP_DEBUG").ok().as_deref() == Some("1") {
            let _ = cat_stderr_if_nonempty(&errf);
        }
        9
    };
    cleanup_tmps(&outf, &errf);
    result
}

fn oneshot(tool: &str, args: &str) -> i32 {
    let runner = resolve_runner();
    let timeout = timeout_secs();
    let max_retries = retries();
    let mut attempt = 0u32;
    loop {
        let outf = mktmp();
        let errf = mktmp();
        let (to_rc, consume_rc) = oneshot_pipe(&runner, tool, args, timeout, &outf, &errf);
        dbg(&format!(
            "oneshot attempt={attempt} to_rc={to_rc} consume_rc={consume_rc}"
        ));
        let code = if to_rc == 124 {
            4
        } else if consume_rc == 0 {
            cat_stdout(&outf);
            cleanup_tmps(&outf, &errf);
            return 0;
        } else if consume_rc == 3 {
            cleanup_tmps(&outf, &errf);
            return 3;
        } else if consume_rc == 2 {
            2
        } else {
            1
        };
        attempt += 1;
        if attempt > max_retries {
            let _ = cat_stderr_if_nonempty(&errf);
            cleanup_tmps(&outf, &errf);
            return code;
        }
        dbg(&format!(
            "retry ({attempt}/{max_retries}) after code={code}"
        ));
        cleanup_tmps(&outf, &errf);
    }
}

/// `emit | timeout RUNNER 2>errf | xtask mcp consume >outf` — returns (timeout rc, consume rc).
fn oneshot_pipe(
    runner: &[String],
    tool: &str,
    args: &str,
    timeout: u64,
    outf: &Path,
    errf: &Path,
) -> (i32, i32) {
    // Without `timeout(1)` an unresponsive server hangs this attempt for the whole wall clock.
    if proc::which("timeout").is_err() {
        dbg("timeout(1) absent — oneshot attempt fails closed");
        return (1, 1);
    }
    let reqs = emit_requests(tool, args);
    let xtask = xtask_bin();
    let err_file = match File::create(errf) {
        Ok(f) => f,
        Err(_) => return (1, 1),
    };
    let out_file = match File::create(outf) {
        Ok(f) => f,
        Err(_) => return (1, 1),
    };

    let mut cmd = Command::new("timeout");
    cmd.arg(timeout.to_string());
    if let Some((prog, rest)) = runner.split_first() {
        cmd.arg(prog);
        cmd.args(rest);
    }

    let mut child = match cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(err_file)
        .spawn()
    {
        Ok(c) => c,
        Err(_) => return (1, 1),
    };

    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(reqs.as_bytes());
    }
    let stdout = match child.stdout.take() {
        Some(s) => s,
        None => {
            let _ = child.kill();
            return (1, 1);
        }
    };

    let mut consume = match Command::new(&xtask)
        .args(["mcp", "consume"])
        .stdin(stdout)
        .stdout(out_file)
        .spawn()
    {
        Ok(c) => c,
        Err(_) => {
            let _ = child.kill();
            return (1, 1);
        }
    };

    let consume_rc = consume.wait().ok().and_then(|s| s.code()).unwrap_or(1);
    let to_rc = child.wait().ok().and_then(|s| s.code()).unwrap_or(1);
    (to_rc, consume_rc)
}

fn cleanup_tmps(a: &Path, b: &Path) {
    let _ = fs::remove_file(a);
    let _ = fs::remove_file(b);
}

fn cat_stdout(path: &Path) {
    if let Ok(mut f) = File::open(path) {
        let mut buf = Vec::new();
        let _ = f.read_to_end(&mut buf);
        let _ = std::io::stdout().write_all(&buf);
        let _ = std::io::stdout().flush();
    }
}

fn cat_stderr_if_nonempty(path: &Path) -> std::io::Result<()> {
    let meta = fs::metadata(path)?;
    if meta.len() == 0 {
        return Ok(());
    }
    let mut f = File::open(path)?;
    let mut buf = Vec::new();
    f.read_to_end(&mut buf)?;
    let _ = std::io::stderr().write_all(&buf);
    Ok(())
}

#[cfg(test)]
#[path = "tests/call/tests.rs"]
mod tests;
