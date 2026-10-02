//! Which rewrites a tracked file may receive: live, frozen, a closed ticket, or none.
//!
//! **Role:** sorts every file into a [`FileTreatment`]. Frozen Markdown records (the ticket
//! documents and the archive) keep their prose and backticks as history and receive only link
//! destination rewrites; a closed ticket file receives rewrites on its `spec` and `plan` lines
//! only, the two fields the ticket engine checks for existence; the relocation manifests, the
//! manifest being run and every other file in a frozen area receive nothing.
//!
//! **Position:** consulted by the plan builder ([`super::relocation_plan`]) before any pass runs
//! and by the verification ([`super::retired_spellings`]), so both judge the same files.
//!
//! **Signals & state:** none; pure functions over a path and its text.
//!
//! **Invariants:** the frozen areas are the ones [`crate::core::repository_layout::documentation`]
//! names; a ticket is closed exactly when its top-level `status` is one the ticket engine's
//! [`ticket_engine::StatusName`] calls shipped or cancelled; an unknown status counts as open, so
//! its paths keep resolving.

use std::ops::Range;

use ticket_engine::StatusName;

use super::path_mapping::{PathMapping, is_at_or_below, parent_folder};
use crate::core::repository_layout::TICKETS_DIR;
use crate::core::repository_layout::documentation::{
    ARCHIVE_DIR, DOCUMENTATION_ROOT, TICKET_DOCUMENTS_DIR,
};

/// The relocation manifests' folder below the documentation root.
const MANIFESTS_BELOW_DOCUMENTATION: &str = "restructure/manifests";

/// The top-level ticket fields the ticket engine checks for existence.
const CHECKED_TICKET_FIELDS: [&str; 2] = ["spec", "plan"];

/// How much of a file the rewrite passes may change.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FileTreatment {
    /// Every rewrite applies.
    Live,
    /// A frozen Markdown record: link destinations only.
    FrozenDocument,
    /// A closed ticket: its `spec` and `plan` lines only.
    ClosedTicket,
    /// Nothing is rewritten or verified.
    Excluded,
}

/// The folder that holds the committed relocation manifests.
pub(crate) fn manifests_folder() -> String {
    format!("{DOCUMENTATION_ROOT}/{MANIFESTS_BELOW_DOCUMENTATION}")
}

/// The areas that decide a file's treatment, as repository paths.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TreatmentAreas {
    manifests: String,
    archive: String,
    ticket_documents: String,
    tickets: String,
    run_manifest: Option<String>,
}

impl TreatmentAreas {
    /// The areas as this build of the tool names them; `run_manifest` is the repository path of
    /// the manifest being run, when it lies in the checkout.
    pub(crate) fn current(run_manifest: Option<&str>) -> TreatmentAreas {
        TreatmentAreas {
            manifests: manifests_folder(),
            archive: ARCHIVE_DIR.to_string(),
            ticket_documents: TICKET_DOCUMENTS_DIR.to_string(),
            tickets: TICKETS_DIR.to_string(),
            run_manifest: run_manifest.map(str::to_string),
        }
    }

    /// The same areas where `mapping`'s moves put them, for judging a checkout the moves have
    /// already reshaped.
    pub(crate) fn relocated(&self, mapping: &PathMapping) -> TreatmentAreas {
        let moved = |path: &str| mapping.relocate(path).into_owned();
        TreatmentAreas {
            manifests: moved(&self.manifests),
            archive: moved(&self.archive),
            ticket_documents: moved(&self.ticket_documents),
            tickets: moved(&self.tickets),
            run_manifest: self.run_manifest.as_deref().map(moved),
        }
    }

    /// The treatment of `path` with content `text`.
    pub(crate) fn treatment_of(&self, path: &str, text: &str) -> FileTreatment {
        if is_at_or_below(path, &self.manifests) || self.run_manifest.as_deref() == Some(path) {
            return FileTreatment::Excluded;
        }
        if is_at_or_below(path, &self.archive) || is_at_or_below(path, &self.ticket_documents) {
            return if is_markdown(path) {
                FileTreatment::FrozenDocument
            } else {
                FileTreatment::Excluded
            };
        }
        if self.is_ticket_file(path) && ticket_is_closed(text) {
            return FileTreatment::ClosedTicket;
        }
        FileTreatment::Live
    }

    /// Whether `path` is a ticket record, `T-<id>.toml` directly in the ticket folder.
    fn is_ticket_file(&self, path: &str) -> bool {
        let name = path.rsplit('/').next().unwrap_or(path);
        parent_folder(path) == self.tickets && name.starts_with("T-") && name.ends_with(".toml")
    }
}

/// Whether `path` names a Markdown file.
pub(crate) fn is_markdown(path: &str) -> bool {
    path.rsplit_once('.')
        .is_some_and(|(_, extension)| extension.eq_ignore_ascii_case("md"))
}

/// Whether the ticket's top-level `status` is shipped or cancelled.
pub(crate) fn ticket_is_closed(text: &str) -> bool {
    top_level_value(text, "status")
        .and_then(StatusName::parse)
        .is_some_and(|status| matches!(status, StatusName::Shipped | StatusName::Cancelled))
}

/// The byte spans of the ticket's top-level `spec` and `plan` lines.
pub(crate) fn checked_ticket_field_lines(text: &str) -> Vec<Range<usize>> {
    top_level_assignments(text)
        .into_iter()
        .filter(|assignment| CHECKED_TICKET_FIELDS.contains(&assignment.key))
        .map(|assignment| assignment.line)
        .collect()
}

/// The string value of a top-level `key = "value"` line.
fn top_level_value<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    top_level_assignments(text)
        .into_iter()
        .find(|assignment| assignment.key == key)
        .map(|assignment| assignment.value.trim().trim_matches('"'))
}

/// One `key = value` line of a TOML document's root table.
struct TopLevelAssignment<'a> {
    key: &'a str,
    value: &'a str,
    line: Range<usize>,
}

/// The root table's `key = value` lines: every line before the first table header that starts an
/// assignment, skipping the bodies of multi-line strings.
fn top_level_assignments(text: &str) -> Vec<TopLevelAssignment<'_>> {
    let mut found = Vec::new();
    let mut offset = 0;
    let mut inside_multiline_string = false;
    for line in text.split_inclusive('\n') {
        let span = offset..offset + line.len();
        offset += line.len();
        let opens_or_closes =
            (line.matches("\"\"\"").count() + line.matches("'''").count()) % 2 == 1;
        let was_inside = inside_multiline_string;
        if opens_or_closes {
            inside_multiline_string = !inside_multiline_string;
        }
        if was_inside {
            continue;
        }
        let trimmed = line.trim_start();
        if trimmed.starts_with('[') {
            break;
        }
        if let Some((key, value)) = trimmed.split_once('=') {
            found.push(TopLevelAssignment {
                key: key.trim(),
                value: value.trim_end(),
                line: span,
            });
        }
    }
    found
}
