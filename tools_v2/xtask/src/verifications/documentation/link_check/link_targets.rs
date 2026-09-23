//! The link rule: every destination in a judged document reaches what it names.
//!
//! **Role:** judges each link a document's scan found. A checkout path must resolve to a tracked
//! file or a folder holding one without climbing above the repository root; a fragment must
//! match a heading or explicit anchor of a rendered Markdown target, or fit a `#L<n>[-L<m>]` line
//! anchor into any other file; a reference must be defined; a URL of this repository must be a
//! sha permalink whose object exists in the local history. External destinations are counted,
//! never fetched.
//!
//! **Position:** a [`DocumentRule`] of the link-check pipeline in [`super::super::link_check`];
//! it reads [`super::target_resolution`] for classification and resolution,
//! [`super::heading_anchors`] for anchors, and settles every permalink of the run in one batch
//! through [`PermalinkObjects`] when the run finishes.
//!
//! **Signals & state:** the anchors and line counts of every target read so far, keyed by path,
//! and the permalinks waiting for the batch; all of it lives for one run.
//!
//! **Invariants:** a target that cannot be read is one "did not run" verdict, never a pass or a
//! break; a failed permalink lookup is one "did not run" verdict for all permalinks; every break
//! names the destination as written.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use verification_core::{Kind, Verdict};

use super::super::path_regions::is_markdown;
use super::super::read_tracked;
use super::heading_anchors::document_anchors;
use super::judged_documents::DocumentArea;
use super::markdown_scan::{ScannedLink, scan};
use super::repository_permalinks::{BlobObject, ObjectLookup, Permalink, PermalinkObjects};
use super::target_resolution::{
    Destination, Resolution, classify, line_anchor, line_anchor_problem, line_count,
    percent_decode, resolve,
};
use super::{BreakRule, DocumentRule, JudgedDocument, RuleContext, RuleFindings};
use crate::core::repository_layout::documentation::PERMALINK_BASE;

/// What a fragment on a file asks of it.
#[derive(Debug, PartialEq, Eq)]
enum FragmentNeed {
    /// A heading or explicit anchor of a rendered Markdown file.
    Anchor(String),
    /// A line range of a file shown as text.
    Lines(usize, usize),
    /// Neither: a non-Markdown file takes line anchors only.
    Unmatchable(String),
}

/// A checkout destination resolved, with what its query and fragment ask of the target.
#[derive(Debug)]
struct CheckoutTarget {
    resolution: Resolution,
    plain_view: bool,
    fragment: Option<String>,
}

/// A permalink waiting for the batch lookup, with where it is written.
#[derive(Debug)]
struct PendingPermalink {
    document: String,
    line: usize,
    destination: String,
    permalink: Permalink,
}

/// How many destinations of each kind the rule judged.
#[derive(Debug, Default)]
struct LinkCounts {
    checkout: usize,
    permalinks: usize,
    repository_pages: usize,
    external: usize,
}

/// The link rule's state for one run.
pub(super) struct LinkTargets<'o> {
    objects: &'o dyn PermalinkObjects,
    /// The anchors of every Markdown target read so far; `None` when it could not be read.
    anchors: HashMap<String, Option<BTreeSet<String>>>,
    /// The line count of every line-anchored target read so far; `None` when it could not be read.
    line_counts: HashMap<String, Option<usize>>,
    pending: Vec<PendingPermalink>,
    counts: LinkCounts,
}

impl<'o> LinkTargets<'o> {
    /// A rule that settles permalinks through `objects`.
    pub(super) fn new(objects: &'o dyn PermalinkObjects) -> LinkTargets<'o> {
        LinkTargets {
            objects,
            anchors: HashMap::new(),
            line_counts: HashMap::new(),
            pending: Vec::new(),
            counts: LinkCounts::default(),
        }
    }

    fn judge_link(
        &mut self,
        document: &str,
        link: &ScannedLink,
        own_anchors: &BTreeSet<String>,
        context: &RuleContext<'_>,
        findings: &mut RuleFindings,
    ) {
        let written = &link.destination;
        let line = link.line;
        match classify(written) {
            Destination::External => self.counts.external += 1,
            Destination::RepositoryUrl => {
                self.counts.repository_pages += 1;
                findings.broke(
                    document,
                    line,
                    BreakRule::NonPermalinkRepositoryUrl,
                    format!(
                        "`{written}` is a page of this repository without a full commit id; link \
                         a repo-root path, or a permalink {PERMALINK_BASE}<commit>/<path>"
                    ),
                );
            }
            Destination::Permalink(permalink) => {
                self.counts.permalinks += 1;
                self.pending.push(PendingPermalink {
                    document: document.to_string(),
                    line,
                    destination: written.clone(),
                    permalink,
                });
            }
            Destination::SameDocument { fragment } => {
                self.counts.checkout += 1;
                let anchor = decoded(&fragment);
                if !anchor.is_empty() && !own_anchors.contains(&anchor) {
                    findings.broke(
                        document,
                        line,
                        BreakRule::MissingAnchor,
                        format!("`{written}`: this document has no heading or anchor `{anchor}`"),
                    );
                }
            }
            Destination::CheckoutPath {
                path,
                plain_view,
                fragment,
            } => {
                self.counts.checkout += 1;
                let target = CheckoutTarget {
                    resolution: if path.is_empty() {
                        Resolution::File(document.to_string())
                    } else {
                        resolve(document, &path, context.tree)
                    },
                    plain_view,
                    fragment,
                };
                if let Some((rule, message)) =
                    self.checkout_break(written, target, context, findings)
                {
                    findings.broke(document, line, rule, message);
                }
            }
        }
    }

    /// The break a checkout destination makes, if any.
    fn checkout_break(
        &mut self,
        written: &str,
        target: CheckoutTarget,
        context: &RuleContext<'_>,
        findings: &mut RuleFindings,
    ) -> Option<(BreakRule, String)> {
        let broken = match (target.resolution, target.fragment) {
            (Resolution::EscapesRepository, _) => (
                BreakRule::EscapesRepository,
                format!("`{written}` climbs above the repository root"),
            ),
            (Resolution::Missing(normalised), _) => (
                BreakRule::MissingTarget,
                format!(
                    "`{written}` resolves to `{normalised}`, which is no tracked file or folder"
                ),
            ),
            (Resolution::Folder(folder), Some(_)) => (
                BreakRule::MissingAnchor,
                format!("`{written}`: `{folder}` is a folder, which has no anchors"),
            ),
            (Resolution::File(file), Some(fragment)) => {
                match fragment_need(&file, target.plain_view, &fragment) {
                    FragmentNeed::Anchor(anchor) => {
                        let anchors = self.markdown_anchors(&file, context, findings)?;
                        if anchors.contains(&anchor) {
                            return None;
                        }
                        (
                            BreakRule::MissingAnchor,
                            format!("`{written}`: `{file}` has no heading or anchor `{anchor}`"),
                        )
                    }
                    FragmentNeed::Lines(first, last) => {
                        let lines = self.line_count(&file, context, findings)?;
                        let problem = line_anchor_problem(first, last, lines)?;
                        (
                            BreakRule::LineAnchorOutOfRange,
                            format!("`{written}`: {problem}"),
                        )
                    }
                    FragmentNeed::Unmatchable(anchor) => (
                        BreakRule::MissingAnchor,
                        unmatchable_message(written, &file, &anchor),
                    ),
                }
            }
            (Resolution::File(_) | Resolution::Folder(_), None) => return None,
        };
        Some(broken)
    }

    /// The anchors of a tracked Markdown file, read once; `None`, reported once as "did not run",
    /// when the file cannot be read.
    fn markdown_anchors(
        &mut self,
        file: &str,
        context: &RuleContext<'_>,
        findings: &mut RuleFindings,
    ) -> Option<&BTreeSet<String>> {
        if !self.anchors.contains_key(file) {
            let anchors = read_target(file, context, findings, "its anchors")
                .map(|text| document_anchors(&scan(&text)));
            self.anchors.insert(file.to_string(), anchors);
        }
        self.anchors.get(file).and_then(Option::as_ref)
    }

    /// The line count of a tracked file, read once; `None`, reported once as "did not run", when
    /// the file cannot be read.
    fn line_count(
        &mut self,
        file: &str,
        context: &RuleContext<'_>,
        findings: &mut RuleFindings,
    ) -> Option<usize> {
        if !self.line_counts.contains_key(file) {
            let lines = read_target(file, context, findings, "its line anchors")
                .map(|text| line_count(&text));
            self.line_counts.insert(file.to_string(), lines);
        }
        self.line_counts.get(file).copied().flatten()
    }

    /// Look every waiting permalink up in one batch, read the blobs a fragment needs in a second,
    /// and judge each permalink.
    fn settle_permalinks(&mut self, findings: &mut RuleFindings) {
        let pending = std::mem::take(&mut self.pending);
        if pending.is_empty() {
            return;
        }
        let names: Vec<String> = pending
            .iter()
            .map(|waiting| waiting.permalink.object_name())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let lookups: BTreeMap<String, ObjectLookup> = match self.objects.lookup(&names) {
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
        let texts = self.blob_texts(&pending, &lookups, findings);
        for waiting in pending {
            let lookup = &lookups[&waiting.permalink.object_name()];
            if let Some((rule, message)) = judge_permalink(&waiting, lookup, &texts) {
                findings.broke(&waiting.document, waiting.line, rule, message);
            }
        }
    }

    /// The text of every found blob a permalink's fragment needs, by blob id; empty, with one
    /// "did not run" verdict, when the batch fails.
    fn blob_texts(
        &self,
        pending: &[PendingPermalink],
        lookups: &BTreeMap<String, ObjectLookup>,
        findings: &mut RuleFindings,
    ) -> BTreeMap<String, String> {
        let needed: Vec<BlobObject> = pending
            .iter()
            .filter(|waiting| waiting.permalink.fragment.is_some())
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
        match self.objects.contents(&needed) {
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
}

impl DocumentRule for LinkTargets<'_> {
    fn judges(&self, _area: DocumentArea) -> bool {
        true
    }

    fn judge(
        &mut self,
        document: &JudgedDocument<'_>,
        context: &RuleContext<'_>,
        findings: &mut RuleFindings,
    ) {
        let own_anchors = document_anchors(document.scan);
        for reference in &document.scan.undefined_references {
            findings.broke(
                document.path,
                reference.line,
                BreakRule::UndefinedReference,
                format!(
                    "`[{}]` names no reference definition in this document",
                    reference.label
                ),
            );
        }
        for link in &document.scan.links {
            self.judge_link(document.path, link, &own_anchors, context, findings);
        }
        self.anchors
            .insert(document.path.to_string(), Some(own_anchors));
    }

    fn finish(&mut self, _context: &RuleContext<'_>, findings: &mut RuleFindings) {
        self.settle_permalinks(findings);
    }

    fn totals(&self) -> Vec<String> {
        let counts = &self.counts;
        let judged =
            counts.checkout + counts.permalinks + counts.repository_pages + counts.external;
        vec![format!(
            "  links: {judged} judged — {} into this checkout, {} permalink(s), {} other page(s) of \
             this repository, {} external and not fetched",
            counts.checkout, counts.permalinks, counts.repository_pages, counts.external
        )]
    }
}

/// The break a settled permalink makes, if any.
fn judge_permalink(
    waiting: &PendingPermalink,
    lookup: &ObjectLookup,
    texts: &BTreeMap<String, String>,
) -> Option<(BreakRule, String)> {
    let written = &waiting.destination;
    let permalink = &waiting.permalink;
    let fragment = permalink.fragment.as_deref();
    match (lookup, fragment) {
        (ObjectLookup::Unknown { answer }, _) => Some((
            BreakRule::UnknownPermalinkBlob,
            format!(
                "`{written}`: `{}` is {answer} in the local history",
                permalink.object_name()
            ),
        )),
        (ObjectLookup::NotBlob { kind }, Some(_)) => Some((
            BreakRule::MissingAnchor,
            format!(
                "`{written}`: `{}` is a {kind}, which has no anchors",
                permalink.path
            ),
        )),
        (ObjectLookup::Blob(blob), Some(fragment)) => {
            let text = texts.get(&blob.id)?;
            match fragment_need(&permalink.path, permalink.plain_view, fragment) {
                FragmentNeed::Anchor(anchor) => (!document_anchors(&scan(text)).contains(&anchor))
                    .then(|| {
                        (
                            BreakRule::MissingAnchor,
                            format!(
                                "`{written}`: `{}` has no heading or anchor `{anchor}`",
                                permalink.path
                            ),
                        )
                    }),
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
                    unmatchable_message(written, &permalink.path, &anchor),
                )),
            }
        }
        (_, None) => None,
    }
}

/// What `fragment` asks of `file`: a heading anchor when GitHub renders the file as Markdown, a
/// line range otherwise.
fn fragment_need(file: &str, plain_view: bool, fragment: &str) -> FragmentNeed {
    let anchor = decoded(fragment);
    if is_markdown(file) && !plain_view {
        return FragmentNeed::Anchor(anchor);
    }
    match line_anchor(&anchor) {
        Some((first, last)) => FragmentNeed::Lines(first, last),
        None => FragmentNeed::Unmatchable(anchor),
    }
}

fn unmatchable_message(written: &str, file: &str, anchor: &str) -> String {
    format!(
        "`{written}`: `{file}` is not rendered Markdown, so `{anchor}` matches nothing; only a \
         #L<n> or #L<n>-L<m> line anchor applies"
    )
}

/// A fragment percent-decoded, or as written when it does not decode.
fn decoded(fragment: &str) -> String {
    percent_decode(fragment).unwrap_or_else(|| fragment.to_string())
}

/// A tracked target's text, or `None` after recording why it could not be read.
fn read_target(
    file: &str,
    context: &RuleContext<'_>,
    findings: &mut RuleFindings,
    purpose: &str,
) -> Option<String> {
    match read_tracked(context.repo_root, file) {
        Ok(text) => Some(text),
        Err(cause) => {
            let what = format!("{file} could not be read to judge {purpose}");
            findings.did_not_run(Verdict::did_not_run(what, Kind::Ban, cause));
            None
        }
    }
}

#[cfg(test)]
#[path = "tests/link_targets.rs"]
mod tests;
