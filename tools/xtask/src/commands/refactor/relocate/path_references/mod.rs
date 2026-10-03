//! The `path` row pass: every reference to a moved file or folder, in every tracked text file.
//!
//! **Role:** produces one file's edits for the manifest's moves: relative literals re-relativised
//! against their anchors ([`relative_references`], [`anchor_resolution`]) and repository-root
//! spellings rewritten through the mapping ([`root_spellings`]), limited to what the file's
//! treatment opens, plus the literals that resolved before the moves and have no single rewrite.
//!
//! **Position:** called by the plan builder ([`super::relocation_plan`]) once per text file, on the
//! file's text before any other pass; its edits are applied before the Rust path and text passes
//! run.
//!
//! **Signals & state:** none; pure functions.
//!
//! **Invariants:** a relative literal's edit outranks a repository-root edit over the same bytes;
//! in Rust source only string literals and comments are edited, never code; a frozen record
//! receives edits inside link destinations only, a closed ticket inside its `spec`, `plan` and
//! `owns` values only; the verification judges exactly the spans [`allowed_spans`] opens, with the
//! same occurrence classes ([`path_tokens::classify_occurrence`]), so a spelling it can see is one
//! this pass rewrites or reports unresolved; the dry run's verification of the planned tree proves
//! that for every run; a relative literal its spelling does not pin to one anchor is never
//! rewritten, only reported as ambiguous.

pub(crate) mod anchor_resolution;
pub(crate) mod markdown_links;
pub(crate) mod path_tokens;
pub(crate) mod relative_references;
pub(crate) mod root_spellings;

use std::ops::Range;

use anchor_resolution::{ReferenceOutcome, ResolutionContext, resolve_and_rewrite};
use relative_references::{ReferenceFileKind, relative_candidates};
use root_spellings::root_spelling_edits;

use super::file_treatment::{FileTreatment, checked_ticket_field_lines};
use super::rust_lexer::{TokenKind, string_content, tokenize};
use super::text_edits::{AllowedSpans, Edit, merge_edits};

/// A literal that resolved before the moves and has no single rewrite.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct UnresolvedReference {
    /// The byte offset of the literal in the file.
    pub(crate) offset: usize,
    /// Why it has no rewrite.
    pub(crate) message: String,
}

/// One file's path-row edits, unresolved literals and literals left as written.
#[derive(Debug, Default)]
pub(crate) struct PathReferenceOutcome {
    /// The edits, merged and in source order.
    pub(crate) edits: Vec<Edit>,
    /// The literals with no single rewrite.
    pub(crate) unresolved: Vec<UnresolvedReference>,
    /// The literals the moves would change that their spelling does not pin to one anchor, left
    /// as written.
    pub(crate) ambiguous: Vec<UnresolvedReference>,
}

/// The path-row edits of `source`, the text of `context.file`, under `treatment`.
pub(crate) fn path_reference_edits(
    source: &str,
    treatment: FileTreatment,
    context: &ResolutionContext<'_>,
) -> PathReferenceOutcome {
    let kind = ReferenceFileKind::of(context.file);
    let allowed = allowed_spans(source, treatment, kind);
    let links_only = treatment == FileTreatment::FrozenDocument;
    let mut outcome = PathReferenceOutcome::default();
    let mut edits = Vec::new();
    for candidate in relative_candidates(source, kind, links_only) {
        if !allowed.allows(&candidate.span) {
            continue;
        }
        let literal = &source[candidate.span.clone()];
        match resolve_and_rewrite(
            literal,
            candidate.leading_slash,
            candidate.anchors,
            candidate.required_match,
            context,
        ) {
            ReferenceOutcome::Rewritten {
                replacement,
                row_line,
            } => edits.push(Edit {
                span: candidate.span,
                replacement,
                row_line,
            }),
            ReferenceOutcome::Unresolvable(message) => {
                outcome.unresolved.push(UnresolvedReference {
                    offset: candidate.span.start,
                    message,
                });
            }
            ReferenceOutcome::Ambiguous(message) => {
                outcome.ambiguous.push(UnresolvedReference {
                    offset: candidate.span.start,
                    message,
                });
            }
            ReferenceOutcome::NotAReference
                if treatment == FileTreatment::Live && names_moved_path(literal, context) =>
            {
                outcome.unresolved.push(UnresolvedReference {
                    offset: candidate.span.start,
                    message: format!(
                        "`{literal}` names a moved path but resolves under none of its anchors; \
                         reword it as a repository-root path"
                    ),
                });
            }
            ReferenceOutcome::NotAReference | ReferenceOutcome::Unchanged => {}
        }
    }
    let root_spellings = root_spelling_edits(source, context.mapping, context.before, &allowed);
    edits.extend(root_spellings.edits);
    outcome.unresolved.extend(root_spellings.unresolved);
    outcome.unresolved.sort_by_key(|item| item.offset);
    outcome.edits = merge_edits(source, edits);
    outcome
}

/// Whether `literal` is led by `./` or `../` and, those segments dropped, starts with a moved
/// path. A literal without that lead is a repository-root spelling, which the root spelling pass
/// owns.
fn names_moved_path(literal: &str, context: &ResolutionContext<'_>) -> bool {
    if !(literal.starts_with("./") || literal.starts_with("../")) {
        return false;
    }
    let named: Vec<&str> = literal
        .split('/')
        .filter(|s| !s.is_empty())
        .skip_while(|s| *s == "." || *s == "..")
        .collect();
    !named.is_empty() && context.mapping.move_of(&named.join("/")).is_some()
}

/// The spans of `source` the path pass may edit under `treatment`.
pub(crate) fn allowed_spans(
    source: &str,
    treatment: FileTreatment,
    kind: ReferenceFileKind,
) -> AllowedSpans {
    let by_treatment = match treatment {
        FileTreatment::Live => AllowedSpans::Everything,
        FileTreatment::FrozenDocument => {
            AllowedSpans::Only(markdown_links::link_destinations(source))
        }
        FileTreatment::ClosedTicket => AllowedSpans::Only(checked_ticket_field_lines(source)),
        FileTreatment::Excluded => AllowedSpans::Only(Vec::new()),
    };
    if kind == ReferenceFileKind::Rust {
        by_treatment.intersect(&AllowedSpans::Only(rust_literal_and_comment_spans(source)))
    } else {
        by_treatment
    }
}

/// The spans of every string literal's content and every comment in Rust source.
fn rust_literal_and_comment_spans(source: &str) -> Vec<Range<usize>> {
    tokenize(source)
        .iter()
        .filter_map(|token| match token.kind {
            TokenKind::StringLiteral => Some(string_content(source, token)),
            TokenKind::LineComment | TokenKind::BlockComment => Some(token.start..token.end),
            _ => None,
        })
        .collect()
}
