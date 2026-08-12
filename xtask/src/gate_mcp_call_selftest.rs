//! T-865 — port of `scripts/mod/mcp-call-selftest.sh` → `cargo xtask mcp selftest`.
//!
//! Offline MCP call-path gates (T-090.0 T1–T7; T-162 consumer). Drives `xtask mcp consume`
//! against recorded fixtures and `xtask mcp call` against the Rust mcpd stub (`MCP_STUB=1`).
//!
//! Wave 226 option 2: `cargo run -q -p xtask --` replaces former `lib/xtask-run.sh`;
//! mcpd path is inlined (`cargo build -q -p tbd-tools --bin mcpd` + `CARGO_TARGET_DIR`
//! honor — former `lib/mcpd-bin.sh`). Daemon control is in-process
//! (`mcp_daemon`, T-888). Warm
//! `CARGO_TARGET_DIR` keeps stdout reproducible.
//!
//! Fail-opens pinned (bash parity):
//! - `cleanup` / daemon `start` discard stdout+stderr (`>/dev/null 2>&1`).
//! - One-shot arms discard stderr via `2>/dev/null` (except empty+retry / T7 which keep it).
//! - Script uses `set -uo pipefail` **without** `-e` — a failed arm increments FAIL and continues.
//!
//! Summary label stays `mcp-call-selftest:` (byte parity with the deleted script).

use std::fs;
use std::io::{self, Write};
use std::path::Path;

use tbd_gate::NotRun;
use tbd_gate::proc::Run;

use crate::root::find_repo_root;

/// Shared stderr dump path — same as bash `/tmp/.st_e`.
const ST_ERR: &str = "/tmp/.st_e";

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

/// Strip trailing newlines the way bash `$(…)` does.
fn bash_chomp(s: &str) -> String {
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
    run_at(&root.join("scripts/mod"))
}

/// Testable entry: `script_dir` is the former `SCRIPT_DIR` (…/scripts/mod).
pub fn run_at(script_dir: &Path) -> i32 {
    let root = script_dir
        .parent()
        .and_then(|p| p.parent())
        .map(Path::to_path_buf)
        .unwrap_or_else(|| script_dir.to_path_buf());
    let fix = script_dir.join("fixtures");
    let sock = format!("/tmp/tbd-mcp-selftest-{}.sock", uid());

    // bash: `export MCP_STUB=1`
    unsafe { std::env::set_var("MCP_STUB", "1") };

    let mut c = Counters { pass: 0, fail: 0 };

    println!("[T-543] mcpd CARGO_TARGET_DIR honor");
    let want_stub = format!(
        "{}/debug/mcpd",
        std::env::var("CARGO_TARGET_DIR")
            .unwrap_or_else(|_| root.join("target").display().to_string())
    );
    let stub = match resolve_mcpd_bin(&root) {
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

    println!("[T2-T5] consumer fixtures");
    match xtask_consume(&root, &fix.join("mcp-wb-state-success.jsonl")) {
        Ok((rc, out, _)) => {
            let out = bash_chomp(&out);
            if rc == 0 && !out.is_empty() {
                c.ok("T2 success rc0 non-empty");
            } else {
                c.no(&format!("T2 success (rc={rc} out=[{out}])"));
            }
        }
        Err(n) => c.no(&format!("T2 DidNotRun: {n:?}")),
    }

    match xtask_consume(&root, &fix.join("mcp-tool-error.jsonl")) {
        Ok((rc, _out, err)) => {
            let _ = fs::write(ST_ERR, &err);
            c.rc_is("T3 error (rpc)", 3, rc);
            if err.contains(r#""code""#) {
                c.ok("T3 error JSON on stderr");
            } else {
                c.no("T3 error JSON missing");
            }
        }
        Err(n) => c.no(&format!("T3 DidNotRun: {n:?}")),
    }

    match xtask_consume(&root, &fix.join("mcp-tool-iserror.jsonl")) {
        Ok((rc, _out, err)) => {
            let _ = fs::write(ST_ERR, &err);
            c.rc_is("T3b error (isError)", 3, rc);
            if err.contains("MCP error") {
                c.ok("T3b isError text on stderr");
            } else {
                c.no("T3b isError text missing");
            }
        }
        Err(n) => c.no(&format!("T3b DidNotRun: {n:?}")),
    }

    match xtask_consume(&root, &fix.join("mcp-init-fail.jsonl")) {
        Ok((rc, _, _)) => c.rc_is("T4 init-fail", 2, rc),
        Err(n) => c.no(&format!("T4 DidNotRun: {n:?}")),
    }

    match xtask_consume(&root, &fix.join("mcp-empty.jsonl")) {
        Ok((rc, out, _)) => {
            let out = bash_chomp(&out);
            if rc == 1 && out.is_empty() {
                c.ok("T5 empty rc1 empty-out");
            } else {
                c.no(&format!("T5 empty (rc={rc} out=[{out}])"));
            }
        }
        Err(n) => c.no(&format!("T5 DidNotRun: {n:?}")),
    }

    println!("[T6] usage error, no spawn");
    match xtask_call(&root, &[], &[]) {
        Ok((rc, _out, err)) => {
            let _ = fs::write(ST_ERR, &err);
            c.rc_is("T6 usage", 1, rc);
            // bash: `grep -q usage` (case-sensitive)
            if err.contains("usage") {
                c.ok("T6 usage text on stderr");
            } else {
                c.no("T6 usage text missing");
            }
        }
        Err(n) => c.no(&format!("T6 DidNotRun: {n:?}")),
    }

    println!("[one-shot wrapper via stub] (MCP_NO_DAEMON=1)");
    let call_args = ["wb_state".to_string(), "{}".to_string()];

    match xtask_call(
        &root,
        &call_args,
        &env_pairs(&[
            ("MCP_NO_DAEMON", "1"),
            ("ENFUSION_MCP_BIN", &stub),
            ("STUB_MODE", "success"),
            ("STUB_LINGER", "0.3"),
        ]),
    ) {
        Ok((rc, out, _err)) => {
            let out = bash_chomp(&out);
            if rc == 0 && out == "STUB-OK wb_state edit 123" {
                c.ok("one-shot success rc0");
            } else {
                c.no(&format!("one-shot success (rc={rc} out=[{out}])"));
            }
        }
        Err(n) => c.no(&format!("one-shot success DidNotRun: {n:?}")),
    }

    match xtask_call(
        &root,
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
        &root,
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
        &root,
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
            let _ = fs::write(ST_ERR, &err);
            c.rc_is("one-shot empty+retry", 1, rc);
            if err.contains("STUB-STDERR-MARKER") {
                c.ok("T7 stderr surfaced on failure");
            } else {
                c.no("T7 stderr not surfaced");
            }
        }
        Err(n) => c.no(&format!("one-shot empty DidNotRun: {n:?}")),
    }

    match xtask_call(
        &root,
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
    // bash: `export MCP_SOCK=… MCP_DAEMON_IDLE=8 MCP_DAEMON_MAX_LIFE=30`
    unsafe {
        std::env::set_var("MCP_SOCK", &sock);
        std::env::set_var("MCP_DAEMON_IDLE", "8");
        std::env::set_var("MCP_DAEMON_MAX_LIFE", "30");
    }

    // PINNED fail-open: start discards streams (T-888 in-process quiet start)
    // Env for stub/idle already set above via set_var; also pin ENFUSION_MCP_BIN.
    unsafe {
        std::env::set_var("ENFUSION_MCP_BIN", &stub);
        std::env::set_var("STUB_DAEMON", "1");
    }
    let _ = crate::mcp_daemon::start_at(&sock, true);
    let code = crate::mcp_daemon::status_at(&sock, true);
    c.rc_is("daemon start+status", 0, code);

    match xtask_call(
        &root,
        &call_args,
        &env_pairs(&[
            ("ENFUSION_MCP_BIN", &stub),
            ("STUB_DAEMON", "1"),
            ("MCP_SOCK", &sock),
        ]),
    ) {
        Ok((rc, out, _)) => {
            let out = bash_chomp(&out);
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
        &root,
        &args_rt,
        &env_pairs(&[
            ("ENFUSION_MCP_BIN", &stub),
            ("STUB_DAEMON", "1"),
            ("MCP_SOCK", &sock),
        ]),
    ) {
        Ok((_rc, out, _)) => {
            let out = bash_chomp(&out);
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
        &root,
        &call_args,
        &env_pairs(&[
            ("MCP_NO_DAEMON", "1"),
            ("ENFUSION_MCP_BIN", &stub),
            ("STUB_MODE", "success"),
            ("STUB_LINGER", "0.3"),
        ]),
    ) {
        Ok((rc, out, _)) => {
            let out = bash_chomp(&out);
            if rc == 0 && !out.is_empty() {
                c.ok("fallback when no daemon");
            } else {
                c.no(&format!("fallback (rc={rc} out=[{out}])"));
            }
        }
        Err(n) => c.no(&format!("fallback DidNotRun: {n:?}")),
    }

    let _ = fs::remove_file(ST_ERR);

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

/// PINNED fail-open: bash `cleanup() { … >/dev/null 2>&1; rm -f "$SOCK"*; }`
fn cleanup(sock: &str) {
    let _ = crate::mcp_daemon::stop_at(sock, true);
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

/// Former `lib/mcpd-bin.sh`: build mcpd quietly, honor `CARGO_TARGET_DIR`, echo path.
fn resolve_mcpd_bin(root: &Path) -> Result<(i32, String), NotRun> {
    let target_dir = std::env::var("CARGO_TARGET_DIR")
        .unwrap_or_else(|_| root.join("target").display().to_string());
    let o = Run::new("cargo")
        .arg("build")
        .arg("-q")
        .arg("-p")
        .arg("tbd-tools")
        .arg("--bin")
        .arg("mcpd")
        .cwd(root)
        .output()?;
    if o.code != 0 {
        return Ok((o.code, String::new()));
    }
    Ok((0, format!("{target_dir}/debug/mcpd")))
}

/// Former `lib/xtask-run.sh` ≡ `cargo run -q -p xtask -- <args>` from monorepo root.
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
mod tests {
    use super::bash_chomp;

    #[test]
    fn bash_chomp_strips_trailing_newlines() {
        assert_eq!(bash_chomp("a\n\n"), "a");
        assert_eq!(bash_chomp(""), "");
    }
}
