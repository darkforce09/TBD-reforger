//! The text a dry run prints: per row, what moves and how many references change, by file kind.
//!
//! **Role:** renders a [`RelocationPlan`] as one block per manifest row (tracked files moved,
//! references rewritten per file kind), the unresolved list and the list of ambiguous literals
//! left as written, with `path:line` for each, and one totals line.
//!
//! **Position:** printed by `--dry-run`, and by `--apply` before it writes.
//!
//! **Signals & state:** none; pure functions.
//!
//! **Invariants:** every count is derived from the plan alone, so a dry run and the apply that
//! follows it report the same numbers; file kinds are listed in name order.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use super::manifest::{ManifestRow, RowKind};
use super::relocation_plan::RelocationPlan;

/// The summary of `plan`, the plan of `rows` from the manifest named `label`.
pub(crate) fn render_summary(label: &str, rows: &[ManifestRow], plan: &RelocationPlan) -> String {
    let mut references: BTreeMap<usize, BTreeMap<String, (usize, usize)>> = BTreeMap::new();
    for rewrite in &plan.rewrites {
        let kind = file_kind(&rewrite.old_path);
        for (row, count) in &rewrite.edits_by_row {
            let entry = references
                .entry(*row)
                .or_default()
                .entry(kind.clone())
                .or_default();
            entry.0 += count;
            entry.1 += 1;
        }
    }
    let mut out = String::new();
    let _ = writeln!(out, "relocation plan of {label}");
    for row in rows {
        let scope = match row.scope.spelling() {
            "" => String::new(),
            scope => format!("  (scope {scope})"),
        };
        let _ = writeln!(
            out,
            "  line {:<4} {:<9}  {} -> {}{scope}",
            row.line,
            row.kind.label(),
            row.from,
            row.to
        );
        if row.kind == RowKind::Path {
            let moved = plan
                .moves
                .iter()
                .find(|planned| planned.row_line == row.line)
                .map_or(0, |planned| planned.tracked_files);
            let _ = writeln!(out, "             moves {moved} tracked file(s)");
        }
        let by_kind = references.get(&row.line).cloned().unwrap_or_default();
        let total: usize = by_kind.values().map(|(count, _)| count).sum();
        let files: usize = by_kind.values().map(|(_, files)| files).sum();
        let breakdown: Vec<String> = by_kind
            .iter()
            .map(|(kind, (count, in_files))| format!("{kind} {count} in {in_files}"))
            .collect();
        let detail = if breakdown.is_empty() {
            String::new()
        } else {
            format!(": {}", breakdown.join(", "))
        };
        let _ = writeln!(
            out,
            "             rewrites {total} reference(s) in {files} file(s){detail}"
        );
    }
    if let Some(untied) = references.get(&0) {
        let count: usize = untied.values().map(|(count, _)| count).sum();
        let _ = writeln!(
            out,
            "  {count} reference(s) re-anchored to a crate folder no row moved"
        );
    }
    let _ = writeln!(out, "  unresolved: {}", plan.unresolved.len());
    for item in &plan.unresolved {
        let _ = writeln!(out, "    {}:{}: {}", item.path, item.line, item.message);
    }
    let _ = writeln!(
        out,
        "  ambiguous, left as written (review each): {}",
        plan.ambiguous.len()
    );
    for item in &plan.ambiguous {
        let _ = writeln!(out, "    {}:{}: {}", item.path, item.line, item.message);
    }
    let moved: usize = plan.moves.iter().map(|planned| planned.tracked_files).sum();
    let references_total: usize = plan
        .rewrites
        .iter()
        .flat_map(|rewrite| rewrite.edits_by_row.values())
        .sum();
    let _ = writeln!(
        out,
        "  totals: {moved} tracked file(s) moved, {} file(s) rewritten, {references_total} \
         reference(s), {} unresolved; {} binary file(s) and {} Git LFS pointer(s) move unread; \
         {} tracked file(s) missing from the working tree",
        plan.rewrites.len(),
        plan.unresolved.len(),
        plan.files_not_text,
        plan.large_file_pointers,
        plan.files_missing
    );
    out
}

/// A file's kind for the breakdown: its extension, or its name when it has none.
fn file_kind(path: &str) -> String {
    let name = path.rsplit('/').next().unwrap_or(path);
    match name.rsplit_once('.') {
        Some((stem, extension)) if !stem.is_empty() => extension.to_ascii_lowercase(),
        _ => name.to_string(),
    }
}
