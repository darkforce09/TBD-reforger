//! T-912.2 — the ONE file allowed to name the dead wave-plan TSVs.
//!
//! `.ai/tickets/wave.lock` replaced both hand-kept TSVs, and `ticket check` carries a
//! fossil-path guard that reds ANY live mention of them outside a tight historical allowlist.
//! This module is on that allowlist, for two jobs that are genuinely about the past:
//!
//!   1. HISTORY READS. The wave-gate's ticket-ledger oracle corroborates a derived wave-close
//!      boundary by reading the plan AT THAT BOUNDARY'S PARENT via `git show` (T-618: the
//!      checkout is not evidence). Every boundary before the T-912.2 cutover has a TSV there
//!      and no lock; refusing to read it would demote every historical close from
//!      "corroborated" to "demand operator confirmation" — a regression in the exact machinery
//!      this program must keep working. History is immutable and TSV-shaped; a reader of
//!      history may name that shape. [`tickets_at`] is the fallback [`super::base`] uses when
//!      `git show rev:.ai/tickets/wave.lock` finds nothing.
//!
//!   2. THE ONE-SHOT MIGRATION. The first `cargo xtask wave repack` on a tree that still has a
//!      TSV compiles the first lock FROM the committed TSV groups and deletes both files, so
//!      the lock and the deletion ride one commit ([`crate::wave_lock`]). After that commit,
//!      [`any_tsv_present`] is false forever and the migration arm is unreachable.
//!
//! Do NOT add a working-tree TSV reader here. The fossil guard exists so no live code ever
//! reads the dead files again; this module reads git blobs and performs the single deletion.

use std::path::Path;

use anyhow::{Context, Result};

/// The dead plan paths — as git blob paths for history reads, and as working-tree paths for
/// exactly one migration run.
pub const LEGACY_PLANS: [&str; 2] = ["docs/platform/wave_plan.tsv", "docs/mod/wave_plan.tsv"];

pub fn any_tsv_present(root: &Path) -> bool {
    LEGACY_PLANS.iter().any(|rel| root.join(rel).is_file())
}

/// Delete both TSVs from the working tree — called exactly once, by the migration repack.
pub fn delete_tsvs(root: &Path) -> Result<()> {
    for rel in LEGACY_PLANS {
        let p = root.join(rel);
        if p.is_file() {
            std::fs::remove_file(&p).with_context(|| p.display().to_string())?;
            println!("deleted {rel}");
        }
    }
    Ok(())
}

/// `(wave label, ticket id)` rows of both working-tree TSVs, platform then mod, file order —
/// the migration's candidate order. Comments, the header row, blank and short lines drop,
/// exactly as the TSV-era parsers dropped them.
pub fn working_tree_rows(root: &Path) -> Result<Vec<(String, String)>> {
    let mut rows = Vec::new();
    for rel in LEGACY_PLANS {
        let p = root.join(rel);
        if !p.is_file() {
            continue;
        }
        let text = std::fs::read_to_string(&p).with_context(|| p.display().to_string())?;
        rows.extend(parse_rows(&text));
    }
    Ok(rows)
}

fn parse_rows(text: &str) -> Vec<(String, String)> {
    text.lines()
        .filter(|l| !l.is_empty())
        .filter_map(|l| {
            let f: Vec<&str> = l.split('\t').collect();
            if f[0].starts_with('#') || f[0] == "wave" || f.len() < 4 {
                return None;
            }
            Some((f[0].to_string(), f[1].to_string()))
        })
        .collect()
}

/// Tickets a HISTORICAL plan blob assigns to wave `n` at revision `rev`, accepting both label
/// spellings (`77` and `w77`).
///
/// The `w`-prefix strip is NOT style tolerance (T-616 note carried over from the deleted
/// working-tree reader): T-616 normalised the WORKING TREE, and this reads history exclusively —
/// every revision at or before wave 79's close still spells those rows `w76`…`w79`, because
/// that is what was committed. Delete the strip and every pre-T-616 wave close becomes
/// unverifiable in one commit.
pub fn tickets_at(rev: &str, n: i64) -> Vec<String> {
    let want = n.to_string();
    let mut out = Vec::new();
    for rel in LEGACY_PLANS {
        let blob = super::git_stdout(&["show", &format!("{rev}:{rel}")]).unwrap_or_default();
        for l in blob.lines() {
            if l.starts_with('#') || l.trim().is_empty() {
                continue;
            }
            if l.starts_with("wave")
                && l[4..]
                    .chars()
                    .next()
                    .map(|c| c.is_whitespace())
                    .unwrap_or(false)
            {
                continue;
            }
            let mut it = l.split('\t');
            let w = it.next().unwrap_or("");
            let t = it.next().unwrap_or("");
            let w = w.strip_prefix('w').unwrap_or(w);
            // awk's `w == n` after `sub()`: numeric when both sides look numeric.
            let eq = match (w.parse::<f64>(), want.parse::<f64>()) {
                (Ok(a), Ok(b)) => a == b,
                _ => w == want,
            };
            if eq && !t.is_empty() {
                out.push(t.to_string());
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_rows_drops_comments_header_blanks_and_short_lines() {
        let text = "# c\nwave\tticket\ttitle\towns\n\n80\tT-1\tTitle\towns\nshort\tline\n";
        assert_eq!(parse_rows(text), vec![("80".into(), "T-1".into())]);
    }
}
