//! Where a manifest's `rust_path` scope lies once its own moves and the later manifests' moves are
//! made.
//!
//! **Role:** composes the `path` rows of the judged manifest and then of the manifests after it,
//! in their chronological order ([`super::manifest_chronology`]), so a `rust_path` row's folder or
//! glob scope is judged where those moves put it: a scope a row moved, or moved with a parent, is
//! judged at its new place, and a scope a row took files out of, the manifest's own rows or a later
//! manifest's, one file at a time or a folder at a time, is known as emptied. [`judged_scope`]
//! turns that into the files a row is judged over, the note of an emptied scope, or the
//! did-not-run of a scope no row explains.
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
//! another crate's module once its file has left the crate.

use verification_core::{Kind, NotRun, Verdict};

use super::manifest::{ManifestRow, RowScope};
use super::path_mapping::{PathMapping, is_at_or_below};
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
