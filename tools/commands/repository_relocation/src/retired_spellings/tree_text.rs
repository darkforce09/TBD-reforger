//! The judged tree's text, read once for every manifest a verification judges.
//!
//! **Role:** reads every text file of a [`TrackedTree`] once, in path order, and answers what each
//! manifest needs of it: the treatment of every file under that manifest's treatment areas
//! (computed once per distinct set of areas) and the spans the path pass opens in a file under a
//! treatment (computed on first use, once per file and treatment).
//!
//! **Position:** built by the verification ([`super::judge_manifests`]) before any row is judged;
//! read by its per-file pass over the combined matcher's hits ([`super::spelling_matcher`]).
//!
//! **Signals & state:** the lazily filled allowed-span cells of each file; nothing is shared
//! across runs.
//!
//! **Invariants:** a file is read exactly once per verification run; the files are the tree's text
//! files in [`crate::repository_files::PathSet::files`] order; a treatment and its allowed spans are
//! the ones [`TreatmentAreas::treatment_of`] and [`allowed_spans`] give, so every manifest judges
//! the files it would judge alone; the first file that cannot be read stops the reading and is
//! reported with its cause, never skipped.

use std::cell::OnceCell;

use verification_core::NotRun;

use crate::file_treatment::{FileTreatment, TreatmentAreas};
use crate::path_references::allowed_spans;
use crate::path_references::relative_references::ReferenceFileKind;
use crate::repository_files::{FileContent, TrackedTree};
use crate::text_edits::AllowedSpans;

/// How many treatments a file can have, one allowed-span cell each.
const TREATMENT_COUNT: usize = 5;

/// One text file of the judged tree.
pub(super) struct TextFile {
    /// The repository path.
    pub(super) path: String,
    /// The whole text.
    pub(super) text: String,
    kind: ReferenceFileKind,
    allowed: [OnceCell<AllowedSpans>; TREATMENT_COUNT],
}

impl TextFile {
    /// The spans the path pass opens in this file under `treatment`.
    pub(super) fn allowed(&self, treatment: FileTreatment) -> &AllowedSpans {
        self.allowed[treatment_slot(treatment)]
            .get_or_init(|| allowed_spans(&self.text, treatment, self.kind))
    }

    /// Whether the file is Rust source.
    pub(super) fn is_rust(&self) -> bool {
        self.path.ends_with(".rs")
    }
}

/// The first file that could not be read, and why.
pub(super) struct UnreadFile {
    /// The repository path.
    pub(super) path: String,
    /// The reason.
    pub(super) cause: NotRun,
}

/// Every text file of the judged tree, and each distinct set of treatment areas' verdict on them.
pub(super) struct TreeText {
    files: Vec<TextFile>,
    treatments: Vec<(TreatmentAreas, Vec<FileTreatment>)>,
}

impl TreeText {
    /// Read every text file of `tree` once.
    pub(super) fn read(tree: &impl TrackedTree) -> Result<TreeText, UnreadFile> {
        let mut files = Vec::new();
        for path in tree.paths().files() {
            let content = tree.read(path).map_err(|cause| UnreadFile {
                path: path.clone(),
                cause,
            })?;
            let FileContent::Text(text) = content else {
                continue;
            };
            files.push(TextFile {
                kind: ReferenceFileKind::of(path),
                path: path.clone(),
                text,
                allowed: Default::default(),
            });
        }
        Ok(TreeText {
            files,
            treatments: Vec::new(),
        })
    }

    /// The text files, in path order.
    pub(super) fn files(&self) -> &[TextFile] {
        &self.files
    }

    /// The index of `areas`' treatments of every file, computed the first time those areas are
    /// asked for.
    pub(super) fn treatments_under(&mut self, areas: TreatmentAreas) -> usize {
        if let Some(known) = self.treatments.iter().position(|(seen, _)| *seen == areas) {
            return known;
        }
        let treatments = self
            .files
            .iter()
            .map(|file| areas.treatment_of(&file.path))
            .collect();
        self.treatments.push((areas, treatments));
        self.treatments.len() - 1
    }

    /// The treatment of the file at `file` under the areas at `areas`.
    pub(super) fn treatment(&self, areas: usize, file: usize) -> FileTreatment {
        self.treatments[areas].1[file]
    }
}

/// Whether the verification judges a file of `treatment` at all.
pub(super) fn is_judged(treatment: FileTreatment) -> bool {
    !matches!(
        treatment,
        FileTreatment::Excluded | FileTreatment::FrozenDocument
    )
}

fn treatment_slot(treatment: FileTreatment) -> usize {
    match treatment {
        FileTreatment::Live => 0,
        FileTreatment::FrozenDocument => 1,
        FileTreatment::FrozenIndex => 2,
        FileTreatment::FixtureSource => 3,
        FileTreatment::Excluded => 4,
    }
}
