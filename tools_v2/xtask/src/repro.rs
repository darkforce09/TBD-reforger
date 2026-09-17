//! Repro helpers for `cargo xtask repro mission-upload` (T-162 / T-867).
//!
//! `mission-id` and `mission-version-body` remain standalone subcommands (shell and
//! the T-867 orchestrator both use them). The upload orchestrator lives in
//! `gate_mission_version_upload_repro.rs`.

use anyhow::{Context, Result, bail};
use serde_json::Value;
use std::fs;
use std::io::{self, Read};
use std::path::Path;

/// Parse mission-create JSON and return `.id`.
pub fn mission_id_from_json(buf: &str) -> Result<String> {
    let v: Value = serde_json::from_str(buf).context("parse JSON")?;
    let id = v.get("id").and_then(|x| x.as_str()).context("missing id")?;
    Ok(id.to_string())
}

/// Read JSON from stdin; print `.id` (mission create response).
pub fn cmd_mission_id() -> Result<()> {
    let mut buf = String::new();
    io::stdin().read_to_string(&mut buf).context("read stdin")?;
    println!("{}", mission_id_from_json(&buf)?);
    Ok(())
}

/// Write a large mission-version POST body (semver + editor_notes padding).
pub fn cmd_mission_version_body(out: &Path, mb: u64, semver: &str) -> Result<()> {
    if mb == 0 {
        bail!("mb must be >= 1");
    }
    let notes_len = (mb as usize)
        .checked_mul(1024)
        .and_then(|x| x.checked_mul(1024))
        .context("mb too large")?;
    let notes = "x".repeat(notes_len);
    // Match Python: simple % formatting, notes are ASCII x only (no JSON escape needed).
    let body = format!(
        "{{\"semver\":\"{semver}\",\"payload\":{{\"spawns\":[]}},\"editor_notes\":\"{notes}\"}}"
    );
    fs::write(out, body).with_context(|| format!("write {}", out.display()))?;
    Ok(())
}
