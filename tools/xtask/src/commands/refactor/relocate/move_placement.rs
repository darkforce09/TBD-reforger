//! Where a manifest's moves land, the rows that would put two things in one place, and the order
//! the apply runs the moves in.
//!
//! **Role:** proves, before anything is written, that the `path` rows place every tracked file at
//! a path no other row claims, so the tree the [`PathMapping`] predicts is the tree `git mv` can
//! make; and orders the moves so each one finds its `to` free when it runs. A `git mv` into a
//! folder that already exists nests the moved folder inside it, so a `to` must never exist when
//! its move runs.
//!
//! **Position:** called by [`super::relocation_plan::build_plan`], which refuses the plan on any
//! conflict and stores the order; [`super::plan_application`] runs the moves in that order.
//!
//! **Signals & state:** none; pure functions over the planned moves and the tracked paths.
//!
//! **Invariants:** two rows never share a `to`; a `to` never lies inside its own `from`; no two
//! tracked files land on one path and no file lands where another needs a folder; a file lands
//! inside another row's `to` only when it comes from that row's `from` or from a row whose `to`
//! lies strictly inside it; a move runs after every move whose `to` holds its `to` and after every
//! move that frees its `to`; rows the order cannot satisfy are refused; every conflict names both
//! manifest lines.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use super::path_mapping::{PathMapping, is_at_or_below, parent_folder};
use super::relocation_plan::PlannedMove;
use super::repository_files::PathSet;

/// Every reason the moves cannot all land where the mapping puts them, one message per pair of
/// rows, each naming both manifest lines.
pub(crate) fn placement_conflicts(
    moves: &[PlannedMove],
    before: &PathSet,
    mapping: &PathMapping,
) -> Vec<String> {
    let mut single = Vec::new();
    let mut pairs: BTreeMap<(usize, usize), String> = BTreeMap::new();
    let mut pair = |a: usize, b: usize, message: String| {
        pairs.entry((a.min(b), a.max(b))).or_insert(message);
    };
    for (index, planned) in moves.iter().enumerate() {
        if is_strictly_below(&planned.to, &planned.from) {
            single.push(format!(
                "line {}: `{}` lies inside its own `from` `{}`",
                planned.row_line, planned.to, planned.from
            ));
        }
        for other in &moves[index + 1..] {
            if other.to == planned.to {
                pair(
                    planned.row_line,
                    other.row_line,
                    format!("both move to `{}`", planned.to),
                );
            }
        }
    }
    let by_line: HashMap<usize, &PlannedMove> = moves
        .iter()
        .map(|planned| (planned.row_line, planned))
        .collect();
    let mut landed: HashMap<String, Option<usize>> = HashMap::new();
    for file in before.files() {
        let row = mapping.move_of(file).map(|found| found.row_line);
        let path = mapping.relocate(file).into_owned();
        match (landed.insert(path.clone(), row), row) {
            (Some(Some(earlier)), Some(line)) => {
                pair(earlier, line, format!("both put a file at `{path}`"));
            }
            (Some(Some(line)), None) | (Some(None), Some(line)) => single.push(format!(
                "line {line}: puts a file at `{path}`, where a tracked file stays"
            )),
            _ => {}
        }
        let Some(placing) = row.and_then(|line| by_line.get(&line)) else {
            continue;
        };
        for other in moves
            .iter()
            .filter(|other| other.row_line != placing.row_line)
        {
            let from_its_source = is_at_or_below(file, &other.from);
            let placed_inside = is_strictly_below(&placing.to, &other.to);
            if is_at_or_below(&path, &other.to) && !from_its_source && !placed_inside {
                let row = placing.row_line;
                pair(
                    row,
                    other.row_line,
                    format!(
                        "line {row} puts `{path}` inside line {}'s destination `{}`",
                        other.row_line, other.to
                    ),
                );
            }
        }
    }
    for (path, row) in &landed {
        let mut folder = parent_folder(path);
        while !folder.is_empty() {
            if let Some(holder) = landed.get(folder) {
                let message =
                    format!("`{folder}` lands as a file and `{path}` needs it as a folder");
                match (holder, row) {
                    (Some(a), Some(b)) => pair(*a, *b, message),
                    (Some(line), None) | (None, Some(line)) => {
                        single.push(format!("line {line}: {message}"));
                    }
                    (None, None) => {}
                }
            }
            folder = parent_folder(folder);
        }
    }
    single.sort();
    single.dedup();
    single.extend(
        pairs
            .into_iter()
            .map(|((a, b), message)| format!("lines {a} and {b}: {message}")),
    );
    single
}

/// The order the moves run in, as indexes into `moves`: a move runs after every move whose `to`
/// strictly holds its `to` (that move needs its `to` absent) and after every move whose `from`
/// holds its `to` (that move frees the place); otherwise shallower `from` first, then manifest
/// order. The rows left in a cycle are refused, named by their lines.
pub(crate) fn execution_order(moves: &[PlannedMove]) -> Result<Vec<usize>, String> {
    let mut waits_for = vec![0usize; moves.len()];
    let mut releases: Vec<Vec<usize>> = vec![Vec::new(); moves.len()];
    for (first, earlier) in moves.iter().enumerate() {
        for (second, later) in moves.iter().enumerate() {
            let must_follow = first != second
                && (is_strictly_below(&later.to, &earlier.to)
                    || is_at_or_below(&later.to, &earlier.from));
            if must_follow {
                waits_for[second] += 1;
                releases[first].push(second);
            }
        }
    }
    let priority = |index: usize| {
        let planned = &moves[index];
        (planned.from.matches('/').count(), planned.row_line, index)
    };
    let mut ready: BTreeSet<(usize, usize, usize)> = (0..moves.len())
        .filter(|index| waits_for[*index] == 0)
        .map(priority)
        .collect();
    let mut order = Vec::with_capacity(moves.len());
    while let Some(next) = ready.pop_first() {
        let index = next.2;
        order.push(index);
        for &released in &releases[index] {
            waits_for[released] -= 1;
            if waits_for[released] == 0 {
                ready.insert(priority(released));
            }
        }
    }
    if order.len() == moves.len() {
        return Ok(order);
    }
    let mut stuck: Vec<usize> = (0..moves.len())
        .filter(|index| waits_for[*index] > 0)
        .map(|index| moves[index].row_line)
        .collect();
    stuck.sort_unstable();
    Err(format!(
        "{}: each needs another's place first, so no order of `git mv` makes them",
        line_list(&stuck)
    ))
}

/// Whether `path` lies below `folder`, not at it.
fn is_strictly_below(path: &str, folder: &str) -> bool {
    path != folder && is_at_or_below(path, folder)
}

/// `lines 2 and 3`, `lines 2, 3 and 5`.
fn line_list(lines: &[usize]) -> String {
    let spelled: Vec<String> = lines.iter().map(usize::to_string).collect();
    match spelled.split_last() {
        Some((last, rest)) if !rest.is_empty() => {
            format!("lines {} and {last}", rest.join(", "))
        }
        _ => format!("line {}", spelled.join("")),
    }
}
