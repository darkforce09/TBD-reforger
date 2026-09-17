//! Git changes.

use super::*;
use anyhow::Context;

// ── LOC mining (the diff_loc input) ────────────────────────────────────────────────────
/// The bookkeeping exclusion (documented in [`FACTOR_DOC_REL`]): paths whose churn
/// is registry/sync/lockfile noise, not implementation work. Matched against the
/// raw numstat path text (rename syntax `old => new` is matched as-is; `.ai/...`
/// renames keep their prefix, so the rule still holds).
pub fn is_excluded_path(path: &str) -> bool {
    path.starts_with(".ai/")
        || (path.starts_with("docs/TICKET_") && path.ends_with(".md"))
        || path == "Cargo.lock"
        || path.ends_with("/Cargo.lock")
}

/// Parse `git log --numstat --pretty=%H` output into `sha → LOC changed` over
/// INCLUDED paths. Every commit gets an entry (0 when it only touched excluded or
/// binary paths — merges too: they emit no numstat lines under the default log).
pub fn parse_numstat(text: &str) -> BTreeMap<String, u64> {
    let mut map: BTreeMap<String, u64> = BTreeMap::new();
    let mut cur: Option<String> = None;
    for line in text.lines() {
        if line.len() == 40
            && line
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            map.entry(line.to_string()).or_insert(0);
            cur = Some(line.to_string());
            continue;
        }
        if line.is_empty() {
            continue;
        }
        let mut parts = line.splitn(3, '\t');
        let (Some(ins), Some(del), Some(path)) = (parts.next(), parts.next(), parts.next()) else {
            continue;
        };
        let Some(sha) = cur.clone() else { continue };
        if is_excluded_path(path) {
            continue;
        }
        // Binary files report "-\t-\tpath" — no line counts exist; they count zero.
        let (Ok(i), Ok(d)) = (ins.parse::<u64>(), del.parse::<u64>()) else {
            continue;
        };
        *map.entry(sha).or_insert(0) += i + d;
    }
    map
}

/// One batched `git log --numstat` pass over main history (HEAD) — the same
/// history walk [`mine_subjects`] reads, so every subject SHA has an entry.
pub fn collect_numstat(root: &Path) -> Result<BTreeMap<String, u64>> {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["log", "--numstat", "--pretty=%H"])
        .output()
        .context("run git log --numstat")?;
    if !out.status.success() {
        bail!(
            "git log --numstat failed (rc {:?}): {}",
            out.status.code(),
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(parse_numstat(&String::from_utf8_lossy(&out.stdout)))
}
