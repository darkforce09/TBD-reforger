//! Repository-root spellings of moved paths: `from/…`, `/from/…` and `elsewhere/from/…`.
//!
//! **Role:** finds every occurrence of a moved path written from the repository root (bare, after
//! a leading `/` as in a Markdown root link, or after a deployment prefix such as
//! `/srv/checkout/from`) and rewrites the whole path token through the [`PathMapping`], so a
//! nested move inside a moved folder lands where its own row sends it.
//!
//! **Position:** one of the two path-row passes [`super`] runs over every text file; the other is
//! the relative-reference pass.
//!
//! **Signals & state:** none; pure functions.
//!
//! **Invariants:** only occurrences on segment boundaries count; URLs and `./` or `../` tokens are
//! left to their owners; an occurrence behind other segments is rewritten only when those
//! segments together with it name no tracked path (so `docs/from` stays when it is a folder of its
//! own); the replacement keeps everything before the moved prefix and after it.

use super::super::path_mapping::PathMapping;
use super::super::repository_files::PathSet;
use super::super::text_edits::Edit;
use super::path_tokens::{PathOccurrence, classify_occurrence, match_starts, token_end};

/// The edits that rewrite every repository-root spelling of a moved path in `source`.
pub(crate) fn root_spelling_edits(
    source: &str,
    mapping: &PathMapping,
    before: &PathSet,
) -> Vec<Edit> {
    let bytes = source.as_bytes();
    let mut edits: Vec<Edit> = Vec::new();
    for moved in mapping.moves() {
        for start in match_starts(source, &moved.from) {
            let end = start + moved.from.len();
            let rewrite = match classify_occurrence(source, start, end) {
                PathOccurrence::RepositoryRoot => true,
                PathOccurrence::Embedded { token_start } => {
                    let spelled = source[token_start..end].trim_start_matches('/');
                    !before.contains(spelled)
                }
                PathOccurrence::NotAPath
                | PathOccurrence::Url
                | PathOccurrence::Relative { .. } => false,
            };
            if !rewrite {
                continue;
            }
            let token = &source[start..token_end(bytes, start).max(end)];
            let relocated = mapping.relocate(token);
            if relocated != token {
                edits.push(Edit {
                    span: start..start + token.len(),
                    replacement: relocated.into_owned(),
                    row_line: mapping
                        .move_of(token)
                        .map_or(moved.row_line, |m| m.row_line),
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
