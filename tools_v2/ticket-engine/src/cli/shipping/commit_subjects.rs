//! Ticket ids mined out of commit subjects.
//!
//! `stamp-sha` and the token estimator both need the same question answered: which commits claim
//! this ticket, and when did each land. One git pass answers it for the whole corpus, so neither
//! caller invents its own id-matching rule or its own date normalisation.

use anyhow::{Context, Result, bail};
use regex::Regex;
use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;
use std::sync::LazyLock;

use time::format_description::well_known::Rfc3339;
use time::{OffsetDateTime, UtcOffset};

use crate::validate_rfc3339_utc;

/// One subject commit naming a ticket id. Per-id lists are oldest→newest.
#[derive(Debug, Clone)]
pub struct SubjectCommit {
    pub sha: String,
    /// Author date normalized to UTC `Z` (already `validate_rfc3339_utc`-clean).
    pub date_utc: String,
}

static ID_TOKEN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"T-[0-9]+(?:\.[0-9]+)*").expect("id token regex"));

/// Extract boundary-matched ticket ids from one commit subject, deduped, in order.
/// Maximal munch supplies the trailing boundary (the id is followed by a non-id
/// character or end); the leading guard refuses an ASCII-alphanumeric predecessor,
/// so `XT-90` is not a claim on `T-90`.
pub fn subject_ids(subject: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for m in ID_TOKEN.find_iter(subject) {
        if m.start() > 0 {
            let prev = subject.as_bytes()[m.start() - 1];
            if prev.is_ascii_alphanumeric() {
                continue;
            }
        }
        let id = m.as_str().to_string();
        if !out.contains(&id) {
            out.push(id);
        }
    }
    out
}

/// Render an instant as canonical whole-second UTC `Z` (the `time` dependency carries only the
/// `parsing` feature, so the format is written by hand and then proven through
/// [`validate_rfc3339_utc`] — the two cannot disagree silently).
fn format_utc_z(t: OffsetDateTime) -> String {
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        t.year(),
        u8::from(t.month()),
        t.day(),
        t.hour(),
        t.minute(),
        t.second()
    )
}

/// Any-offset RFC 3339 → UTC `Z` (whole seconds). A naive date-time refuses.
pub fn to_utc_z(rfc3339: &str) -> Result<String> {
    let parsed = OffsetDateTime::parse(rfc3339, &Rfc3339)
        .with_context(|| format!("not an RFC 3339 date-time: {rfc3339:?}"))?;
    let s = format_utc_z(parsed.to_offset(UtcOffset::UTC));
    validate_rfc3339_utc("mined stamp", &s).map_err(anyhow::Error::msg)?;
    Ok(s)
}

/// Mine every id-mentioning subject commit from the checked-out history (HEAD). One git pass;
/// per-id lists come back oldest→newest.
pub fn mine_subjects(root: &Path) -> Result<BTreeMap<String, Vec<SubjectCommit>>> {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["log", "--pretty=%H%x1f%aI%x1f%s"])
        .output()
        .context("run git log")?;
    if !out.status.success() {
        bail!(
            "git log failed (rc {:?}): {}",
            out.status.code(),
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let mut map: BTreeMap<String, Vec<SubjectCommit>> = BTreeMap::new();
    for line in text.lines() {
        let mut parts = line.splitn(3, '\u{1f}');
        let (Some(sha), Some(date), Some(subject)) = (parts.next(), parts.next(), parts.next())
        else {
            continue;
        };
        let ids = subject_ids(subject);
        if ids.is_empty() {
            continue;
        }
        let date_utc = to_utc_z(date).with_context(|| format!("commit {sha}"))?;
        for id in ids {
            map.entry(id).or_default().push(SubjectCommit {
                sha: sha.to_string(),
                date_utc: date_utc.clone(),
            });
        }
    }
    // git log emits newest-first; the miner speaks oldest-first.
    for v in map.values_mut() {
        v.reverse();
    }
    Ok(map)
}

#[cfg(test)]
#[path = "tests/commit_subjects_tests.rs"]
mod tests;
