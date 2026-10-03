//! Changed lines per commit, read from git history.
//!
//! **Role:** runs one `git log --numstat --pretty=%H` over the checkout ([`collect_numstat`]),
//! parses it into changed lines per commit ([`parse_numstat`]) and names the bookkeeping paths
//! that never count ([`is_excluded_path`]).
//! **Position:** feeds the `diff_loc` estimates of the planners; `ticket_registry`'s
//! `stamp-sha` verb calls [`collect_numstat`]; the excluded prefixes come from
//! `ticket_model::repository::documentation`.
//! **Signals & state:** none; [`collect_numstat`] spawns `git`, the rest is pure.
//! **Invariants:** every commit in the log gets an entry, 0 when it changed only excluded or
//! binary paths; the `.ai/` tree, the retired queue views (`.md` files only) and every
//! `Cargo.lock` never count.

use super::*;
use crate::error::{Error, Result, ResultExt};

/// The bookkeeping exclusion, documented in [`TOKEN_ESTIMATE_FACTOR_DOC`]: paths whose churn is
/// registry, sync and lockfile noise rather than implementation work. Matched against the raw
/// numstat path text — rename syntax `old => new` is matched as-is, and a rename inside an
/// excluded tree keeps that tree's prefix, so the rule still holds.
pub fn is_excluded_path(path: &str) -> bool {
    use ticket_model::repository::documentation::{
        NUMSTAT_EXCLUDED_PREFIXES, RETIRED_QUEUE_VIEW_PREFIX,
    };
    NUMSTAT_EXCLUDED_PREFIXES.iter().any(|prefix| {
        path.starts_with(prefix) && (*prefix != RETIRED_QUEUE_VIEW_PREFIX || path.ends_with(".md"))
    }) || path == "Cargo.lock"
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
/// history walk [`mine_subjects`](ticket_model::commit_subjects::mine_subjects) reads,
/// so every subject SHA has an entry.
pub fn collect_numstat(root: &Path) -> Result<BTreeMap<String, u64>> {
    let out = Run::new("git")
        .arg("-C")
        .arg(root)
        .args(["log", "--numstat", "--pretty=%H"])
        .output()
        .map_err(process_runner::Error::from)
        .context("run git log --numstat")?;
    if out.code != 0 {
        return Err(Error::msg(format!(
            "git log --numstat failed (rc {:?}): {}",
            Some(out.code),
            out.stderr.trim()
        )));
    }
    Ok(parse_numstat(&out.stdout))
}
