//! The tree a relocation plan would leave, read in memory without writing anything.
//!
//! **Role:** answers the verification's two questions (which paths are tracked, what a file holds)
//! for the checkout as the plan's moves and rewrites would leave it: every tracked path relocated
//! through the [`PathMapping`], each rewritten file holding its planned text, and every other file
//! read from where it lies before the moves.
//!
//! **Position:** built by `--dry-run` and `--apply` from the [`RepositorySnapshot`] and the
//! [`RelocationPlan`]; judged by [`super::retired_spellings::verify_rows`] before anything is
//! written, the same verification `--verify` and the end of `--apply` run on the checkout.
//!
//! **Signals & state:** none held beyond the borrowed snapshot and plan.
//!
//! **Invariants:** the paths are the tracked paths relocated, which `git ls-files` lists after the
//! apply; a file's content is the text the apply writes, or the unchanged file, classified exactly
//! as the snapshot classifies it before the moves.

use std::collections::HashMap;
use std::path::Path;

use verification_core::NotRun;

use super::path_mapping::PathMapping;
use super::relocation_plan::RelocationPlan;
use super::repository_files::{FileContent, PathSet, RepositorySnapshot, TrackedTree};

/// The checkout as one plan would leave it.
pub(crate) struct PlannedTree<'a> {
    snapshot: &'a RepositorySnapshot,
    paths: PathSet,
    /// Each tracked path after the moves, mapped to the path it has before them.
    origins: HashMap<String, &'a str>,
    /// Each rewritten file's path after the moves, mapped to its planned text.
    planned_texts: HashMap<&'a str, &'a str>,
}

impl<'a> PlannedTree<'a> {
    /// The tree `plan`, built from `snapshot` with `mapping`'s moves, would leave.
    pub(crate) fn new(
        snapshot: &'a RepositorySnapshot,
        mapping: &PathMapping,
        plan: &'a RelocationPlan,
    ) -> PlannedTree<'a> {
        let origins: HashMap<String, &'a str> = snapshot
            .paths()
            .files()
            .map(|file| (mapping.relocate(file).into_owned(), file.as_str()))
            .collect();
        PlannedTree {
            snapshot,
            paths: PathSet::from_files(origins.keys().cloned()),
            origins,
            planned_texts: plan
                .rewrites
                .iter()
                .map(|rewrite| (rewrite.new_path.as_str(), rewrite.rewritten.as_str()))
                .collect(),
        }
    }
}

impl TrackedTree for PlannedTree<'_> {
    fn root(&self) -> &Path {
        self.snapshot.root()
    }

    fn paths(&self) -> &PathSet {
        &self.paths
    }

    fn read(&self, path: &str) -> Result<FileContent, NotRun> {
        if let Some(text) = self.planned_texts.get(path) {
            return Ok(FileContent::Text((*text).to_string()));
        }
        match self.origins.get(path) {
            Some(origin) => self.snapshot.read(origin),
            None => Ok(FileContent::Missing),
        }
    }
}
