//! Writing a plan: the `git mv` moves, then every rewritten file at its new path, all undone on
//! any failure.
//!
//! **Role:** makes a [`RelocationPlan`] real. Moves run in the plan's order
//! ([`super::move_placement::execution_order`]), each from where the earlier moves left it, into a
//! `to` that must not exist (a `git mv` into an existing folder would nest the moved folder inside
//! it), with its destination's missing parent folders created; then every rewritten file is
//! written at its path after the moves. Every step is journaled, and a failure at any step undoes
//! the journal.
//!
//! **Position:** the `--apply` step after [`super::relocation_plan::build_plan`] returned a plan
//! with nothing unresolved; the verification runs right after it.
//!
//! **Signals & state:** the checkout's index and working tree, which it changes; the journal of
//! one apply, held for its duration.
//!
//! **Invariants:** a plan that holds an unresolved item is never applied; each move lands exactly
//! at its `to`; a failure at any step leaves the index and the working tree byte-identical to
//! before the apply: rewritten files get their original bytes back, the moves are renamed back
//! last first, the folders the apply created are removed deepest first, and the index file is
//! written back from the copy taken before the first move.

use std::path::{Path, PathBuf};

use super::move_placement::{location_after, missing_when_run, occupied_when_run};
use super::path_mapping::parent_folder;
use super::relocation_plan::RelocationPlan;
use super::repository_files::git;

/// Apply `plan` to the checkout at `root`, or undo what was done and say why it stopped.
pub(crate) fn apply_plan(root: &Path, plan: &RelocationPlan) -> Result<(), String> {
    apply_plan_with_checkpoint(root, plan, &mut |_| Ok(()))
}

/// [`apply_plan`], asking `checkpoint` whether to go on after every finished step, a move or a
/// write, with the count of steps finished so far; an `Err` from it stops the apply there and
/// undoes every step, exactly as a failing step does.
pub(crate) fn apply_plan_with_checkpoint(
    root: &Path,
    plan: &RelocationPlan,
    checkpoint: &mut dyn FnMut(usize) -> Result<(), String>,
) -> Result<(), String> {
    if !plan.unresolved.is_empty() {
        return Err(format!(
            "{} reference(s) are unresolved; nothing was written",
            plan.unresolved.len()
        ));
    }
    let index = IndexCopy::take(root)?;
    let mut journal = Journal::default();
    match run_steps(root, plan, &mut journal, checkpoint) {
        Ok(()) => Ok(()),
        Err(reason) => match journal.undo(root, &index) {
            Ok(()) => Err(format!("{reason}; every change was undone")),
            Err(left) => Err(format!(
                "{reason}; the undo stopped and the checkout needs a hand restore: {left}"
            )),
        },
    }
}

/// Run the moves in the plan's order, then the writes, journaling each step.
fn run_steps(
    root: &Path,
    plan: &RelocationPlan,
    journal: &mut Journal,
    checkpoint: &mut dyn FnMut(usize) -> Result<(), String>,
) -> Result<(), String> {
    let mut steps = 0;
    for &index in &plan.move_order {
        let planned = &plan.moves[index];
        let source = location_after(&planned.from, &journal.moves);
        if std::fs::symlink_metadata(root.join(&source)).is_err() {
            return Err(missing_when_run(planned.row_line, &source));
        }
        if std::fs::symlink_metadata(root.join(&planned.to)).is_ok() {
            return Err(occupied_when_run(planned.row_line, &planned.to, &source));
        }
        journal.create_parents(root, parent_folder(&planned.to))?;
        git(root, &["mv", "--", &source, &planned.to]).map_err(|cause| {
            format!(
                "line {}: `git mv {source} {}` failed ({cause:?})",
                planned.row_line, planned.to
            )
        })?;
        journal.moves.push((source, planned.to.clone()));
        steps += 1;
        checkpoint(steps)?;
    }
    for rewrite in &plan.rewrites {
        journal
            .written
            .push((rewrite.new_path.clone(), rewrite.original.clone()));
        std::fs::write(root.join(&rewrite.new_path), &rewrite.rewritten)
            .map_err(|error| format!("writing {} failed ({error})", rewrite.new_path))?;
        steps += 1;
        checkpoint(steps)?;
    }
    Ok(())
}

/// The bytes of the checkout's index file before the apply, and where it lives.
struct IndexCopy {
    path: PathBuf,
    /// `None` when the checkout has no index file yet.
    bytes: Option<Vec<u8>>,
}

impl IndexCopy {
    /// Copy the index file git uses for the checkout at `root`.
    fn take(root: &Path) -> Result<IndexCopy, String> {
        let spelled = git(root, &["rev-parse", "--git-path", "index"])
            .map_err(|cause| format!("the index file could not be located ({cause:?})"))?;
        let path = root.join(spelled.trim());
        let bytes = match std::fs::read(&path) {
            Ok(bytes) => Some(bytes),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(format!("{} could not be read ({error})", path.display())),
        };
        Ok(IndexCopy { path, bytes })
    }

    /// Put the index file back as it was.
    fn restore(&self) -> Result<(), String> {
        let outcome = match &self.bytes {
            Some(bytes) => std::fs::write(&self.path, bytes),
            None => std::fs::remove_file(&self.path).or_else(|error| {
                if error.kind() == std::io::ErrorKind::NotFound {
                    Ok(())
                } else {
                    Err(error)
                }
            }),
        };
        outcome.map_err(|error| format!("restoring {} failed ({error})", self.path.display()))
    }
}

/// Every step one apply took, in order, for the undo.
#[derive(Default)]
struct Journal {
    /// Each finished move: where it moved from (as the tree stood then) and its `to`.
    moves: Vec<(String, String)>,
    /// The folders the apply created, parents before children.
    created_folders: Vec<String>,
    /// Each rewritten file's path after the moves and its text before any pass.
    written: Vec<(String, String)>,
}

impl Journal {
    /// Create `folder` and its missing parents, journaling each one created.
    fn create_parents(&mut self, root: &Path, folder: &str) -> Result<(), String> {
        let mut missing = Vec::new();
        let mut current = folder;
        while !current.is_empty() && std::fs::symlink_metadata(root.join(current)).is_err() {
            missing.push(current.to_string());
            current = parent_folder(current);
        }
        for created in missing.into_iter().rev() {
            std::fs::create_dir(root.join(&created))
                .map_err(|error| format!("creating {created} failed ({error})"))?;
            self.created_folders.push(created);
        }
        Ok(())
    }

    /// Undo every journaled step, last first, then restore the index; the first step that cannot
    /// be undone is reported, after every other step was still tried.
    fn undo(&self, root: &Path, index: &IndexCopy) -> Result<(), String> {
        let mut failures = Vec::new();
        for (path, original) in self.written.iter().rev() {
            let target = root.join(path);
            let current = std::fs::read(&target).ok();
            if current.as_deref() != Some(original.as_bytes())
                && let Err(error) = std::fs::write(&target, original)
            {
                failures.push(format!("restoring {path} failed ({error})"));
            }
        }
        for (from, to) in self.moves.iter().rev() {
            if let Err(error) = std::fs::rename(root.join(to), root.join(from)) {
                failures.push(format!("moving {to} back to {from} failed ({error})"));
            }
        }
        for folder in self.created_folders.iter().rev() {
            if let Err(error) = std::fs::remove_dir(root.join(folder)) {
                failures.push(format!(
                    "removing the created folder {folder} failed ({error})"
                ));
            }
        }
        if let Err(failure) = index.restore() {
            failures.push(failure);
        }
        if failures.is_empty() {
            Ok(())
        } else {
            Err(failures.join("; "))
        }
    }
}
