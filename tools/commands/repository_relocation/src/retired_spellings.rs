//! The relocation verification: no live file still spells what a manifest retired.
//!
//! **Role:** judges one or more manifests against a [`TrackedTree`]: the checkout, or the tree a
//! plan would leave. For each `path` row: its `from` is no longer tracked, and no live text file
//! spells it on segment boundaries, whether from the repository root, after a leading `/`, after
//! the letter of a control escape, behind a deployment prefix (a `file:` URL's path included) or
//! through `..` segments that name nothing any more. For each `rust_path` row: no `.rs` file in
//! its scope, read after the moves, still holds the retired prefix in a path, a `use` tree, a
//! comment or a string literal. Each row is one [`Verdict`]. A folder or glob scope is first
//! followed through the manifest's own moves and then those of the manifests after it
//! ([`LaterMoves`], [`judged_scope`]): judged where they put it, and, when they emptied it,
//! judged as holding nothing, with a note.
//!
//! The run is one pass: the tree's text files are read once ([`tree_text`]), every manifest's
//! `path` spellings and `rust_path` prefix segments go into one Aho-Corasick automaton
//! ([`spelling_matcher`]) that scans each file once, and each hit is judged for every row that
//! retires it, under that row's manifest's treatment of the file and within that row's scope. The
//! Rust path pass runs on a file only for a prefix all of whose segments the file spells.
//!
//! **Position:** `cargo xtask refactor relocate --verify` and the last step of `--apply` judge the
//! checkout; `--dry-run` and the first check of `--apply` judge the planned tree
//! ([`super::planned_tree::PlannedTree`]).
//!
//! **Signals & state:** none held; reads the judged tree once per run.
//!
//! **Invariants:** the files judged and the parts of them judged are the ones the rewrite passes
//! may edit (same [`crate::file_treatment::TreatmentAreas::treatment_of`] and
//! [`crate::path_references::allowed_spans`], the areas moved where the manifest's own moves put
//! them), so a clean apply verifies clean; the frozen records and the manifests are never judged,
//! a frozen area's README index only in its link destinations and its Contents tree lines, this
//! tool's test sources only in their code; a spelling that, with the segments before it or the escape letter glued to it, names a path that
//! exists now is another path and no finding; a listing that cannot be read is a did-not-run,
//! never a pass; a folder scope that is missing after every move is a did-not-run unless a row of
//! its own or a later manifest took files out of it; judging several manifests in one run gives each the
//! verdicts, notes and offence order it gets when judged alone (files in path order, offsets in
//! source order).

mod spelling_matcher;
mod tree_text;

use std::cell::OnceCell;
use std::ops::Range;

use aho_corasick::BuildError;
use verification_core::{Finding, Kind, NotRun, Verdict};

use super::file_treatment::{FileTreatment, TreatmentAreas};
use super::manifest::{ManifestRow, RowKind, RowScope};
use super::path_mapping::{PathMapping, normalize, parent_folder};
use super::path_references::path_tokens::{PathOccurrence, classify_occurrence};
use super::repository_files::{PathSet, TrackedTree};
use super::rust_lexer::{code_spans, line_of};
use super::rust_paths::rust_path_edits;
use super::scope_history::{JudgedScope, LaterMoves, judged_scope};
use super::text_edits::AllowedSpans;
use spelling_matcher::{FileHits, SpellingIndex, SpellingMatcher};
use tree_text::{TextFile, TreeText, UnreadFile, is_judged};

/// How many offending lines one finding lists before it counts the rest.
const LISTED_OFFENCES: usize = 40;

/// One manifest to judge: its label, its rows, the manifest being run, and the later moves.
pub(crate) struct ManifestToJudge<'a> {
    /// The manifest's label in reports.
    pub(crate) label: &'a str,
    /// Its rows.
    pub(crate) rows: &'a [ManifestRow],
    /// The repository path of the manifest being run, when it lies in the checkout.
    pub(crate) run_manifest: Option<&'a str>,
    /// The moves of the manifests after it.
    pub(crate) later: &'a LaterMoves,
}

/// The verdicts on one manifest's rows, and the notes on rows that judged nothing.
#[derive(Debug, Default)]
pub(crate) struct RowJudgement {
    /// One verdict per judged row.
    pub(crate) verdicts: Vec<Verdict>,
    /// One line per row that held because the later moves left nothing to judge.
    pub(crate) notes: Vec<String>,
}

/// The offences found so far, per manifest, per row.
type Offences = Vec<Vec<Vec<String>>>;

/// The judgement of every manifest of `manifests` over `tree`, in the same order, each manifest's
/// scopes followed through its later moves.
pub(crate) fn judge_manifests(
    tree: &impl TrackedTree,
    manifests: &[ManifestToJudge<'_>],
) -> Vec<RowJudgement> {
    let mut text = match TreeText::read(tree) {
        Ok(text) => text,
        Err(unread) => return unreadable(tree, manifests, unread),
    };
    let mut index = SpellingIndex::default();
    let plans: Vec<ManifestPlan> = manifests
        .iter()
        .enumerate()
        .map(|(position, manifest)| {
            ManifestPlan::new(tree, &mut text, &mut index, position, manifest)
        })
        .collect();
    let matcher = match SpellingMatcher::new(&index) {
        Ok(matcher) => matcher,
        Err(cause) => return not_matched(manifests, &cause),
    };
    let mut offences: Offences = manifests
        .iter()
        .map(|manifest| still_tracked(tree.paths(), manifest.rows))
        .collect();
    for (position, file) in text.files().iter().enumerate() {
        let hits = matcher.scan(&file.text, file.is_rust());
        let judged = JudgedFile {
            position,
            file,
            text: &text,
        };
        judged.path_offences(tree.paths(), &hits, &index, &plans, &mut offences);
        if file.is_rust() {
            judged.rust_path_offences(&hits, &index, &plans, &mut offences);
        }
    }
    manifests
        .iter()
        .zip(plans)
        .zip(offences)
        .map(|((manifest, plan), offences)| assemble(manifest, plan, offences))
        .collect()
}

/// How one row of a manifest is judged.
enum RowPlan {
    /// A `path` row: offences collected from the combined matcher.
    Path,
    /// A `rust_path` row judged over the files of its scope after every move.
    RustPath(RowScope),
    /// A `rust_path` row whose scope the later moves emptied: held, with this note.
    Emptied(String),
    /// A `rust_path` row whose scope is missing: this did-not-run.
    Missing(Verdict),
    /// A `text` row: never judged.
    Text,
}

/// One manifest's treatment areas and row plans.
struct ManifestPlan {
    /// The index of its treatments in [`TreeText`].
    areas: usize,
    rows: Vec<RowPlan>,
}

impl ManifestPlan {
    fn new(
        tree: &impl TrackedTree,
        text: &mut TreeText,
        index: &mut SpellingIndex,
        position: usize,
        manifest: &ManifestToJudge<'_>,
    ) -> ManifestPlan {
        let mapping = PathMapping::from_rows(manifest.rows);
        let areas = TreatmentAreas::current(manifest.run_manifest).relocated(&mapping);
        let areas = text.treatments_under(areas);
        let mut rows = Vec::with_capacity(manifest.rows.len());
        for (row_index, row) in manifest.rows.iter().enumerate() {
            let at = (position, row_index);
            rows.push(match row.kind {
                RowKind::Path => {
                    index.add_path_row(&row.from, at);
                    RowPlan::Path
                }
                RowKind::RustPath => {
                    match judged_scope(tree, &mapping, manifest.later, manifest.label, row) {
                        JudgedScope::Files(scope) => {
                            index.add_rust_path_row(row, at);
                            RowPlan::RustPath(scope)
                        }
                        JudgedScope::Emptied(note) => RowPlan::Emptied(note),
                        JudgedScope::Missing(verdict) => RowPlan::Missing(verdict),
                    }
                }
                RowKind::Text => RowPlan::Text,
            });
        }
        ManifestPlan { areas, rows }
    }
}

/// One text file under judgement, with its place in [`TreeText`].
struct JudgedFile<'a> {
    position: usize,
    file: &'a TextFile,
    text: &'a TreeText,
}

impl JudgedFile<'_> {
    /// The treatment the manifest of `plan` gives the file.
    fn treatment(&self, plan: &ManifestPlan) -> FileTreatment {
        self.text.treatment(plan.areas, self.position)
    }

    /// Each path spelling's retired occurrences in the file, added to every row that retires the
    /// spelling and whose manifest judges the file, where its treatment opens them.
    fn path_offences(
        &self,
        paths: &PathSet,
        hits: &FileHits,
        index: &SpellingIndex,
        plans: &[ManifestPlan],
        offences: &mut Offences,
    ) {
        for (spelling, starts) in &hits.path_starts {
            let length = index.path_spellings[*spelling].len();
            let retired: Vec<usize> = starts
                .iter()
                .copied()
                .filter(|start| is_retired_spelling(paths, self.file, *start, start + length))
                .collect();
            if retired.is_empty() {
                continue;
            }
            for &(manifest, row) in &index.path_rows[*spelling] {
                let treatment = self.treatment(&plans[manifest]);
                if !is_judged(treatment) {
                    continue;
                }
                let allowed = self.file.allowed(treatment);
                offences[manifest][row].extend(
                    retired
                        .iter()
                        .filter(|start| allowed.allows(&(**start..**start + length)))
                        .map(|start| offence(&self.file.path, &self.file.text, *start)),
                );
            }
        }
    }

    /// The Rust path pass over the file for each prefix it spells every segment of, added to
    /// every row of that prefix whose manifest treats the file as live, or as this tool's test
    /// sources outside their string literals and comments, and whose scope holds it.
    fn rust_path_offences(
        &self,
        hits: &FileHits,
        index: &SpellingIndex,
        plans: &[ManifestPlan],
        offences: &mut Offences,
    ) {
        let code_only = OnceCell::new();
        for (prefix, retired) in index.prefixes.iter().enumerate() {
            if !retired.segments.iter().all(|s| hits.segments.contains(s)) {
                continue;
            }
            let judging: Vec<(usize, usize)> = index.prefix_rows[prefix]
                .iter()
                .copied()
                .filter(|&(manifest, row)| {
                    let plan = &plans[manifest];
                    matches!(
                        self.treatment(plan),
                        FileTreatment::Live | FileTreatment::FixtureSource
                    ) && matches!(&plan.rows[row], RowPlan::RustPath(scope)
                            if scope.contains(&self.file.path))
                })
                .collect();
            if judging.is_empty() {
                continue;
            }
            let outcome = rust_path_edits(&self.file.text, &retired.rules, None);
            let found: Vec<Range<usize>> = outcome
                .edits
                .iter()
                .map(|edit| edit.span.clone())
                .chain(
                    outcome
                        .unresolved
                        .iter()
                        .map(|(offset, _)| *offset..offset + 1),
                )
                .collect();
            for (manifest, row) in judging {
                let fixture = self.treatment(&plans[manifest]) == FileTreatment::FixtureSource;
                let allowed = if fixture {
                    code_only.get_or_init(|| AllowedSpans::Only(code_spans(&self.file.text)))
                } else {
                    &AllowedSpans::Everything
                };
                offences[manifest][row].extend(
                    found
                        .iter()
                        .filter(|span| allowed.allows(span))
                        .map(|span| offence(&self.file.path, &self.file.text, span.start)),
                );
            }
        }
    }
}

/// One offence list per row of `rows`: a `path` row's `from` still tracked opens its list.
fn still_tracked(paths: &PathSet, rows: &[ManifestRow]) -> Vec<Vec<String>> {
    rows.iter()
        .map(|row| match row.kind {
            RowKind::Path if paths.contains(&row.from) => {
                vec![format!("`{}` is still tracked", row.from)]
            }
            _ => Vec::new(),
        })
        .collect()
}

/// One manifest's verdicts and notes, in row order.
fn assemble(
    manifest: &ManifestToJudge<'_>,
    plan: ManifestPlan,
    offences: Vec<Vec<String>>,
) -> RowJudgement {
    let mut judgement = RowJudgement::default();
    for ((row, row_plan), offences) in manifest.rows.iter().zip(plan.rows).zip(offences) {
        let verdict = match row_plan {
            RowPlan::Path | RowPlan::RustPath(_) => verdict(manifest.label, row, offences),
            RowPlan::Emptied(note) => {
                judgement.notes.push(note);
                Verdict::Held
            }
            RowPlan::Missing(verdict) => verdict,
            RowPlan::Text => continue,
        };
        judgement.verdicts.push(verdict);
    }
    judgement
}

/// Every manifest's did-not-run when a file of the tree could not be read: the first with the
/// cause the read gave, the others with the cause a second read of the same file gives.
fn unreadable(
    tree: &impl TrackedTree,
    manifests: &[ManifestToJudge<'_>],
    unread: UnreadFile,
) -> Vec<RowJudgement> {
    let mut first = Some(unread.cause);
    manifests
        .iter()
        .map(|manifest| {
            let cause = first.take().unwrap_or_else(|| {
                tree.read(&unread.path)
                    .err()
                    .unwrap_or_else(|| NotRun::Unreadable {
                        path: tree.root().join(&unread.path),
                        source: std::io::Error::other("unreadable during the verification"),
                    })
            });
            RowJudgement {
                verdicts: vec![Verdict::did_not_run(
                    format!("{}: the tracked files could not be read", manifest.label),
                    Kind::Ban,
                    cause,
                )],
                notes: Vec::new(),
            }
        })
        .collect()
}

/// Every manifest's did-not-run when the combined matcher could not be built.
fn not_matched(manifests: &[ManifestToJudge<'_>], cause: &BuildError) -> Vec<RowJudgement> {
    manifests
        .iter()
        .map(|manifest| RowJudgement {
            verdicts: vec![Verdict::did_not_run(
                format!(
                    "{}: the retired spellings could not be matched",
                    manifest.label
                ),
                Kind::Ban,
                NotRun::ToolError {
                    tool: "the retired-spelling matcher".to_string(),
                    status: -1,
                    stderr: cause.to_string(),
                },
            )],
            notes: Vec::new(),
        })
        .collect()
}

/// Whether the occurrence at `start..end` still spells the retired path.
fn is_retired_spelling(paths: &PathSet, file: &TextFile, start: usize, end: usize) -> bool {
    match classify_occurrence(&file.text, start, end) {
        PathOccurrence::NotAPath | PathOccurrence::Url => false,
        PathOccurrence::RepositoryRoot => true,
        PathOccurrence::AfterControlEscape => !paths.contains(&file.text[start - 1..end]),
        PathOccurrence::Embedded { token_start } => {
            !paths.contains(file.text[token_start..end].trim_start_matches('/'))
        }
        PathOccurrence::Relative { token_start } => {
            let literal = &file.text[token_start..end];
            let folder = parent_folder(&file.path);
            let anchors = [
                folder.to_string(),
                paths.crate_folder_of(folder),
                String::new(),
            ];
            !anchors.iter().any(|anchor| {
                normalize(anchor, literal)
                    .is_some_and(|target| !target.is_empty() && paths.contains(&target))
            })
        }
    }
}

fn offence(path: &str, text: &str, offset: usize) -> String {
    let line = line_of(text, offset);
    let content = text.lines().nth(line - 1).unwrap_or("").trim();
    let shown: String = content.chars().take(120).collect();
    format!("{path}:{line}: {shown}")
}

fn verdict(label: &str, row: &ManifestRow, offences: Vec<String>) -> Verdict {
    if offences.is_empty() {
        return Verdict::Held;
    }
    let mut detail: Vec<String> = offences.iter().take(LISTED_OFFENCES).cloned().collect();
    if offences.len() > LISTED_OFFENCES {
        detail.push(format!("… and {} more", offences.len() - LISTED_OFFENCES));
    }
    Verdict::Failed(Finding {
        headline: format!(
            "{label} line {}: {} retired spelling(s) of `{}` ({}) remain",
            row.line,
            offences.len(),
            row.from,
            row.kind.label()
        ),
        detail,
    })
}
