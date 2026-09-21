//! `cargo xtask mcp selftest` — the whole MCP call path, offline.
//!
//! Two halves. `cargo xtask mcp consume` is driven against the recorded transcripts in
//! [`repository_layout::MCP_TRANSCRIPT_FIXTURES_DIR`], one per response shape, to pin the exit
//! code each shape produces. `cargo xtask mcp call` is then driven against the `mcpd` stub
//! (`MCP_STUB=1`), one-shot and through the daemon, to pin the same codes end to end — success,
//! tool error, failed initialize, empty after retries, and timeout.
//!
//! Every arm runs even after one fails, so a run reports the whole call path rather than the
//! first broken arm. A warm `CARGO_TARGET_DIR` keeps the build step's output off stdout.

use std::fs;
use std::io::{self, Write};
use std::path::Path;

use verification_core::NotRun;
use verification_core::proc::Run;

use crate::core::repository_layout;
use crate::core::repository_root::find_repo_root;

/// Where an arm's captured stderr is parked so a failing run can be read after the fact.
const CAPTURED_STDERR_PATH: &str = "/tmp/xtask-mcp-selftest-stderr";

struct Counters {
    pass: u32,
    fail: u32,
}

impl Counters {
    fn ok(&mut self, msg: &str) {
        println!("  ✓ {msg}");
        self.pass += 1;
    }

    fn no(&mut self, msg: &str) {
        let _ = writeln!(io::stderr(), "  ✗ {msg}");
        self.fail += 1;
    }

    fn rc_is(&mut self, label: &str, want: i32, got: i32) {
        if got == want {
            self.ok(&format!("{label} (rc={want})"));
        } else {
            self.no(&format!("{label} (want rc{want} got rc{got})"));
        }
    }
}

/// Strip every trailing newline, so an otherwise empty body reads as empty.
fn strip_trailing_newlines(s: &str) -> String {
    let mut t = s.to_string();
    while t.ends_with('\n') {
        t.pop();
    }
    t
}

fn uid() -> u32 {
    unsafe { libc::getuid() }
}

fn env_pairs(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
        .collect()
}

/// Entry for `xtask mcp selftest`.
pub fn run() -> i32 {
    let root = match find_repo_root() {
        Ok(r) => r,
        Err(e) => {
            let _ = writeln!(io::stderr(), "mcp-call-selftest: FAIL (no repo root: {e})");
            return 1;
        }
    };
    run_at(&root)
}

/// Testable entry: `root` is the checkout the transcripts and the commands are read from.
pub fn run_at(root: &Path) -> i32 {
    let fix = root.join(repository_layout::MCP_TRANSCRIPT_FIXTURES_DIR);
    let sock = format!("/tmp/tbd-mcp-selftest-{}.sock", uid());

    // SAFETY: set before any thread is spawned. Every `mcp call` this selftest spawns must reach
    // the stub rather than a real Workbench.
    unsafe { std::env::set_var("MCP_STUB", "1") };

    let mut c = Counters { pass: 0, fail: 0 };

    println!("[mcpd build] CARGO_TARGET_DIR is honoured");
    let want_stub = format!(
        "{}/debug/mcpd",
        std::env::var("CARGO_TARGET_DIR")
            .unwrap_or_else(|_| root.join("target").display().to_string())
    );
    let stub = match resolve_mcpd_bin(root) {
        Ok((0, path)) => {
            if path == want_stub {
                c.ok(&format!("mcpd build+path ({path})"));
            } else {
                c.no(&format!("mcpd build+path (want={want_stub} got={path})"));
            }
            path
        }
        Ok((code, path)) => {
            c.no(&format!("mcpd build (want rc0 got rc{code} path=[{path}])"));
            String::new()
        }
        Err(n) => {
            c.no(&format!("mcpd build DidNotRun: {n:?}"));
            String::new()
        }
    };

    cleanup(&sock);

    println!("[transcripts] one recorded response shape per exit code");
    match xtask_consume(root, &fix.join("mcp-wb-state-success.jsonl")) {
        Ok((rc, out, _)) => {
            let out = strip_trailing_newlines(&out);
            if rc == 0 && !out.is_empty() {
                c.ok("success rc0 non-empty");
            } else {
                c.no(&format!("success (rc={rc} out=[{out}])"));
            }
        }
        Err(n) => c.no(&format!("success DidNotRun: {n:?}")),
    }

    match xtask_consume(root, &fix.join("mcp-tool-error.jsonl")) {
        Ok((rc, _out, err)) => {
            let _ = fs::write(CAPTURED_STDERR_PATH, &err);
            c.rc_is("JSON-RPC error", 3, rc);
            if err.contains(r#""code""#) {
                c.ok("error JSON on stderr");
            } else {
                c.no("error JSON missing");
            }
        }
        Err(n) => c.no(&format!("JSON-RPC error DidNotRun: {n:?}")),
    }

    match xtask_consume(root, &fix.join("mcp-tool-iserror.jsonl")) {
        Ok((rc, _out, err)) => {
            let _ = fs::write(CAPTURED_STDERR_PATH, &err);
            c.rc_is("tool-reported error", 3, rc);
            if err.contains("MCP error") {
                c.ok("tool-reported error text on stderr");
            } else {
                c.no("tool-reported error text missing");
            }
        }
        Err(n) => c.no(&format!("tool-reported error DidNotRun: {n:?}")),
    }

    match xtask_consume(root, &fix.join("mcp-init-fail.jsonl")) {
        Ok((rc, _, _)) => c.rc_is("initialize failed", 2, rc),
        Err(n) => c.no(&format!("initialize failed DidNotRun: {n:?}")),
    }

    match xtask_consume(root, &fix.join("mcp-empty.jsonl")) {
        Ok((rc, out, _)) => {
            let out = strip_trailing_newlines(&out);
            if rc == 1 && out.is_empty() {
                c.ok("empty response rc1 empty-out");
            } else {
                c.no(&format!("empty response (rc={rc} out=[{out}])"));
            }
        }
        Err(n) => c.no(&format!("empty response DidNotRun: {n:?}")),
    }

    println!("[usage] a missing tool name is refused without spawning a server");
    match xtask_call(root, &[], &[]) {
        Ok((rc, _out, err)) => {
            let _ = fs::write(CAPTURED_STDERR_PATH, &err);
            c.rc_is("usage", 1, rc);
            if err.contains("usage") {
                c.ok("usage text on stderr");
            } else {
                c.no("usage text missing");
            }
        }
        Err(n) => c.no(&format!("usage DidNotRun: {n:?}")),
    }

    println!("[one-shot wrapper via stub] (MCP_NO_DAEMON=1)");
    let call_args = ["wb_state".to_string(), "{}".to_string()];

    match xtask_call(
        root,
        &call_args,
        &env_pairs(&[
            ("MCP_NO_DAEMON", "1"),
            ("ENFUSION_MCP_BIN", &stub),
            ("STUB_MODE", "success"),
            ("STUB_LINGER", "0.3"),
        ]),
    ) {
        Ok((rc, out, _err)) => {
            let out = strip_trailing_newlines(&out);
            if rc == 0 && out == "STUB-OK wb_state edit 123" {
                c.ok("one-shot success rc0");
            } else {
                c.no(&format!("one-shot success (rc={rc} out=[{out}])"));
            }
        }
        Err(n) => c.no(&format!("one-shot success DidNotRun: {n:?}")),
    }

    match xtask_call(
        root,
        &call_args,
        &env_pairs(&[
            ("MCP_NO_DAEMON", "1"),
            ("ENFUSION_MCP_BIN", &stub),
            ("STUB_MODE", "error"),
            ("STUB_LINGER", "0.3"),
        ]),
    ) {
        Ok((rc, _, _)) => c.rc_is("one-shot error", 3, rc),
        Err(n) => c.no(&format!("one-shot error DidNotRun: {n:?}")),
    }

    match xtask_call(
        root,
        &call_args,
        &env_pairs(&[
            ("MCP_NO_DAEMON", "1"),
            ("ENFUSION_MCP_BIN", &stub),
            ("STUB_MODE", "initfail"),
            ("STUB_LINGER", "0.3"),
            ("MCP_CALL_RETRIES", "0"),
        ]),
    ) {
        Ok((rc, _, _)) => c.rc_is("one-shot init-fail", 2, rc),
        Err(n) => c.no(&format!("one-shot init-fail DidNotRun: {n:?}")),
    }

    match xtask_call(
        root,
        &call_args,
        &env_pairs(&[
            ("MCP_NO_DAEMON", "1"),
            ("ENFUSION_MCP_BIN", &stub),
            ("STUB_MODE", "empty"),
            ("STUB_LINGER", "0.3"),
            ("MCP_CALL_RETRIES", "1"),
        ]),
    ) {
        Ok((rc, _out, err)) => {
            let _ = fs::write(CAPTURED_STDERR_PATH, &err);
            c.rc_is("one-shot empty+retry", 1, rc);
            if err.contains("STUB-STDERR-MARKER") {
                c.ok("stderr surfaced on failure");
            } else {
                c.no("stderr not surfaced");
            }
        }
        Err(n) => c.no(&format!("one-shot empty DidNotRun: {n:?}")),
    }

    match xtask_call(
        root,
        &call_args,
        &env_pairs(&[
            ("MCP_NO_DAEMON", "1"),
            ("ENFUSION_MCP_BIN", &stub),
            ("STUB_MODE", "empty"),
            ("STUB_LINGER", "4"),
            ("MCP_CALL_TIMEOUT", "1"),
            ("MCP_CALL_RETRIES", "0"),
        ]),
    ) {
        Ok((rc, _, _)) => c.rc_is("one-shot timeout", 4, rc),
        Err(n) => c.no(&format!("one-shot timeout DidNotRun: {n:?}")),
    }

    println!("[daemon via stub-daemon] (short socket, offline)");
    // SAFETY: single-threaded; the daemon and the calls below inherit these.
    unsafe {
        std::env::set_var("MCP_SOCK", &sock);
        std::env::set_var("MCP_DAEMON_IDLE", "8");
        std::env::set_var("MCP_DAEMON_MAX_LIFE", "30");
    }

    // The daemon must resolve the stub rather than a real server, so its entry is pinned here.
    // SAFETY: single-threaded; the daemon inherits these.
    unsafe {
        std::env::set_var("ENFUSION_MCP_BIN", &stub);
        std::env::set_var("STUB_DAEMON", "1");
    }
    let _ = crate::commands::mcp::daemon::start_at(&sock, true);
    let code = crate::commands::mcp::daemon::status_at(&sock, true);
    c.rc_is("daemon start+status", 0, code);

    match xtask_call(
        root,
        &call_args,
        &env_pairs(&[
            ("ENFUSION_MCP_BIN", &stub),
            ("STUB_DAEMON", "1"),
            ("MCP_SOCK", &sock),
        ]),
    ) {
        Ok((rc, out, _)) => {
            let out = strip_trailing_newlines(&out);
            if rc == 0 && out == "STUB-DAEMON-OK wb_state args={}" {
                c.ok("daemon call rc0");
            } else {
                c.no(&format!("daemon call (rc={rc} out=[{out}])"));
            }
        }
        Err(n) => c.no(&format!("daemon call DidNotRun: {n:?}")),
    }

    let args_rt = ["api_search".to_string(), r#"{"query":"Ztest"}"#.to_string()];
    match xtask_call(
        root,
        &args_rt,
        &env_pairs(&[
            ("ENFUSION_MCP_BIN", &stub),
            ("STUB_DAEMON", "1"),
            ("MCP_SOCK", &sock),
        ]),
    ) {
        Ok((_rc, out, _)) => {
            let out = strip_trailing_newlines(&out);
            if out == r#"STUB-DAEMON-OK api_search args={"query":"Ztest"}"# {
                c.ok("args round-trip (no brace corruption)");
            } else {
                c.no(&format!("args round-trip (out=[{out}])"));
            }
        }
        Err(n) => c.no(&format!("args round-trip DidNotRun: {n:?}")),
    }

    cleanup(&sock);

    match xtask_call(
        root,
        &call_args,
        &env_pairs(&[
            ("MCP_NO_DAEMON", "1"),
            ("ENFUSION_MCP_BIN", &stub),
            ("STUB_MODE", "success"),
            ("STUB_LINGER", "0.3"),
        ]),
    ) {
        Ok((rc, out, _)) => {
            let out = strip_trailing_newlines(&out);
            if rc == 0 && !out.is_empty() {
                c.ok("fallback when no daemon");
            } else {
                c.no(&format!("fallback (rc={rc} out=[{out}])"));
            }
        }
        Err(n) => c.no(&format!("fallback DidNotRun: {n:?}")),
    }

    let _ = fs::remove_file(CAPTURED_STDERR_PATH);

    println!("---");
    if c.fail == 0 {
        println!("mcp-call-selftest: ALL PASS ({})", c.pass);
        0
    } else {
        let _ = writeln!(
            io::stderr(),
            "mcp-call-selftest: FAIL ({} failed, {} passed)",
            c.fail,
            c.pass
        );
        1
    }
}

/// Stop any daemon on `sock` and remove the socket plus everything it named beside it.
///
/// Every step is best-effort: this runs before the first arm, when nothing exists yet.
fn cleanup(sock: &str) {
    let _ = crate::commands::mcp::daemon::stop_at(sock, true);
    if let Ok(rd) = fs::read_dir("/tmp") {
        let prefix = Path::new(sock)
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        for ent in rd.flatten() {
            let name = ent.file_name();
            let n = name.to_string_lossy();
            if n.starts_with(&prefix) {
                let _ = fs::remove_file(ent.path());
            }
        }
    }
    let _ = fs::remove_file(sock);
}

/// Build `mcpd` quietly into `CARGO_TARGET_DIR` and return where the binary landed.
fn resolve_mcpd_bin(root: &Path) -> Result<(i32, String), NotRun> {
    let target_dir = std::env::var("CARGO_TARGET_DIR")
        .unwrap_or_else(|_| root.join("target").display().to_string());
    let o = Run::new("cargo")
        .arg("build")
        .arg("-q")
        .arg("-p")
        .arg("developer-tools")
        .arg("--bin")
        .arg("mcpd")
        .cwd(root)
        .output()?;
    if o.code != 0 {
        return Ok((o.code, String::new()));
    }
    Ok((0, format!("{target_dir}/debug/mcpd")))
}

/// `cargo run -q -p xtask -- <args>` from the checkout root.
fn cargo_xtask(
    root: &Path,
    args: &[&str],
    stdin: Option<&str>,
    envs: &[(String, String)],
) -> Result<(i32, String, String), NotRun> {
    let mut r = Run::new("cargo")
        .arg("run")
        .arg("-q")
        .arg("-p")
        .arg("xtask")
        .arg("--");
    for a in args {
        r = r.arg(*a);
    }
    r = r.cwd(root);
    for (k, v) in envs {
        r = r.env(k, v);
    }
    if let Some(body) = stdin {
        r = r.stdin(body);
    }
    let o = r.output()?;
    Ok((o.code, o.stdout, o.stderr))
}

fn xtask_consume(root: &Path, fixture: &Path) -> Result<(i32, String, String), NotRun> {
    let body = fs::read_to_string(fixture).unwrap_or_default();
    cargo_xtask(root, &["mcp", "consume"], Some(&body), &[])
}

fn xtask_call(
    root: &Path,
    args: &[String],
    envs: &[(String, String)],
) -> Result<(i32, String, String), NotRun> {
    let mut argv: Vec<&str> = vec!["mcp", "call"];
    let owned: Vec<&str> = args.iter().map(String::as_str).collect();
    argv.extend(owned.iter().copied());
    cargo_xtask(root, &argv, None, envs)
}

#[cfg(test)]
#[path = "tests/call_selftest/tests.rs"]
mod tests;
