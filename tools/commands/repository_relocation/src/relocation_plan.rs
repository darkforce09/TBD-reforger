//! Every move and every file rewrite one manifest asks for, computed before anything is written.
//!
//! **Role:** checks the manifest's moves against the tracked tree, then runs the three passes over
//! every tracked text file in order — the `path` rows ([`super::path_references`]), the
//! `rust_path` rows ([`super::rust_paths`]) and the `text` rows ([`super::text_tokens`]) — and
//! collects the result as a [`RelocationPlan`]: the moves, the new text of every file that changes,
//! and the literals and `use` trees no pass could rewrite.
//!
//! **Position:** built by every `--dry-run` and `--apply` run; printed by
//! [`super::plan_summary`] and written by [`super::plan_application`].
//!
//! **Signals & state:** none held; the plan is a value. Module trees are built per crate, once,
//! only when a `rust_path` row moves a `crate::` module.
//!
//! **Invariants:** nothing is written while a plan is built; each pass reads the text the previous
//! pass produced; a pass edits a file only as far as its [`FileTreatment`] allows; binaries and
//! Git LFS pointers are moved with their folders and never read as text; a move whose `from` is
//! not tracked or whose `to` already exists, and rows that would put two things in one place or
//! that no order of moves can make or that would meet an occupied `to` when the apply runs them
//! ([`super::move_placement`]), refuse the whole plan; the crate folders that anchor relative
//! literals come from the tracked crate manifests plus the untracked ones on disk, the same set on
//! both sides of the moves, so a literal changes only when a row moves what it names or the file
//! holding it.

use std::collections::{BTreeMap, HashMap};

use verification_core::NotRun;

use super::file_treatment::{FileTreatment, TreatmentAreas};
use super::manifest::{ManifestRow, RowKind};
use super::move_placement::{execution_order, placement_conflicts, sequence_conflicts};
use super::path_mapping::{PathMapping, is_at_or_below, parent_folder};
use super::path_references::anchor_resolution::ResolutionContext;
use super::path_references::path_reference_edits;
use super::repository_files::{FileContent, PathSet, RepositorySnapshot};
use super::rust_lexer::line_of;
use super::rust_paths::module_tree::ModuleTree;
use super::rust_paths::path_rules::RustPathRules;
use super::rust_paths::rust_path_edits;
use super::text_edits::{Edit, apply_edits, merge_edits};
use super::text_tokens::text_token_edits;

/// One move of the plan.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PlannedMove {
    /// The `path` row's manifest line.
    pub(crate) row_line: usize,
    /// The tracked path that moves.
    pub(crate) from: String,
    /// Where it lands.
    pub(crate) to: String,
    /// How many tracked files move with it.
    pub(crate) tracked_files: usize,
}

/// One file whose text changes.
#[derive(Clone, Debug)]
pub(crate) struct PlannedRewrite {
    /// The file's path before the moves.
    pub(crate) old_path: String,
    /// The file's path after the moves, where the new text is written.
    pub(crate) new_path: String,
    /// The text before any pass.
    pub(crate) original: String,
    /// The text after every pass.
    pub(crate) rewritten: String,
    /// How many edits each row made, by manifest line.
    pub(crate) edits_by_row: BTreeMap<usize, usize>,
}

/// A literal or `use` tree no pass could rewrite, or a literal a pass left as written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct UnresolvedItem {
    /// The file, before the moves.
    pub(crate) path: String,
    /// The 1-based line.
    pub(crate) line: usize,
    /// Why.
    pub(crate) message: String,
}

/// What one manifest asks of the checkout.
#[derive(Clone, Debug, Default)]
pub(crate) struct RelocationPlan {
    /// The moves, in manifest order.
    pub(crate) moves: Vec<PlannedMove>,
    /// The order the apply runs [`RelocationPlan::moves`] in, as indexes into it
    /// ([`super::move_placement::execution_order`]).
    pub(crate) move_order: Vec<usize>,
    /// The files whose text changes.
    pub(crate) rewrites: Vec<PlannedRewrite>,
    /// What could not be rewritten; a plan with any is never applied.
    pub(crate) unresolved: Vec<UnresolvedItem>,
    /// Relative literals the moves would change that their spelling does not pin to one anchor,
    /// left as written and listed for review; they never stop an apply.
    pub(crate) ambiguous: Vec<UnresolvedItem>,
    /// Tracked files that are binaries or symbolic links, moved but never read.
    pub(crate) files_not_text: usize,
    /// Tracked Git LFS pointers, moved but never read.
    pub(crate) large_file_pointers: usize,
    /// Tracked files deleted from the working tree, which no pass can read.
    pub(crate) files_missing: usize,
}

/// Why a plan could not be built.
#[derive(Debug)]
pub(crate) enum PlanRefusal {
    /// The manifest asks for moves the tree cannot take.
    Refused(Vec<String>),
    /// A file or git query could not be read.
    NotRun(NotRun),
}

impl From<NotRun> for PlanRefusal {
    fn from(cause: NotRun) -> PlanRefusal {
        PlanRefusal::NotRun(cause)
    }
}

/// The plan of `rows` over `snapshot`; `run_manifest` is the manifest's repository path when it
/// lies in the checkout, so it is never rewritten.
pub(crate) fn build_plan(
    snapshot: &RepositorySnapshot,
    rows: &[ManifestRow],
    run_manifest: Option<&str>,
) -> Result<RelocationPlan, PlanRefusal> {
    let tracked = snapshot.paths();
    let mapping = PathMapping::from_rows(rows);
    let (moves, move_order) = checked_moves(snapshot, rows, &mapping)?;
    // A crate being born anchors its files before the moves exactly as after them, so the
    // untracked crate manifests join both sides and only the rows' own moves change an anchor.
    // `after` is the planned tree: a crate manifest a row moves into a folder, or an untracked
    // one already there, owns the files moved below it.
    let before = &PathSet::from_files(
        tracked
            .files()
            .chain(snapshot.untracked_crate_manifests())
            .cloned(),
    );
    let after = PathSet::from_files(
        before
            .files()
            .map(|file| mapping.relocate(file).into_owned()),
    );
    let mut plan = RelocationPlan {
        moves,
        move_order,
        ..RelocationPlan::default()
    };
    let crate_folders = before.crate_folders();
    let areas = TreatmentAreas::current(run_manifest).relocated(&mapping);
    let mut module_trees: HashMap<String, ModuleTree> = HashMap::new();
    for file in tracked.files() {
        let original = match snapshot.read(file)? {
            FileContent::Text(text) => text,
            FileContent::NotText => {
                plan.files_not_text += 1;
                continue;
            }
            FileContent::LargeFilePointer => {
                plan.large_file_pointers += 1;
                continue;
            }
            FileContent::Missing => {
                plan.files_missing += 1;
                continue;
            }
        };
        let treatment = areas.treatment_of(&mapping.relocate(file), &original);
        if treatment == FileTreatment::Excluded {
            continue;
        }
        let context = ResolutionContext {
            file,
            before,
            after: &after,
            mapping: &mapping,
            crate_folders: &crate_folders,
        };
        let mut text = original.clone();
        let mut edits_by_row = BTreeMap::new();
        if !mapping.is_empty() {
            let outcome = path_reference_edits(&text, treatment, &context);
            for unresolved in outcome.unresolved {
                plan.unresolved.push(UnresolvedItem {
                    path: file.clone(),
                    line: line_of(&text, unresolved.offset),
                    message: unresolved.message,
                });
            }
            for ambiguous in outcome.ambiguous {
                plan.ambiguous.push(UnresolvedItem {
                    path: file.clone(),
                    line: line_of(&text, ambiguous.offset),
                    message: ambiguous.message,
                });
            }
            text = apply_counted(&text, &outcome.edits, &mut edits_by_row);
        }
        if treatment == FileTreatment::Live {
            let rules = RustPathRules::from_rows(
                rows.iter()
                    .filter(|row| row.kind == RowKind::RustPath && row.scope.contains(file)),
            );
            if file.ends_with(".rs") && !rules.is_empty() {
                let module = if rules.moves_crate_modules() {
                    let crate_folder = before.crate_folder_of(parent_folder(file));
                    module_trees
                        .entry(crate_folder.clone())
                        .or_insert_with(|| {
                            ModuleTree::build(snapshot.root(), &crate_folder, before)
                        })
                        .module_of(file)
                        .map(<[String]>::to_vec)
                } else {
                    None
                };
                let outcome = rust_path_edits(&text, &rules, module.as_deref());
                for (offset, message) in outcome.unresolved {
                    plan.unresolved.push(UnresolvedItem {
                        path: file.clone(),
                        line: line_of(&text, offset),
                        message,
                    });
                }
                text = apply_counted(&text, &outcome.edits, &mut edits_by_row);
            }
            let text_rows: Vec<&ManifestRow> = rows
                .iter()
                .filter(|row| row.kind == RowKind::Text && row.scope.contains(file))
                .collect();
            if !text_rows.is_empty() {
                let edits = merge_edits(&text, text_token_edits(&text, &text_rows));
                text = apply_counted(&text, &edits, &mut edits_by_row);
            }
        }
        if text != original {
            plan.rewrites.push(PlannedRewrite {
                old_path: file.clone(),
                new_path: mapping.relocate(file).into_owned(),
                original,
                rewritten: text,
                edits_by_row,
            });
        }
    }
    Ok(plan)
}

/// `text` with `edits` applied, each edit counted against its row.
fn apply_counted(text: &str, edits: &[Edit], counts: &mut BTreeMap<usize, usize>) -> String {
    for edit in edits {
        *counts.entry(edit.row_line).or_default() += 1;
    }
    apply_edits(text, edits)
}

/// The moves of the `path` rows and the order they run in, each checked: its `from` is tracked,
/// its `to` is free or freed by another move, and no two rows put things in one place.
fn checked_moves(
    snapshot: &RepositorySnapshot,
    rows: &[ManifestRow],
    mapping: &PathMapping,
) -> Result<(Vec<PlannedMove>, Vec<usize>), PlanRefusal> {
    let before = snapshot.paths();
    let mut errors = Vec::new();
    let mut moves = Vec::new();
    for row in rows.iter().filter(|row| row.kind == RowKind::Path) {
        if !before.contains(&row.from) {
            errors.push(format!(
                "line {}: `{}` names no tracked file or folder",
                row.line, row.from
            ));
        }
        let destination_taken = before.contains(&row.to) || snapshot.root().join(&row.to).exists();
        let freed_by_another_move = rows
            .iter()
            .any(|other| other.kind == RowKind::Path && is_at_or_below(&row.to, &other.from));
        if destination_taken && !freed_by_another_move {
            errors.push(format!("line {}: `{}` already exists", row.line, row.to));
        }
        moves.push(PlannedMove {
            row_line: row.line,
            from: row.from.clone(),
            to: row.to.clone(),
            tracked_files: before
                .files()
                .filter(|file| is_at_or_below(file, &row.from))
                .count(),
        });
    }
    if !errors.is_empty() {
        return Err(PlanRefusal::Refused(errors));
    }
    errors.extend(placement_conflicts(&moves, before, mapping));
    match execution_order(&moves) {
        Ok(order) if errors.is_empty() => {
            let sequence = sequence_conflicts(&moves, &order, before);
            if sequence.is_empty() {
                Ok((moves, order))
            } else {
                Err(PlanRefusal::Refused(sequence))
            }
        }
        Ok(_) => Err(PlanRefusal::Refused(errors)),
        Err(cycle) => {
            errors.push(cycle);
            Err(PlanRefusal::Refused(errors))
        }
    }
}
