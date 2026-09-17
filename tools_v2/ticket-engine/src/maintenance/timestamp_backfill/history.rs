//! History.

use super::*;
use anyhow::Context;

/// One subject commit naming a ticket id. Per-id lists are oldest→newest.
#[derive(Debug, Clone)]
pub struct SubjectCommit {
    pub sha: String,
    /// Author date normalized to UTC `Z` (already `validate_rfc3339_utc`-clean).
    pub date_utc: String,
}

pub(super) static ID_TOKEN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"T-[0-9]+(?:\.[0-9]+)*").expect("id token regex"));

/// Extract boundary-matched ticket ids from one commit subject, deduped, in order.
/// Maximal munch supplies the trailing boundary (the id is followed by a non-id
/// character or end); the leading guard refuses an ASCII-alphanumeric predecessor.
pub(super) fn subject_ids(subject: &str) -> Vec<String> {
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

/// Render an instant as canonical whole-second UTC `Z` (xtask's `time` dep carries
/// only the `parsing` feature, so the format is written by hand and then proven
/// through `validate_rfc3339_utc` — the two cannot disagree silently).
pub(super) fn format_utc_z(t: OffsetDateTime) -> String {
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

/// Any-offset RFC 3339 → UTC `Z` (whole seconds).
pub(super) fn to_utc_z(rfc3339: &str) -> Result<String> {
    let parsed = OffsetDateTime::parse(rfc3339, &Rfc3339)
        .with_context(|| format!("not an RFC 3339 date-time: {rfc3339:?}"))?;
    let s = format_utc_z(parsed.to_offset(UtcOffset::UTC));
    validate_rfc3339_utc("mined stamp", &s).map_err(anyhow::Error::msg)?;
    Ok(s)
}

pub(super) fn parse_utc(stamp: &str) -> Result<OffsetDateTime> {
    OffsetDateTime::parse(stamp, &Rfc3339).with_context(|| format!("parse stamp {stamp:?}"))
}

/// Day-precision floor: `YYYY-MM-DDT00:00:00Z`.
pub(super) fn day_floor(t: OffsetDateTime) -> String {
    let t = t.to_offset(UtcOffset::UTC);
    format!(
        "{:04}-{:02}-{:02}T00:00:00Z",
        t.year(),
        u8::from(t.month()),
        t.day()
    )
}

/// Mine every id-mentioning subject commit from main history (HEAD). One git pass;
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

pub(super) fn short_sha(full: &str) -> String {
    full.chars().take(8).collect()
}

/// `shipped_at` value read through BOTH arms (work field / program status).
pub(super) fn shipped_sha_of(t: &Ticket) -> Option<String> {
    match t {
        Ticket::Work(w) => w.shipped_at.clone(),
        Ticket::Program(p) => {
            if let Status::Shipped { shipped_at, .. } = &p.status {
                shipped_at.clone()
            } else {
                None
            }
        }
    }
}

pub(super) fn stamps_of(t: &Ticket) -> (Option<&str>, Option<&str>) {
    match t {
        Ticket::Work(w) => (w.created_at.as_deref(), w.completed_at.as_deref()),
        Ticket::Program(p) => (p.created_at.as_deref(), p.completed_at.as_deref()),
    }
}

pub(super) fn estimated_of(t: &Ticket) -> &[String] {
    match t {
        Ticket::Work(w) => &w.estimated,
        Ticket::Program(p) => &p.estimated,
    }
}

pub(super) fn is_date_shaped(v: &str) -> bool {
    let b = v.as_bytes();
    b.len() == 10
        && b[4] == b'-'
        && b[7] == b'-'
        && b.iter()
            .enumerate()
            .all(|(i, c)| matches!(i, 4 | 7) || c.is_ascii_digit())
}
