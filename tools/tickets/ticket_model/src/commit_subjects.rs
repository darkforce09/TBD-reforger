//! Ticket ids mined out of commit subjects.
//!
//! **Role:** [`mine_subjects`], one `git log` pass over the checked-out history that maps every
//! ticket id a subject names to the commits that name it, with [`subject_ids`] (the id-matching
//! rule) and [`to_utc_z`] (the date normalisation).
//! **Position:** `ticket_registry`'s `stamp-sha` and `ticket_metrics`' token estimator both ask
//! which commits claim a ticket and when each landed; both ask here, so neither carries its own
//! matching rule or date format.
//! **Signals & state:** none; each call runs `git` once and returns owned data.
//! **Invariants:** an id matches only at a boundary (no ASCII alphanumeric before it, maximal
//! munch after it); each per-id list runs oldest to newest; every date is whole-second UTC `Z`
//! that `time_source::validate_rfc3339_utc` accepts.

use process_runner::Run;
use regex::Regex;
use std::collections::BTreeMap;

use crate::TicketId;
use std::path::Path;
use std::sync::LazyLock;

use time::format_description::well_known::Rfc3339;
use time::{OffsetDateTime, UtcOffset};

use crate::error::{Error, Result, ResultExt};
use time_source::validate_rfc3339_utc;

/// One commit whose subject names a ticket id. Per-id lists run oldest to newest.
#[derive(Debug, Clone)]
pub struct SubjectCommit {
    /// The full 40-character commit SHA, as `git log` prints `%H`.
    pub sha: String,
    /// The author date normalised to whole-second UTC `Z`, already accepted by
    /// `validate_rfc3339_utc`.
    pub date_utc: String,
}

static ID_TOKEN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"T-[0-9]+(?:\.[0-9]+)*").expect("id token regex"));

/// The boundary-matched ticket ids in one commit subject, each once, in order of appearance.
/// Maximal munch supplies the trailing boundary (the id is followed by a non-id
/// character or end); the leading guard refuses an ASCII-alphanumeric predecessor,
/// so a subject prefixed with another letter is not a claim on a ticket id.
pub fn subject_ids(subject: &str) -> Vec<TicketId> {
    let mut out: Vec<TicketId> = Vec::new();
    for m in ID_TOKEN.find_iter(subject) {
        if m.start() > 0 {
            let prev = subject.as_bytes()[m.start() - 1];
            if prev.is_ascii_alphanumeric() {
                continue;
            }
        }
        let id = TicketId::new(m.as_str());
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

/// An RFC 3339 date-time at any offset, rewritten as whole-second UTC `Z`. A date-time without
/// an offset refuses with [`Error::Context`] over the parse error.
pub fn to_utc_z(rfc3339: &str) -> Result<String> {
    let parsed = OffsetDateTime::parse(rfc3339, &Rfc3339)
        .with_context(|| format!("not an RFC 3339 date-time: {rfc3339:?}"))?;
    let s = format_utc_z(parsed.to_offset(UtcOffset::UTC));
    validate_rfc3339_utc("mined stamp", &s)?;
    Ok(s)
}

/// Every commit reachable from `HEAD` in the checkout at `root` whose subject names a ticket
/// id, grouped by id, each list oldest to newest. Fails when `git` cannot run or exits
/// non-zero, and when a claiming commit's author date does not parse.
pub fn mine_subjects(root: &Path) -> Result<BTreeMap<TicketId, Vec<SubjectCommit>>> {
    let out = Run::new("git")
        .arg("-C")
        .arg(root)
        .args(["log", "--pretty=%H%x1f%aI%x1f%s"])
        .output()
        .map_err(process_runner::Error::from)
        .context("run git log")?;
    if out.code != 0 {
        return Err(Error::msg(format!(
            "git log failed (rc {:?}): {}",
            Some(out.code),
            out.stderr.trim()
        )));
    }
    let text = out.stdout;
    let mut map: BTreeMap<TicketId, Vec<SubjectCommit>> = BTreeMap::new();
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
