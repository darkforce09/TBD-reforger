//! Where every repository path lands once a manifest's moves are made, and relative-path math.
//!
//! **Role:** turns the `path` rows into one [`PathMapping`] that relocates any repository path by
//! its longest moved prefix, and supplies the lexical path operations the rewrite passes share:
//! segment-boundary containment, `..` normalisation and relative paths between two folders.
//!
//! **Position:** built once per run from [`super::manifest`]; read by the reference passes
//! ([`super::path_references`]), the Rust path pass for folder scopes, the plan application
//! ([`super::plan_application`]) and the verification ([`super::retired_spellings`]).
//!
//! **Signals & state:** none; a mapping is immutable after construction.
//!
//! **Invariants:** a path is inside a folder only when it equals the folder or continues it after a
//! `/`; the empty folder is the repository root; relocation picks the longest matching `from`, so
//! a nested move inside a moved folder wins over its parent's; normalisation never climbs above
//! the repository root (it answers `None` instead).

use std::borrow::Cow;

use super::manifest::{ManifestRow, RowKind};

/// One move: the row that asked for it and the two spellings.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PathMove {
    /// The manifest line of the `path` row.
    pub(crate) row_line: usize,
    /// The path before the move.
    pub(crate) from: String,
    /// The path after the move.
    pub(crate) to: String,
}

/// Every move of one manifest, ready to relocate paths.
#[derive(Clone, Debug, Default)]
pub(crate) struct PathMapping {
    /// The moves, longest `from` first.
    moves: Vec<PathMove>,
}

impl PathMapping {
    /// The mapping of the manifest's `path` rows.
    pub(crate) fn from_rows(rows: &[ManifestRow]) -> PathMapping {
        let mut moves: Vec<PathMove> = rows
            .iter()
            .filter(|row| row.kind == RowKind::Path)
            .map(|row| PathMove {
                row_line: row.line,
                from: row.from.clone(),
                to: row.to.clone(),
            })
            .collect();
        moves.sort_by(|a, b| b.from.len().cmp(&a.from.len()).then(a.from.cmp(&b.from)));
        PathMapping { moves }
    }

    /// Whether the manifest moves nothing.
    pub(crate) fn is_empty(&self) -> bool {
        self.moves.is_empty()
    }

    /// The moves, longest `from` first.
    pub(crate) fn moves(&self) -> &[PathMove] {
        &self.moves
    }

    /// The move whose `from` is the longest folder prefix of `path`, if any.
    pub(crate) fn move_of(&self, path: &str) -> Option<&PathMove> {
        self.moves
            .iter()
            .find(|candidate| is_at_or_below(path, &candidate.from))
    }

    /// Where `path` lands after the moves; unchanged when no move holds it.
    pub(crate) fn relocate<'a>(&self, path: &'a str) -> Cow<'a, str> {
        match self.move_of(path) {
            Some(found) => Cow::Owned(format!("{}{}", found.to, &path[found.from.len()..])),
            None => Cow::Borrowed(path),
        }
    }
}

/// Whether `path` is `folder` or lies below it; the empty folder holds every path.
pub(crate) fn is_at_or_below(path: &str, folder: &str) -> bool {
    folder.is_empty()
        || path
            .strip_prefix(folder)
            .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
}

/// The folder holding `path`; a top-level path sits in the repository root, `""`.
pub(crate) fn parent_folder(path: &str) -> &str {
    path.rsplit_once('/').map_or("", |(folder, _)| folder)
}

/// `folder/relative` with `.` and `..` resolved lexically, or `None` when it climbs above the
/// repository root. Empty segments collapse, so `a//b` is `a/b`.
pub(crate) fn normalize(folder: &str, relative: &str) -> Option<String> {
    let mut segments: Vec<&str> = folder.split('/').filter(|s| !s.is_empty()).collect();
    for segment in relative.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                segments.pop()?;
            }
            name => segments.push(name),
        }
    }
    Some(segments.join("/"))
}

/// The relative path from folder `from_folder` to `target`: `..` steps up to the shared prefix,
/// then the rest of `target`; `.` when the two are the same.
pub(crate) fn relative_path(from_folder: &str, target: &str) -> String {
    let base: Vec<&str> = from_folder.split('/').filter(|s| !s.is_empty()).collect();
    let goal: Vec<&str> = target.split('/').filter(|s| !s.is_empty()).collect();
    let shared = base
        .iter()
        .zip(goal.iter())
        .take_while(|(a, b)| a == b)
        .count();
    let mut parts: Vec<&str> = std::iter::repeat_n("..", base.len() - shared).collect();
    parts.extend(&goal[shared..]);
    if parts.is_empty() {
        ".".to_string()
    } else {
        parts.join("/")
    }
}
