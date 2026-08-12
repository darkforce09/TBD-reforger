//! T-860 — port of `scripts/mod/mcp-call.sh` → `cargo xtask mcp call`.
//!
//! Exit codes (locked by mcp-call-selftest): 0 success · 1 usage/empty-after-retries ·
//! 2 init-failed · 3 JSON-RPC tool error · 4 timeout. Internal 9 = fall back to one-shot.
//!
//! Fail-opens pinned (bash parity — changing them is a behaviour change):
//! - `flock -w 65 … 2>/dev/null || true` — lock failure must not block the call.
//! - `find ~/.npm/_npx … 2>/dev/null | head -1` — missing npx cache is not fatal.
//! - daemon `status`/`start` stderr discarded; unavailable daemon → oneshot (rc 9 path).
//!
//! Fail-open closed: bash assumed `timeout(1)` on PATH. If absent we fail the attempt with
//! rc mapping to empty/fail (1) rather than hanging until MCP_CALL_TIMEOUT wall-clock via a
//! racy kill thread.
//!
//! Preserved oddity: usage text still names `mcp-call.sh` (byte-parity with bash baseline /
//! T6 selftest `grep -q usage`).

use std::env;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use tbd_gate::lock::flock_exclusive;
use tbd_gate::proc::{self, Run};

/// Byte-identical to bash usage line (T6 + `/tmp/t853/w220/t860/usage`).
const USAGE: &str = "usage: mcp-call.sh <tool> '<json-args>'";

/// Entry for `xtask mcp call [tool] [args-json]`.
pub fn run(tool: Option<String>, args_json: Option<String>) -> i32 {
    let Some(tool) = tool.filter(|t| !t.is_empty()) else {
        eprintln!("{USAGE}");
        return 1;
    };
    // Bash: empty $2 → `{}` (never `${2:-{}}` — that appends a stray `}`).
    let args = match args_json {
        None => "{}".to_string(),
        Some(s) if s.is_empty() => "{}".to_string(),
        Some(s) => s,
    };

    export_enfusion_defaults();
    let sock = resolve_sock();
    // SAFETY: bash exports MCP_SOCK for daemon children / helpers.
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

fn scripts_mod() -> PathBuf {
    crate::root::find_repo_root()
        .map(|r| r.join("scripts/mod"))
        .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../scripts/mod"))
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

/// 4-tier runner resolution (one-shot path). Returns argv (program + args).
fn resolve_runner(script_dir: &Path) -> Vec<String> {
    if let Ok(bin) = env::var("ENFUSION_MCP_BIN") {
        let p = PathBuf::from(&bin);
        if p.is_file() {
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if name.ends_with(".js") || name.ends_with(".mjs") {
                dbg("runner=tier1(ENFUSION_MCP_BIN)");
                return vec!["node".into(), bin];
            }
            dbg("runner=tier1(ENFUSION_MCP_BIN)");
            return vec![bin];
        }
    }
    let pinned = script_dir.join("node_modules/enfusion-mcp/dist/index.js");
    if pinned.is_file() {
        dbg("runner=tier2(pinned)");
        return vec!["node".into(), pinned.to_string_lossy().into_owned()];
    }
    // PINNED fail-open: bash `find … 2>/dev/null | head -1`.
    if let Some(hit) = find_npx_enfusion() {
        dbg("runner=tier3(npx-cache)");
        return vec!["node".into(), hit];
    }
    dbg("runner=tier4(npx)");
    vec!["npx".into(), "-y".into(), "enfusion-mcp".into()]
}

fn find_npx_enfusion() -> Option<String> {
    let home = env::var("HOME").ok()?;
    let root = PathBuf::from(home).join(".npm/_npx");
    if !root.is_dir() {
        return None;
    }
    let mut hits = Vec::new();
    let _ = visit_depth(&root, 0, 4, &mut hits);
    // Sorted for determinism (bash `find|head -1` was readdir-order — pin the swap).
    hits.sort();
    hits.into_iter().next()
}

fn visit_depth(dir: &Path, depth: u32, max: u32, out: &mut Vec<String>) -> std::io::Result<()> {
    if depth > max {
        return Ok(());
    }
    let rd = match fs::read_dir(dir) {
        Ok(rd) => rd,
        Err(_) => return Ok(()), // PINNED fail-open (find 2>/dev/null)
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

fn ensure_daemon(script_dir: &Path, sock: &str) -> bool {
    if env::var("MCP_NO_DAEMON").ok().as_deref() == Some("1") {
        return false;
    }
    let daemon = script_dir.join("mcp-daemon.sh");
    if daemon_status(&daemon) {
        return true;
    }
    // PINNED fail-open: bash opens SOCK.lock + flock -w 65 || true, then status||start.
    let lock_path = format!("{sock}.lock");
    // PINNED fail-open: ignore flock errors (bash `flock … || true`).
    let _held = flock_exclusive(
        Path::new(&lock_path),
        Duration::from_secs(1),
        Duration::from_secs(65),
        |_| {},
    )
    .ok();
    if !daemon_status(&daemon) {
        let _ = Run::new("bash").arg(&daemon).arg("start").merged_output(); // bash >/dev/null 2>&1
    }
    drop(_held);
    daemon_status(&daemon)
}

fn daemon_status(daemon: &Path) -> bool {
    matches!(
        Run::new("bash").arg(daemon).arg("status").merged_output(),
        Ok(m) if m.code == 0
    )
}

/// 0 success · 3 tool error · 9 fall-back-to-oneshot
fn daemon_try(tool: &str, args: &str, sock: &str) -> i32 {
    let script_dir = scripts_mod();
    if !ensure_daemon(&script_dir, sock) {
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
    let script_dir = scripts_mod();
    let runner = resolve_runner(&script_dir);
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

/// `emit | timeout RUNNER 2>errf | xtask mcp consume >outf` — returns (to_rc, consume_rc).
fn oneshot_pipe(
    runner: &[String],
    tool: &str,
    args: &str,
    timeout: u64,
    outf: &Path,
    errf: &Path,
) -> (i32, i32) {
    // CLOSED fail-open: without timeout(1) bash could hang the server; refuse the attempt.
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
mod tests {
    use super::*;

    #[test]
    fn usage_when_tool_missing() {
        assert_eq!(run(None, None), 1);
        assert_eq!(run(Some(String::new()), None), 1);
    }

    #[test]
    fn emit_requests_embeds_tool_and_args() {
        let s = emit_requests("wb_state", r#"{"x":1}"#);
        assert!(s.contains(r#""name":"wb_state""#));
        assert!(s.contains(r#""arguments":{"x":1}"#));
        assert!(s.contains(r#""id":2"#));
    }
}
