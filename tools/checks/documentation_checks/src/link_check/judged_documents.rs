//! Which tracked files the link check judges, and the area each belongs to.
//!
//! **Role:** selects the judged documents — every Markdown file under the documentation root,
//! every README.md anywhere and the project instructions — and files each in one area for the
//! totals.
//!
//! **Position:** called by the gate in [`crate::link_check`] for every tracked file; the
//! locations come from [`repository_layout`].
//!
//! **Signals & state:** none; pure functions over repository-relative paths.
//!
//! **Invariants:** every judged file lands in exactly one area, the first that holds it in the
//! order of [`DocumentArea::ALL`].

use crate::path_regions::{README, file_name, is_markdown, is_within};
use repository_layout::PROJECT_INSTRUCTIONS;
use repository_layout::documentation::DOCUMENTATION_ROOT;

/// Where a judged document sits, for the totals.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum DocumentArea {
    /// A document under the documentation root.
    Documentation,
    /// The project instructions at the repository root.
    ProjectInstructions,
    /// A README.md outside the documentation root, the repository root's included.
    Readmes,
}

impl DocumentArea {
    /// Every area, in the order a path is matched against them and the totals list them.
    pub(super) const ALL: [DocumentArea; 3] = [
        DocumentArea::Documentation,
        DocumentArea::ProjectInstructions,
        DocumentArea::Readmes,
    ];

    /// The area's name in the totals.
    pub(super) fn label(self) -> String {
        match self {
            DocumentArea::Documentation => format!("{DOCUMENTATION_ROOT} documents"),
            DocumentArea::ProjectInstructions => PROJECT_INSTRUCTIONS.to_string(),
            DocumentArea::Readmes => format!("{README} files elsewhere"),
        }
    }
}

/// The area of a tracked file the link check judges, or `None` when it judges no such file.
pub(super) fn judged_area(path: &str) -> Option<DocumentArea> {
    if is_within(path, DOCUMENTATION_ROOT) {
        return is_markdown(path).then_some(DocumentArea::Documentation);
    }
    if path == PROJECT_INSTRUCTIONS {
        return Some(DocumentArea::ProjectInstructions);
    }
    (file_name(path) == README).then_some(DocumentArea::Readmes)
}
