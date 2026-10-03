//! The toolchain and Git identity every readiness receipt records.
//!
//! **Role:** Captures one `--version` line each from `rustc`, `cargo` and `git`: the
//! `tool_versions` a receipt carries.
//!
//! **Position:** `execute_local` in the parent module calls [`versions`] before it runs the local
//! checks, and [`super::operational_recording::RecordingSession::begin`] calls it when a staging
//! recording starts; `evidence.rs` refuses a receipt whose version lines are missing or blank.
//!
//! **Signals & state:** none; spawns each tool once from the repository root.
//!
//! **Invariants:** every tool runs from `root`, so a `rust-toolchain.toml` there selects the
//! toolchain that is identified. A tool that cannot start, times out, exits non-zero or prints
//! no version line fails the capture instead of yielding a blank identity.

use anyhow::{Result, ensure};
use process_runner::Run;
use std::{path::Path, time::Duration};

/// The tools whose `--version` line every receipt records, in receipt order.
const IDENTIFIED_TOOLS: [&str; 3] = ["rustc", "cargo", "git"];

/// How long one `--version` call may take before the capture fails.
const VERSION_TIMEOUT: Duration = Duration::from_secs(10);

/// Returns the trimmed `--version` line of each tool in [`IDENTIFIED_TOOLS`], run from `root`.
pub(super) fn versions(root: &Path) -> Result<Vec<String>> {
    IDENTIFIED_TOOLS
        .into_iter()
        .map(|tool| version(root, tool))
        .collect()
}

/// Runs `<tool> --version` from `root` and returns its trimmed output, refusing a tool that
/// cannot start, times out, exits non-zero or prints nothing.
pub(super) fn version(root: &Path, tool: &str) -> Result<String> {
    let output = Run::new(tool)
        .arg("--version")
        .cwd(root)
        .timeout(VERSION_TIMEOUT)
        .output()
        .map_err(|error| anyhow::anyhow!("{tool} version: {error:?}"))?;
    ensure!(output.code == 0, "cannot identify {tool}");
    let version = output.stdout.trim();
    ensure!(!version.is_empty(), "{tool} printed no version");
    Ok(version.to_owned())
}
