//! Repository-root spellings of moved paths: `from/…`, `/from/…` and `elsewhere/from/…`.
//!
//! **Role:** finds every occurrence of a moved path written from the repository root (bare, after
//! a leading `/` as in a Markdown root link, after the letter of a control escape as in
//! `"a.rs\0from/b.rs"`, or after a deployment prefix such as `/srv/checkout/from` or
//! `file:///srv/checkout/from`) and rewrites the whole path token through the [`PathMapping`], so
//! a nested move inside a moved folder lands where its own row sends it.
//!
//! **Position:** one of the two path-row passes [`super`] runs over every text file; the other is
//! the relative-reference pass.
//!
//! **Signals & state:** none; pure functions.
//!
//! **Invariants:** only occurrences on segment boundaries and inside the allowed spans count;
//! URLs other than `file:` URLs and `./` or `../` tokens are left to their owners; an occurrence
//! behind other segments is rewritten only when those segments together with it name no tracked
//! path (so `docs/from` stays when it is a folder of its own); an occurrence after an escape letter
//! whose letter and spelling together name a tracked path is unresolved, never guessed; the
//! replacement keeps everything before the moved prefix and after it.

use super::super::path_mapping::PathMapping;
use super::super::repository_files::PathSet;
use super::super::text_edits::{AllowedSpans, Edit};
use super::path_tokens::{PathOccurrence, classify_occurrence, match_starts, token_end};
use super::{PathReferenceOutcome, UnresolvedReference};

/// The edits that rewrite every repository-root spelling of a moved path inside the `allowed`
/// spans of `source`, and the escape-adjacent spellings that have no single reading.
pub(crate) fn root_spelling_edits(
    source: &str,
    mapping: &PathMapping,
    before: &PathSet,
    allowed: &AllowedSpans,
) -> PathReferenceOutcome {
    let bytes = source.as_bytes();
    let mut outcome = PathReferenceOutcome::default();
    for moved in mapping.moves() {
        for start in match_starts(source, &moved.from) {
            let end = start + moved.from.len();
            let rewrite = match classify_occurrence(source, start, end) {
                PathOccurrence::RepositoryRoot => true,
                PathOccurrence::AfterControlEscape => {
                    let glued = &source[start - 1..end];
                    if before.contains(glued) && allowed.allows(&(start..end)) {
                        outcome.unresolved.push(UnresolvedReference {
                            offset: start,
                            message: format!(
                                "`{}` follows an escape letter, and `{glued}` read with that \
                                 letter is a tracked path too; reword it",
                                moved.from
                            ),
                        });
                        continue;
                    }
                    true
                }
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
            let span = start..start + token.len();
            if relocated != token && allowed.allows(&span) {
                outcome.edits.push(Edit {
                    span,
                    replacement: relocated.into_owned(),
                    row_line: mapping
                        .move_of(token)
                        .map_or(moved.row_line, |m| m.row_line),
                });
            }
        }
    }
    outcome.edits.sort_by(|a, b| {
        a.span
            .start
            .cmp(&b.span.start)
            .then(b.span.end.cmp(&a.span.end))
    });
    outcome
}
