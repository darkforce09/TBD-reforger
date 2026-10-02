//! Writing a plan: the `git mv` moves, then every rewritten file at its new path.
//!
//! **Role:** makes a [`RelocationPlan`] real. Moves run shallowest `from` first, each from where
//! the earlier moves left it, with its destination's parent folders created; then every rewritten
//! file is written at its path after the moves.
//!
//! **Position:** the `--apply` step after [`super::relocation_plan::build_plan`] returned a plan
//! with nothing unresolved; the verification runs right after it.
//!
//! **Signals & state:** the checkout's index and working tree, which it changes.
//!
//! **Invariants:** a failed move undoes the moves before it, last first; a failed write restores
//! the files already written and then undoes every move, last first; a plan that holds an
//! unresolved item is never applied.

use std::path::Path;

use super::path_mapping::{is_at_or_below, parent_folder};
use super::relocation_plan::{PlannedMove, RelocationPlan};
use super::repository_files::git;

/// Apply `plan` to the checkout at `root`, or undo what was done and say why it stopped.
pub(crate) fn apply_plan(root: &Path, plan: &RelocationPlan) -> Result<(), String> {
    if !plan.unresolved.is_empty() {
        return Err(format!(
            "{} reference(s) are unresolved; nothing was written",
            plan.unresolved.len()
        ));
    }
    let done = run_moves(root, &plan.moves)?;
    let mut written: Vec<(&str, &str)> = Vec::new();
    for rewrite in &plan.rewrites {
        let target = root.join(&rewrite.new_path);
        if let Err(error) = std::fs::write(&target, &rewrite.rewritten) {
            for (path, original) in written.iter().rev() {
                let _ = std::fs::write(root.join(path), original);
            }
            undo_moves(root, &done);
            return Err(format!(
                "writing {} failed ({error}); every change was undone",
                rewrite.new_path
            ));
        }
        written.push((&rewrite.new_path, &rewrite.original));
    }
    Ok(())
}

/// Run the moves shallowest first; on a failure undo the finished ones, last first.
fn run_moves(root: &Path, moves: &[PlannedMove]) -> Result<Vec<(String, String)>, String> {
    let mut ordered: Vec<&PlannedMove> = moves.iter().collect();
    ordered.sort_by_key(|planned| (planned.from.matches('/').count(), planned.row_line));
    let mut done: Vec<(String, String)> = Vec::new();
    for planned in ordered {
        let source = current_location(&planned.from, &done);
        let parent = parent_folder(&planned.to);
        let prepared = if parent.is_empty() {
            Ok(())
        } else {
            std::fs::create_dir_all(root.join(parent)).map_err(|error| error.to_string())
        };
        let moved = prepared.and_then(|()| {
            git(root, &["mv", "--", &source, &planned.to])
                .map(|_| ())
                .map_err(|cause| format!("{cause:?}"))
        });
        if let Err(reason) = moved {
            undo_moves(root, &done);
            return Err(format!(
                "line {}: `git mv {source} {}` failed ({reason}); the earlier moves were undone",
                planned.row_line, planned.to
            ));
        }
        done.push((source, planned.to.clone()));
    }
    Ok(done)
}

/// Where `path` sits after the moves in `done`, applied in order.
fn current_location(path: &str, done: &[(String, String)]) -> String {
    done.iter().fold(path.to_string(), |current, (from, to)| {
        if is_at_or_below(&current, from) {
            format!("{to}{}", &current[from.len()..])
        } else {
            current
        }
    })
}

/// Undo `done`, last move first.
fn undo_moves(root: &Path, done: &[(String, String)]) {
    for (from, to) in done.iter().rev() {
        let _ = git(root, &["mv", "--", to, from]);
    }
}
