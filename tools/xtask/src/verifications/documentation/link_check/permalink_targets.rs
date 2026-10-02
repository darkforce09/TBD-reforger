//! The permalink half of the link rule: every permalink of a run, settled in two git batches.
//!
//! **Role:** holds each permalink the link rule meets with where it is written, looks all of
//! them up in one `git cat-file --batch-check` when the run ends, reads the blobs a fragment
//! needs in one `git cat-file --batch`, and judges each against its view: a blob view names a
//! file whose text holds its fragment, or a folder GitHub opens as its tree view; a tree view
//! names a folder and carries no fragment.
//!
//! **Position:** owned by the link rule ([`super::link_targets`]), which hands it every
//! [`Permalink`] a document holds and settles it when the run finishes; the objects come from a
//! [`PermalinkObjects`] source, and fragments are read through [`super::target_resolution`].
//!
//! **Signals & state:** the permalinks waiting for the batch; they live for one run.
//!
//! **Invariants:** each distinct `<commit>:<path>` is looked up once and each blob read once,
//! whatever view names it; a failed lookup is one "did not run" verdict for every permalink and
//! a failed read one for the blobs it needed, never a break; every break names the destination
//! as written.

use std::collections::{BTreeMap, BTreeSet};

use verification_core::{Kind, Verdict};

use super::heading_anchors::document_anchors;
use super::markdown_scan::scan;
use super::repository_permalinks::{
    BlobObject, ObjectLookup, Permalink, PermalinkObjects, PermalinkView,
};
use super::target_resolution::{
    FragmentNeed, fragment_need, line_anchor_problem, line_count, unmatchable_anchor_problem,
};
use super::{BreakRule, RuleFindings};

/// A permalink waiting for the batch lookup, with where it is written.
#[derive(Debug)]
struct PendingPermalink {
    document: String,
    line: usize,
    destination: String,
    permalink: Permalink,
}

/// The permalinks of one run, waiting to be looked up.
#[derive(Debug, Default)]
pub(super) struct PendingPermalinks {
    waiting: Vec<PendingPermalink>,
}

impl PendingPermalinks {
    /// Hold `permalink`, written as `destination` at `line` of `document`, for the batch.
    pub(super) fn hold(
        &mut self,
        document: &str,
        line: usize,
        destination: &str,
        permalink: Permalink,
    ) {
        self.waiting.push(PendingPermalink {
            document: document.to_string(),
            line,
            destination: destination.to_string(),
            permalink,
        });
    }

    /// Look every waiting permalink up in one batch, read the blobs a fragment needs in a second,
    /// and judge each permalink.
    pub(super) fn settle(&mut self, objects: &dyn PermalinkObjects, findings: &mut RuleFindings) {
        let pending = std::mem::take(&mut self.waiting);
        if pending.is_empty() {
            return;
        }
        let names: Vec<String> = pending
            .iter()
            .map(|waiting| waiting.permalink.object_name())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let lookups: BTreeMap<String, ObjectLookup> = match objects.lookup(&names) {
            Ok(found) => names.into_iter().zip(found).collect(),
            Err(cause) => {
                let what = format!(
                    "{} permalink(s) into this repository could not be looked up",
                    pending.len()
                );
                findings.did_not_run(Verdict::did_not_run(what, Kind::Ban, cause));
                return;
            }
        };
        let texts = blob_texts(objects, &pending, &lookups, findings);
        for waiting in pending {
            let lookup = &lookups[&waiting.permalink.object_name()];
            if let Some((rule, message)) = judge_permalink(&waiting, lookup, &texts) {
                findings.broke(&waiting.document, waiting.line, rule, message);
            }
        }
    }
}

/// The text of every found blob a blob view's fragment needs, by blob id; empty, with one
/// "did not run" verdict, when the batch fails.
fn blob_texts(
    objects: &dyn PermalinkObjects,
    pending: &[PendingPermalink],
    lookups: &BTreeMap<String, ObjectLookup>,
    findings: &mut RuleFindings,
) -> BTreeMap<String, String> {
    let needed: Vec<BlobObject> = pending
        .iter()
        .filter(|waiting| {
            waiting.permalink.view == PermalinkView::Blob && waiting.permalink.fragment.is_some()
        })
        .filter_map(|waiting| match &lookups[&waiting.permalink.object_name()] {
            ObjectLookup::Blob(blob) => Some(blob.clone()),
            _ => None,
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    if needed.is_empty() {
        return BTreeMap::new();
    }
    match objects.contents(&needed) {
        Ok(texts) => needed.into_iter().map(|blob| blob.id).zip(texts).collect(),
        Err(cause) => {
            let what = format!(
                "{} permalinked blob(s) could not be read to judge their fragments",
                needed.len()
            );
            findings.did_not_run(Verdict::did_not_run(what, Kind::Ban, cause));
            BTreeMap::new()
        }
    }
}

/// The break a settled permalink makes, if any; a blob whose text could not be read makes none.
fn judge_permalink(
    waiting: &PendingPermalink,
    lookup: &ObjectLookup,
    texts: &BTreeMap<String, String>,
) -> Option<(BreakRule, String)> {
    let written = &waiting.destination;
    let permalink = &waiting.permalink;
    let path = &permalink.path;
    match (permalink.view, lookup, permalink.fragment.as_deref()) {
        (_, ObjectLookup::Unknown { answer }, _) => Some((
            BreakRule::UnknownPermalinkObject,
            format!(
                "`{written}`: `{}` is {answer} in the local history",
                permalink.object_name()
            ),
        )),
        (PermalinkView::Tree, ObjectLookup::Blob(_), _) => {
            Some(no_folder_for_tree_view(written, permalink, "file"))
        }
        (PermalinkView::Tree, ObjectLookup::Other { kind }, _) => {
            Some(no_folder_for_tree_view(written, permalink, kind))
        }
        (_, ObjectLookup::Tree, Some(_)) => Some((
            BreakRule::MissingAnchor,
            format!("`{written}`: `{path}` is a folder, which has no anchors"),
        )),
        (_, ObjectLookup::Other { kind }, Some(_)) => Some((
            BreakRule::MissingAnchor,
            format!("`{written}`: `{path}` is a {kind}, which has no anchors"),
        )),
        (PermalinkView::Blob, ObjectLookup::Blob(blob), Some(fragment)) => {
            let text = texts.get(&blob.id)?;
            fragment_break(written, permalink, fragment, text)
        }
        (_, _, None) => None,
    }
}

/// The break of a tree view whose `<commit>:<path>` the local history holds as a `kind`, not a
/// folder.
fn no_folder_for_tree_view(
    written: &str,
    permalink: &Permalink,
    kind: &str,
) -> (BreakRule, String) {
    (
        BreakRule::UnknownPermalinkObject,
        format!(
            "`{written}`: `{}` is a {kind} in the local history, and a tree view opens a folder",
            permalink.object_name()
        ),
    )
}

/// The break a fragment on a permalinked file with `text` makes, if any.
fn fragment_break(
    written: &str,
    permalink: &Permalink,
    fragment: &str,
    text: &str,
) -> Option<(BreakRule, String)> {
    let path = &permalink.path;
    match fragment_need(path, permalink.plain_view, fragment) {
        FragmentNeed::Anchor(anchor) => {
            (!document_anchors(&scan(text)).contains(&anchor)).then(|| {
                (
                    BreakRule::MissingAnchor,
                    format!("`{written}`: `{path}` has no heading or anchor `{anchor}`"),
                )
            })
        }
        FragmentNeed::Lines(first, last) => {
            line_anchor_problem(first, last, line_count(text)).map(|problem| {
                (
                    BreakRule::LineAnchorOutOfRange,
                    format!("`{written}`: {problem}"),
                )
            })
        }
        FragmentNeed::Unmatchable(anchor) => Some((
            BreakRule::MissingAnchor,
            format!("`{written}`: {}", unmatchable_anchor_problem(path, &anchor)),
        )),
    }
}

#[cfg(test)]
#[path = "tests/permalink_targets.rs"]
mod tests;
