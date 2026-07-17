//! Repro helpers for scripts/website/mission-version-upload-repro.sh (T-162).

use anyhow::{Context, Result, bail};
use serde_json::Value;
use std::fs;
use std::io::{self, Read};
use std::path::Path;

/// Read JSON from stdin; print `.id` (mission create response).
pub fn cmd_mission_id() -> Result<()> {
    let mut buf = String::new();
    io::stdin().read_to_string(&mut buf).context("read stdin")?;
    let v: Value = serde_json::from_str(&buf).context("parse JSON")?;
    let id = v.get("id").and_then(|x| x.as_str()).context("missing id")?;
    println!("{id}");
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
