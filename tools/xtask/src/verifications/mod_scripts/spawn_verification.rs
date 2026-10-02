//! `cargo xtask mod spawn-verify`: spawn the mission in the Workbench and read the result.
//!
//! Thin wrapper around `mcp wb-logs` + MCP play/stop:
//! - `--selftest` → `exec cargo run -q -p xtask -- mcp wb-logs --selftest` (no Workbench)
//! - else → `mcp call wb_play` / sleep 25 / `mcp call wb_stop` / `mcp wb-logs PATTERN`
//!
//! Fail-opens pinned (bash parity — do not "fix"):
//! - `mcp call wb_play '{}'`, failures ignored
//! - `mcp call wb_stop '{}'`, failures ignored
//!
//! Play/stop MCP failures must not abort the verify; the log grep verdict is what matters.
//!
//! Default display filter pins tags + event keys, never deleted prose. Verdict logic
//! lives in `mcp wb-logs` — one definition, shared.
//!
//! §Non-reproducible: the live (non-`--selftest`) arm sleeps 25s wall-clock between play and
//! stop. Acceptance for that arm uses PATH stubs for `cargo` so MCP is throwaway; only the
//! final `mcp wb-logs` lines are compared. Prefer `--selftest` for clean byte parity.

use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::Command;
use std::thread;
use std::time::Duration;

use anyhow::Result;

use crate::core::repository_root::find_repo_root;

/// The default world when the operator names none.
const DEFAULT_PATTERN: &str =
    r"\[TBD\]\[Slots\]|\[TBD\]\[Loadout\]|\[TBD\]\[Spawn\]|assigned slot|bound player";

/// Entry for `xtask mod spawn-verify`.
pub fn run(selftest: bool, pattern: Option<String>) -> Result<u8> {
    let root = find_repo_root()?;
    run_with_root(&root, selftest, pattern)
}

pub(crate) fn run_with_root(root: &Path, selftest: bool, pattern: Option<String>) -> Result<u8> {
    if selftest {
        // bash: `exec cargo run -q -p xtask -- mcp wb-logs --selftest`
        let err = Command::new("cargo")
            .args([
                "run",
                "-q",
                "-p",
                "xtask",
                "--",
                "mcp",
                "wb-logs",
                "--selftest",
            ])
            .current_dir(root)
            .exec();
        eprintln!("tbd-spawn-verify: failed to exec cargo: {err}");
        return Ok(127);
    }

    let pattern = pattern.unwrap_or_else(|| DEFAULT_PATTERN.to_string());

    // `mcp call wb_play '{}'`, failures ignored.
    // FAIL-OPEN PIN: play MCP failure must not abort (preserved).
    let _ = cargo_xtask(root, &["mcp", "call", "wb_play", "{}"]);

    thread::sleep(Duration::from_secs(25));

    // `mcp call wb_stop '{}'`, failures ignored.
    // FAIL-OPEN PIN: stop MCP failure must not abort (preserved).
    let _ = cargo_xtask(root, &["mcp", "call", "wb_stop", "{}"]);

    // bash: `cargo run -q -p xtask -- mcp wb-logs "$PATTERN"` (exit code propagates)
    match cargo_xtask(root, &["mcp", "wb-logs", &pattern]) {
        Ok(code) => Ok(code),
        Err(_) => Ok(127),
    }
}

/// `cargo run -q -p xtask -- <args>` from the monorepo root.
fn cargo_xtask(root: &Path, args: &[&str]) -> std::io::Result<u8> {
    let mut cmd = Command::new("cargo");
    cmd.args(["run", "-q", "-p", "xtask", "--"])
        .args(args)
        .current_dir(root);
    let status = cmd.status()?;
    Ok(status.code().unwrap_or(1) as u8)
}

#[cfg(test)]
#[path = "tests/spawn_verification/tests.rs"]
mod tests;
