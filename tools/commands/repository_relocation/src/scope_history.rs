//! Where a manifest's `rust_path` scope lies, and where its retired `path` spellings are in use
//! again, once its own moves and the later manifests' moves are made.
//!
//! **Role:** composes the `path` rows of the judged manifest and then of the manifests after it,
//! in their chronological order ([`super::manifest_chronology`]), so a `rust_path` row's folder or
//! glob scope is judged where those moves put it: a scope a row moved, or moved with a parent, is
//! judged at its new place, and a scope a row took files out of, the manifest's own rows or a later
//! manifest's, one file at a time or a folder at a time, is known as emptied. [`judged_scope`]
//! turns that into the files a row is judged over, the note of an emptied scope, or the
//! did-not-run of a scope no row explains. [`LaterMoves::revivals_of`] lists the later `path` rows
//! whose `to` is a retired `from` or a path below it: at or below such a `to` the retired spelling
//! is a live path again ([`Revivals`]); a `to` above the retired `from` revives nothing.
//!
//! **Position:** built by `--verify` for each committed manifest from the manifests after it, and
//! read by the verification ([`super::retired_spellings`]); a dry run, an apply and a verify of a
//! manifest outside the folder judge with [`LaterMoves::none`], since nothing came after them, and
//! still compose the manifest's own moves.
//!
//! **Signals & state:** none; immutable once built.
//!
//! **Invariants:** composition is chronological, the judged manifest's own mapping first and each
//! later manifest's mapping applied to the result of the one before it (longest `from` first
//! within one manifest, as [`PathMapping::relocate`] picks it); a scope no row touches is returned
//! unchanged; a scope counts as emptied only when a row's `from` lies strictly below it as it
//! stood at that row's manifest and that manifest does not move the scope itself, so a folder
//! emptied file by file within one manifest, or partly by one manifest and the rest by a later
//! one, holds with a note, while a missing scope no row explains is never excused; the files a row
//! took out of a scope are not judged at their destinations, since a `crate::` prefix names
//! another crate's module once its file has left the crate. Only a strictly later manifest
//! revives a spelling, never the judged manifest's own rows; revivals are listed oldest manifest
//! first and, within one manifest, in line order; a revival frees the judged row for good, since a
//! still-later manifest that retires the spelling again judges it with its own `path` row.

use verification_core::{Kind, NotRun, Verdict};

use super::manifest::{ManifestRow, RowScope};
use super::path_mapping::{PathMapping, is_at_or_below};
use super::path_references::path_tokens::ends_on_boundary;
use super::repository_files::TrackedTree;

/// One later manifest: its label in reports and its moves.
#[derive(Clone, Debug)]
struct LaterManifest {
    label: String,
    mapping: PathMapping,
}

/// The moves of every manifest after a judged one, oldest first.
#[derive(Clone, Debug, Default)]
pub(crate) struct LaterMoves {
    manifests: Vec<LaterManifest>,
}

/// A folder scope after the moves.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FollowedFolder {
    /// Where the folder lies after every move.
    pub(crate) folder: String,
    /// The first row that took files out from below the folder, as `<manifest> line <n>`, when
    /// one did.
    pub(crate) emptied_by: Option<String>,
}

impl LaterMoves {
    /// No later manifest: scopes stay where the judged manifest's own moves leave them.
    pub(crate) fn none() -> LaterMoves {
        LaterMoves::default()
    }

    /// The later manifests, oldest first, each as its label and its mapping.
    pub(crate) fn new(manifests: impl IntoIterator<Item = (String, PathMapping)>) -> LaterMoves {
        LaterMoves {
            manifests: manifests
                .into_iter()
                .map(|(label, mapping)| LaterManifest { label, mapping })
                .collect(),
        }
    }

    /// Where `folder`, a scope of the manifest labelled `label` whose moves are `own`, lies after
    /// those moves and every later one.
    pub(crate) fn follow_folder(
        &self,
        label: &str,
        own: &PathMapping,
        folder: &str,
    ) -> FollowedFolder {
        let mut followed = FollowedFolder {
            folder: folder.to_string(),
            emptied_by: None,
        };
        followed.step(label, own);
        for later in &self.manifests {
            followed.step(&later.label, &later.mapping);
        }
        followed
    }

    /// `pattern` with its literal lead relocated by the judged manifest's own moves (`own`) and
    /// then by every later move.
    pub(crate) fn follow_pattern(&self, own: &PathMapping, pattern: &str) -> String {
        self.manifests
            .iter()
            .fold(own.relocate(pattern).into_owned(), |current, later| {
                later.mapping.relocate(&current).into_owned()
            })
    }
}

impl LaterMoves {
    /// The later `path` rows whose `to` is `from` or lies below it, oldest manifest first and in
    /// line order within one manifest; a `to` above `from` revives nothing.
    pub(crate) fn revivals_of(&self, from: &str) -> Revivals {
        let mut revivals = Vec::new();
        for later in &self.manifests {
            let mut moves: Vec<_> = later
                .mapping
                .moves()
                .iter()
                .filter(|row| is_at_or_below(&row.to, from))
                .collect();
            moves.sort_by_key(|row| row.row_line);
            revivals.extend(moves.into_iter().map(|row| Revival {
                prefix: row.to.clone(),
                by: format!("{} line {}", later.label, row.row_line),
            }));
        }
        Revivals {
            from: from.to_string(),
            revivals,
        }
    }
}

/// A later `path` row whose `to` puts a retired spelling back in use: at or below `prefix` the
/// spelling names a live path again.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Revival {
    /// The later row's `to`: the retired `from` or a path below it.
    pub(crate) prefix: String,
    /// The later row, as `<manifest> line <n>`.
    pub(crate) by: String,
}

/// Every revival of one retired `path` spelling, in the order [`LaterMoves::revivals_of`] gives.
#[derive(Clone, Debug, Default)]
pub(crate) struct Revivals {
    /// The retired spelling: the `from` of the judged row.
    from: String,
    revivals: Vec<Revival>,
}

impl Revivals {
    /// Whether the occurrence of the retired spelling that starts `spelled` (the file's text from
    /// the occurrence on) lies at or below a revived prefix: `spelled` begins with the prefix and
    /// the prefix ends on a segment boundary there. A prefix equal to the retired spelling covers
    /// every occurrence of it.
    pub(crate) fn covers_occurrence(&self, spelled: &str) -> bool {
        self.revivals.iter().any(|revival| {
            spelled.starts_with(&revival.prefix)
                && ends_on_boundary(spelled.as_bytes(), revival.prefix.len())
        })
    }

    /// Whether the repository path `path` lies at or below a revived prefix.
    pub(crate) fn covers_path(&self, path: &str) -> bool {
        self.revivals
            .iter()
            .any(|revival| is_at_or_below(path, &revival.prefix))
    }

    /// One `note:` line per revival of the row on `line` of the manifest labelled `label`.
    pub(crate) fn notes(&self, label: &str, line: usize) -> impl Iterator<Item = String> + '_ {
        let label = label.to_string();
        self.revivals.iter().map(move |revival| {
            format!(
                "{label} line {line}: `{}` is in use again at or below `{}`, where {} moved a \
                 path; its spellings there are not judged against this row",
                self.from, revival.prefix, revival.by
            )
        })
    }
}

impl FollowedFolder {
    /// The scope after the moves of `mapping`, the manifest labelled `label`: the first row that
    /// takes files out from below a folder the manifest does not move is recorded as emptying it.
    fn step(&mut self, label: &str, mapping: &PathMapping) {
        if mapping.move_of(&self.folder).is_none()
            && self.emptied_by.is_none()
            && let Some(found) = mapping
                .moves()
                .iter()
                .find(|row| row.from != self.folder && is_at_or_below(&row.from, &self.folder))
        {
            self.emptied_by = Some(format!("{label} line {}", found.row_line));
        }
        self.folder = mapping.relocate(&self.folder).into_owned();
    }
}

/// Where a `rust_path` row's scope is judged.
pub(crate) enum JudgedScope {
    /// The files at or below this scope after every move.
    Files(RowScope),
    /// The folder is gone because a row took its files out: the note says which.
    Emptied(String),
    /// The folder is gone and no row explains it: a did-not-run.
    Missing(Verdict),
}

/// The scope of `row`, a row of the manifest labelled `label` whose moves are `own`, followed
/// through `own` and then `later`, and read against `tree`.
pub(crate) fn judged_scope(
    tree: &impl TrackedTree,
    own: &PathMapping,
    later: &LaterMoves,
    label: &str,
    row: &ManifestRow,
) -> JudgedScope {
    let followed = match &row.scope {
        RowScope::Everywhere => return JudgedScope::Files(RowScope::Everywhere),
        RowScope::Glob(pattern) => {
            return JudgedScope::Files(RowScope::Glob(later.follow_pattern(own, pattern)));
        }
        RowScope::Folder(folder) => later.follow_folder(label, own, folder),
    };
    if tree.paths().contains(&followed.folder) {
        return JudgedScope::Files(RowScope::Folder(followed.folder));
    }
    match followed.emptied_by {
        Some(emptied_by) => JudgedScope::Emptied(format!(
            "{label} line {}: the scope `{}` of `{}` holds nothing after {emptied_by} moved its \
             files; nothing left to judge there",
            row.line,
            row.scope.spelling(),
            row.from
        )),
        None => JudgedScope::Missing(Verdict::did_not_run(
            format!("{label} line {}: the scope of `{}`", row.line, row.from),
            Kind::Ban,
            NotRun::TargetMissing(tree.root().join(&followed.folder)),
        )),
    }
}
