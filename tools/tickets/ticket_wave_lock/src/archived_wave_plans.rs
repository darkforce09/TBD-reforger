//! The archived wave plans: the two tab-separated plan files that preceded the wave lock.
//!
//! **Role:** reads those plans at past revisions through `git show` ([`tickets_at`]), and serves
//! the one-shot build of a first lock on a tree that still holds them ([`any_tsv_present`],
//! [`working_tree_rows`], [`delete_tsvs`]).
//! **Position:** the platform wave driver calls [`tickets_at`] when a revision has no
//! `.ai/tickets/wave.lock`, to corroborate a wave close against the plan at that close's parent;
//! the repack in `persistence` calls the rest. The paths come from
//! `ticket_model::repository::documentation::ARCHIVED_WAVE_PLANS`.
//! **Signals & state:** none in memory; runs `git show`, and the build of a first lock reads and
//! deletes the working-tree files.
//! **Invariants:** this is the only module that reads the archived plan format. Live code never
//! reads the plans from the working tree except to build the first lock, which deletes them in
//! the same repack, so after that commit [`any_tsv_present`] stays false.

use std::path::Path;

use crate::error::{Result, ResultExt};

use ticket_model::repository::documentation::ARCHIVED_WAVE_PLANS;

/// Whether either archived wave plan exists in the working tree under `root`.
pub fn any_tsv_present(root: &Path) -> bool {
    ARCHIVED_WAVE_PLANS
        .iter()
        .any(|rel| root.join(rel).is_file())
}

/// Deletes both archived plans from the working tree, printing each deleted path; the build of
/// a first lock calls it once, after writing the lock.
pub fn delete_tsvs(root: &Path) -> Result<()> {
    for rel in ARCHIVED_WAVE_PLANS {
        let p = root.join(rel);
        if p.is_file() {
            std::fs::remove_file(&p).with_context(|| p.display().to_string())?;
            println!("deleted {rel}");
        }
    }
    Ok(())
}

/// The `(wave label, ticket id)` rows of both working-tree plans, platform then mod, in file
/// order: the candidate order of the first lock. Comment lines, the header row, blank lines and
/// rows with fewer than four columns are skipped.
pub fn working_tree_rows(root: &Path) -> Result<Vec<(String, String)>> {
    let mut rows = Vec::new();
    for rel in ARCHIVED_WAVE_PLANS {
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

/// The tickets the archived plans at revision `rev` assign to wave `n`; empty when neither plan
/// exists at that revision.
///
/// Both label spellings match (`77` and `w77`): committed revisions keep the spelling they were
/// written with, and some spell their labels with the `w` prefix, so the prefix is stripped
/// before the numeric comparison or those wave closes could not be corroborated.
pub fn tickets_at(root: &Path, rev: &str, n: i64) -> Vec<String> {
    let want = n.to_string();
    let mut out = Vec::new();
    for rel in ARCHIVED_WAVE_PLANS {
        let blob =
            super::history::git_in(root, &["show", &format!("{rev}:{rel}")]).unwrap_or_default();
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
            // Numeric comparison when both sides parse as numbers, string comparison otherwise.
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
#[path = "tests/archived_wave_plans/mod.rs"]
mod tests;
