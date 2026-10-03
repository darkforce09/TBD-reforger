//! Where an earlier manifest's scope lies once the manifests after it have moved things.
//!
//! **Role:** composes the `path` rows of the manifests that came after a judged one, in their
//! chronological order ([`super::manifest_chronology`]), so a `rust_path` row's folder or glob
//! scope is judged where those moves put it: a scope a later row moved, or moved with a parent, is
//! judged at its new place, and a scope a later row took files out of is known as emptied.
//!
//! **Position:** built by `--verify` for each committed manifest from the manifests after it, and
//! read by the verification ([`super::retired_spellings`]); a dry run, an apply and a verify of a
//! manifest outside the folder judge with [`LaterMoves::none`], since nothing came after them.
//!
//! **Signals & state:** none; immutable once built.
//!
//! **Invariants:** composition is chronological, each later manifest's mapping applied to the
//! result of the one before it (longest `from` first within one manifest, as
//! [`PathMapping::relocate`] picks it); a scope no later row touches is returned unchanged; a
//! scope counts as emptied only when a later row's `from` lies strictly below it as it stood at
//! that manifest, so a missing scope no later row explains is never excused.

use super::path_mapping::{PathMapping, is_at_or_below};

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

/// A folder scope after the later moves.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FollowedFolder {
    /// Where the folder lies after every later move.
    pub(crate) folder: String,
    /// The first later row that took files out from below the folder, as
    /// `<manifest> line <n>`, when one did.
    pub(crate) emptied_by: Option<String>,
}

impl LaterMoves {
    /// No later manifest: scopes stay where the judged manifest left them.
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

    /// Where `folder`, as the judged manifest's own moves left it, lies after every later move.
    pub(crate) fn follow_folder(&self, folder: &str) -> FollowedFolder {
        let mut current = folder.to_string();
        let mut emptied_by = None;
        for later in &self.manifests {
            if later.mapping.move_of(&current).is_none()
                && emptied_by.is_none()
                && let Some(found) = later
                    .mapping
                    .moves()
                    .iter()
                    .find(|candidate| is_strictly_below(&candidate.from, &current))
            {
                emptied_by = Some(format!("{} line {}", later.label, found.row_line));
            }
            current = later.mapping.relocate(&current).into_owned();
        }
        FollowedFolder {
            folder: current,
            emptied_by,
        }
    }

    /// `pattern` with its literal lead relocated by every later move, as the judged manifest's
    /// own moves relocate it.
    pub(crate) fn follow_pattern(&self, pattern: &str) -> String {
        self.manifests
            .iter()
            .fold(pattern.to_string(), |current, later| {
                later.mapping.relocate(&current).into_owned()
            })
    }
}

/// Whether `path` lies below `folder`, not at it.
fn is_strictly_below(path: &str, folder: &str) -> bool {
    path != folder && is_at_or_below(path, folder)
}
