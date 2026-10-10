//! Which tracked files the link check judges, and the area each belongs to.
//!
//! **Role:** selects the judged documents — every Markdown file under the documentation root,
//! every README.md anywhere, the project instructions, and the Markdown and `.mdc` files under the
//! Cursor rule folders, none of them inside the agent artifact tree or the legacy ticket data
//! folder — and files each in one area for the totals, telling frozen records from live documents.
//!
//! **Position:** called by the gate in [`crate::link_check`] for every tracked file; the
//! locations come from [`repository_layout`].
//!
//! **Signals & state:** none; pure functions over repository-relative paths.
//!
//! **Invariants:** every judged file lands in exactly one area, the first that holds it in the
//! order of [`DocumentArea::ALL`]; no file of the agent artifact tree or of the legacy ticket data
//! folder (records awaiting import into the central ticket manager) is ever judged; the ticket
//! documents and the archive are the frozen areas.

use crate::path_regions::{README, file_name, is_markdown, is_within};
use repository_layout::{
    ARCHIVE_DIR, CURSOR_RULE_DIRS, LEGACY_TICKETS_DIR, PROJECT_INSTRUCTIONS, TICKET_DOCUMENTS_DIR,
};
use repository_layout::{ARTIFACTS_DIR, documentation::DOCUMENTATION_ROOT};

/// The extension of a Cursor rule file, Markdown with a front matter header.
const CURSOR_RULE_EXTENSION: &str = "mdc";

/// Where a judged document sits, for the totals and for the rules that skip frozen records.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum DocumentArea {
    /// A live document under the documentation root.
    LiveDocumentation,
    /// A ticket record or an archived document: frozen, only its links change.
    FrozenDocumentation,
    /// A Cursor rule.
    CursorRules,
    /// The project instructions at the repository root.
    ProjectInstructions,
    /// A README.md outside the areas above and the agent artifact tree, the repository root's
    /// included.
    Readmes,
}

impl DocumentArea {
    /// Every area, in the order a path is matched against them and the totals list them.
    pub(super) const ALL: [DocumentArea; 5] = [
        DocumentArea::LiveDocumentation,
        DocumentArea::FrozenDocumentation,
        DocumentArea::CursorRules,
        DocumentArea::ProjectInstructions,
        DocumentArea::Readmes,
    ];

    /// The area's name in the totals.
    pub(super) fn label(self) -> String {
        match self {
            DocumentArea::LiveDocumentation => format!("{DOCUMENTATION_ROOT} live documents"),
            DocumentArea::FrozenDocumentation => format!("{DOCUMENTATION_ROOT} frozen records"),
            DocumentArea::CursorRules => "Cursor rules".to_string(),
            DocumentArea::ProjectInstructions => PROJECT_INSTRUCTIONS.to_string(),
            DocumentArea::Readmes => format!("{README} files elsewhere"),
        }
    }

    /// Whether the area holds frozen records, which only the link rules judge.
    pub(super) fn is_frozen(self) -> bool {
        self == DocumentArea::FrozenDocumentation
    }
}

/// The area of a tracked file the link check judges, or `None` when it judges no such file.
pub(super) fn judged_area(path: &str) -> Option<DocumentArea> {
    if is_within(path, ARTIFACTS_DIR) || is_within(path, LEGACY_TICKETS_DIR) {
        return None;
    }
    if is_within(path, DOCUMENTATION_ROOT) {
        if !is_markdown(path) {
            return None;
        }
        let frozen = is_within(path, TICKET_DOCUMENTS_DIR) || is_within(path, ARCHIVE_DIR);
        return Some(if frozen {
            DocumentArea::FrozenDocumentation
        } else {
            DocumentArea::LiveDocumentation
        });
    }
    if CURSOR_RULE_DIRS
        .iter()
        .any(|folder| is_within(path, folder))
        && (is_markdown(path) || has_extension(path, CURSOR_RULE_EXTENSION))
    {
        return Some(DocumentArea::CursorRules);
    }
    if path == PROJECT_INSTRUCTIONS {
        return Some(DocumentArea::ProjectInstructions);
    }
    (file_name(path) == README).then_some(DocumentArea::Readmes)
}

/// Whether `path`'s extension is `extension`, in any letter case.
fn has_extension(path: &str, extension: &str) -> bool {
    file_name(path)
        .rsplit_once('.')
        .is_some_and(|(stem, found)| !stem.is_empty() && found.eq_ignore_ascii_case(extension))
}
