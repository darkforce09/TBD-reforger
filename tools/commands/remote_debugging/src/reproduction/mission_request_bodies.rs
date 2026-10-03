//! The request helpers of `cargo xtask repro mission-upload`.
//!
//! **Role:** `repro mission-id` (the `id` of a mission-create answer) and
//! `repro mission-version-body` (a version body padded to a size in MiB).
//! **Position:** subcommands of their own, routed by [`crate::reproduction::run`], and called in
//! process by the upload orchestrator, [`super::mission_version_upload`].
//! **Signals & state:** `mission-id` reads stdin; `mission-version-body` writes the file it is
//! given.
//! **Invariants:** a size of 0 MiB, or one whose byte count overflows, is refused; the padding is
//! ASCII `x` only, so the body needs no JSON escaping.

use serde_json::Value;
use std::fs;
use std::io::{self, Read};
use std::path::Path;

use crate::error::{Error, Result, ResultExt};

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
        return Err(Error::Refused("mb must be >= 1".to_string()));
    }
    let notes_len = (mb as usize)
        .checked_mul(1024)
        .and_then(|x| x.checked_mul(1024))
        .context("mb too large")?;
    let notes = "x".repeat(notes_len);
    // The notes are ASCII `x` only, so plain formatting writes valid JSON.
    let body = format!(
        "{{\"semver\":\"{semver}\",\"payload\":{{\"spawns\":[]}},\"editor_notes\":\"{notes}\"}}"
    );
    fs::write(out, body).with_context(|| format!("write {}", out.display()))?;
    Ok(())
}
