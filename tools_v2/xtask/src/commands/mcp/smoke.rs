//! T-877 — port of `scripts/mod/mcp-smoke.sh` → `cargo xtask mcp smoke`.
//!
//! Live MCP smoke (T-090.0 gate S1): `wb_connect` + `wb_state` must both return
//! non-empty via in-process `cargo run -q -p xtask -- mcp call` (former
//! `lib/xtask-run.sh` parity — wave 226 option 2; libs stay on disk for OOS bash).
//!
//! Preserved bash shape (`set -uo pipefail`, **no** `-e`):
//! - a failed / empty tool call does not abort the loop
//! - command substitution captures **stdout only**; child stderr leaks to our stderr
//! - `$()` strips all trailing newlines before `[ -n "$out" ]`
//!
//! Exit: 0 all tools OK · 1 any tool FAIL.

use std::io::{self, Write};
use std::path::{Path, PathBuf};

use verification_core::NotRun;
use verification_core::proc::Run;

use crate::core::repository_root::find_repo_root;

const TOOLS: &[&str] = &["wb_connect", "wb_state"];

/// Entry for `xtask mcp smoke`.
pub fn run() -> i32 {
    let root = match find_repo_root() {
        Ok(r) => r,
        Err(e) => {
            let _ = writeln!(io::stderr(), "mcp-smoke: FAIL (no repo root: {e})");
            return 1;
        }
    };
    run_at(&root.join("scripts/mod"))
}

/// Testable entry: `script_dir` is the former `SCRIPT_DIR` (`…/scripts/mod`).
pub fn run_at(script_dir: &Path) -> i32 {
    run_writers(script_dir, &mut io::stdout(), &mut io::stderr())
}

fn mono_root_from_script_dir(script_dir: &Path) -> PathBuf {
    // scripts/mod → ../../ = monorepo root (former xtask-run.sh dirname climb).
    script_dir
        .parent()
        .and_then(|p| p.parent())
        .map(Path::to_path_buf)
        .unwrap_or_else(|| script_dir.to_path_buf())
}

fn run_writers(script_dir: &Path, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let root = mono_root_from_script_dir(script_dir);
    let mut fail = 0i32;

    for tool in TOOLS {
        match call_tool(&root, tool) {
            Ok((rc, body, child_err)) => {
                // Bash `$()` keeps child stderr on the smoke's stderr.
                if !child_err.is_empty() {
                    let _ = err.write_all(child_err.as_bytes());
                    let _ = err.flush();
                }
                if rc == 0 && !body.is_empty() {
                    let _ = writeln!(out, "mcp-smoke: {tool} OK");
                } else {
                    let _ = writeln!(err, "mcp-smoke: {tool} FAIL (rc={rc})");
                    fail = 1;
                }
            }
            Err(n) => {
                // No bool fold: DidNotRun must not look like Held. Surface as a tool FAIL
                // with a non-zero rc so the smoke stays red (bash would also fail the arm).
                let rc = match &n {
                    NotRun::ToolAbsent(_) => 127,
                    NotRun::Signalled { signal, .. } => 128 + signal,
                    NotRun::Timeout { .. } => 124,
                    NotRun::ToolError { status, .. } if *status > 0 => *status,
                    _ => 1,
                };
                let _ = writeln!(err, "mcp-smoke: DidNotRun ({n:?})");
                let _ = writeln!(err, "mcp-smoke: {tool} FAIL (rc={rc})");
                fail = 1;
            }
        }
    }

    if fail == 0 {
        let _ = writeln!(out, "mcp-smoke: OK");
        0
    } else {
        let _ = writeln!(err, "mcp-smoke: FAIL");
        1
    }
}

/// `(rc, bash-chomped stdout, raw stderr)`.
/// Former `lib/xtask-run.sh mcp call TOOL '{}'` ≡ `cargo run -q -p xtask -- mcp call …`.
fn call_tool(root: &Path, tool: &str) -> Result<(i32, String, String), NotRun> {
    let o = Run::new("cargo")
        .arg("run")
        .arg("-q")
        .arg("-p")
        .arg("xtask")
        .arg("--")
        .arg("mcp")
        .arg("call")
        .arg(tool)
        .arg("{}")
        .cwd(root)
        .output()?;
    Ok((o.code, bash_chomp(&o.stdout), o.stderr))
}

/// Bash command-substitution strips every trailing newline.
fn bash_chomp(s: &str) -> String {
    let mut t = s.to_string();
    while t.ends_with('\n') {
        t.pop();
    }
    t
}

#[cfg(test)]
#[path = "tests/smoke/tests.rs"]
mod tests;
