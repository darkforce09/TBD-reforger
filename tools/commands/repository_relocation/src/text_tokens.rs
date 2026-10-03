//! The `text` row pass: identifier-like tokens such as package names, on word boundaries.
//!
//! **Role:** rewrites every whole occurrence of a `text` row's `from` token in one file, such as a
//! package name in a manifest, a workflow, a document or a Rust `use` line.
//!
//! **Position:** the last pass the plan builder ([`super::relocation_plan`]) runs over a live file
//! inside the row's scope, after the path and Rust path passes.
//!
//! **Signals & state:** none; pure functions.
//!
//! **Invariants:** an occurrence counts only when neither neighbour is a letter, a digit, `_` or
//! `-`, so `name` never matches inside `name-types`, `name_types` or `other-name`; among rows that
//! match at the same place the longest `from` wins.

use super::manifest::ManifestRow;
use super::path_references::path_tokens::{is_name_byte, match_starts};
use super::text_edits::Edit;

/// The edits of the `text` rows `rows` over `source`.
pub(crate) fn text_token_edits(source: &str, rows: &[&ManifestRow]) -> Vec<Edit> {
    let bytes = source.as_bytes();
    let mut ordered: Vec<&&ManifestRow> = rows.iter().collect();
    ordered.sort_by_key(|row| std::cmp::Reverse(row.from.len()));
    let mut edits = Vec::new();
    for row in ordered {
        for start in match_starts(source, &row.from) {
            let end = start + row.from.len();
            let clean_start = start == 0 || !is_name_byte(bytes[start - 1]);
            let clean_end = end == bytes.len() || !is_name_byte(bytes[end]);
            if clean_start && clean_end {
                edits.push(Edit {
                    span: start..end,
                    replacement: row.to.clone(),
                    row_line: row.line,
                });
            }
        }
    }
    edits.sort_by(|a, b| {
        a.span
            .start
            .cmp(&b.span.start)
            .then(b.span.end.cmp(&a.span.end))
    });
    edits
}
