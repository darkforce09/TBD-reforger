//! `cargo xtask mcp smoke` — one live call per tool against a connected Workbench.
//!
//! `wb_connect` and `wb_state` must each return a non-empty body through
//! `cargo run -q -p xtask -- mcp call`. Every tool is attempted even after one fails, so a run
//! reports which tools are reachable rather than only the first that is not; the child's stderr
//! is passed through so the reason is visible beside the verdict.
//!
//! Exit: 0 every tool OK · 1 any tool FAIL.

use std::io::{self, Write};
use std::path::Path;

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
    run_at(&root)
}

/// Testable entry: `root` is the checkout the calls are made from.
pub fn run_at(root: &Path) -> i32 {
    run_writers(root, &mut io::stdout(), &mut io::stderr())
}

fn run_writers(root: &Path, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let mut fail = 0i32;

    for tool in TOOLS {
        match call_tool(root, tool) {
            Ok((rc, body, child_err)) => {
                // The child's own diagnostics belong beside the verdict they explain.
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
                // A command that never ran is not a tool that answered: it surfaces as a FAIL
                // with the exit code its failure mode implies, never folded into a bare bool.
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

/// `(exit code, stdout without trailing newlines, raw stderr)`.
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
    Ok((o.code, strip_trailing_newlines(&o.stdout), o.stderr))
}

/// Strip every trailing newline, so an otherwise empty body reads as empty.
fn strip_trailing_newlines(s: &str) -> String {
    let mut t = s.to_string();
    while t.ends_with('\n') {
        t.pop();
    }
    t
}

#[cfg(test)]
#[path = "tests/smoke/tests.rs"]
mod tests;
