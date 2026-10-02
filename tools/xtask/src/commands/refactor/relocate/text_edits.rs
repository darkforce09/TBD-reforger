//! Byte-span edits over one file's text, and the spans a pass may touch.
//!
//! **Role:** the edit currency every rewrite pass produces: an [`Edit`] replaces one byte span and
//! names the manifest row it serves; [`merge_edits`] orders a pass's edits and drops overlaps;
//! [`apply_edits`] produces the new text; [`AllowedSpans`] limits a pass to the parts of a file
//! its treatment opens.
//!
//! **Position:** produced by [`super::path_references`], [`super::rust_paths`] and
//! [`super::text_tokens`]; applied by the plan builder ([`super::relocation_plan`]).
//!
//! **Signals & state:** none; pure functions.
//!
//! **Invariants:** applied edits never overlap; among overlapping edits the one listed first wins,
//! so a caller orders its edits by priority before merging; an edit outside every allowed span is
//! never applied.

use std::ops::Range;

/// One replacement of a byte span.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Edit {
    /// The bytes replaced.
    pub(crate) span: Range<usize>,
    /// The text that replaces them.
    pub(crate) replacement: String,
    /// The manifest line of the row the edit serves.
    pub(crate) row_line: usize,
}

/// The parts of a file a pass may edit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum AllowedSpans {
    /// The whole file.
    Everything,
    /// Only edits that lie inside one of these spans.
    Only(Vec<Range<usize>>),
}

impl AllowedSpans {
    /// Whether an edit of `span` is allowed.
    pub(crate) fn allows(&self, span: &Range<usize>) -> bool {
        match self {
            AllowedSpans::Everything => true,
            AllowedSpans::Only(spans) => spans
                .iter()
                .any(|allowed| allowed.start <= span.start && span.end <= allowed.end),
        }
    }

    /// The spans both `self` and `other` allow.
    pub(crate) fn intersect(&self, other: &AllowedSpans) -> AllowedSpans {
        match (self, other) {
            (AllowedSpans::Everything, any) | (any, AllowedSpans::Everything) => any.clone(),
            (AllowedSpans::Only(left), AllowedSpans::Only(right)) => AllowedSpans::Only(
                left.iter()
                    .flat_map(|a| {
                        right.iter().filter_map(move |b| {
                            let start = a.start.max(b.start);
                            let end = a.end.min(b.end);
                            (start < end).then_some(start..end)
                        })
                    })
                    .collect(),
            ),
        }
    }
}

/// `edits` in source order with every edit that overlaps an earlier-listed one dropped, and
/// edits that change nothing dropped.
pub(crate) fn merge_edits(source: &str, edits: Vec<Edit>) -> Vec<Edit> {
    let mut kept: Vec<Edit> = Vec::new();
    for edit in edits {
        if source.get(edit.span.clone()) == Some(edit.replacement.as_str()) {
            continue;
        }
        let overlaps = kept.iter().any(|other| {
            edit.span.start < other.span.end && other.span.start < edit.span.end
                || (edit.span.is_empty() && edit.span.start == other.span.start)
        });
        if !overlaps {
            kept.push(edit);
        }
    }
    kept.sort_by_key(|edit| (edit.span.start, edit.span.end));
    kept
}

/// `source` with every edit applied; the edits must come from [`merge_edits`].
pub(crate) fn apply_edits(source: &str, edits: &[Edit]) -> String {
    let mut output = String::with_capacity(source.len());
    let mut cursor = 0;
    for edit in edits {
        output.push_str(&source[cursor..edit.span.start]);
        output.push_str(&edit.replacement);
        cursor = edit.span.end;
    }
    output.push_str(&source[cursor..]);
    output
}
