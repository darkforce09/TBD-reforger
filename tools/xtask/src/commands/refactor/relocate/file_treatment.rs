//! Which rewrites a tracked file may receive: live, frozen, a closed ticket, or none.
//!
//! **Role:** sorts every file into a [`FileTreatment`]. Frozen Markdown records (the ticket
//! documents and the archive) keep their prose and backticks as history and receive only link
//! destination rewrites; a closed ticket file receives rewrites on its `spec`, `plan` and `owns`
//! values only, the fields the ticket engine resolves against the tree (`spec` and `plan` must
//! exist, `owns` must match the wave lock); the relocation manifests (the
//! `.tsv` files of the manifests folder, whose `from` columns name retired paths on purpose), the
//! manifest being run, every SQL migration (a `.sql` file directly in a `migrations` folder;
//! `sqlx` refuses to boot on an applied migration whose checksum changed) and every other file in
//! a frozen area receive nothing. The rest of the manifests folder, such as its README, is live,
//! and so is every other `.sql` file and every other file of a `migrations` folder.
//!
//! **Position:** consulted by the plan builder ([`super::relocation_plan`]) before any pass runs
//! and by the verification ([`super::retired_spellings`]), both with the areas where the
//! manifest's moves put them ([`TreatmentAreas::relocated`]) and each file at its path after the
//! moves, so both judge the same files the same way.
//!
//! **Signals & state:** none; pure functions over a path and its text.
//!
//! **Invariants:** the frozen areas are the ones [`crate::core::repository_layout::documentation`]
//! names; a file's treatment follows its destination, so a file a move puts into a frozen area is
//! frozen and one a move takes out of it is live, and the areas are found whether this build
//! spells them as they lie before or after the moves; a ticket is closed exactly when its
//! top-level `status` is one the ticket engine's [`ticket_engine::StatusName`] calls shipped or
//! cancelled; an unknown status counts as open, so its paths keep resolving.

use std::ops::Range;

use ticket_engine::StatusName;

use super::path_mapping::{PathMapping, is_at_or_below, parent_folder};
use crate::core::repository_layout::documentation::{ARCHIVE_DIR, TICKET_DOCUMENTS_DIR};
use repository_layout::TICKETS_DIR;
use repository_layout::documentation::DOCUMENTATION_ROOT;

/// The relocation manifests' folder below the documentation root.
const MANIFESTS_BELOW_DOCUMENTATION: &str = "restructure/manifests";

/// The extension of a relocation manifest.
const MANIFEST_EXTENSION: &str = ".tsv";

/// The folder name `sqlx` reads a crate's migrations from.
const MIGRATIONS_FOLDER_NAME: &str = "migrations";

/// The extension of a SQL migration.
const MIGRATION_EXTENSION: &str = ".sql";

/// The top-level ticket fields the ticket engine resolves against the tree.
const CHECKED_TICKET_FIELDS: [&str; 3] = ["spec", "plan", "owns"];

/// How much of a file the rewrite passes may change.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FileTreatment {
    /// Every rewrite applies.
    Live,
    /// A frozen Markdown record: link destinations only.
    FrozenDocument,
    /// A closed ticket: its `spec`, `plan` and `owns` values only.
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

    /// The same areas where `mapping`'s moves put them: the areas every file is judged against at
    /// its path after the moves, by the plan builder and by the verification.
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
        let is_manifest =
            is_at_or_below(path, &self.manifests) && path.ends_with(MANIFEST_EXTENSION);
        if is_manifest || self.run_manifest.as_deref() == Some(path) || is_sql_migration(path) {
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

/// Whether `path` is a SQL migration: a `.sql` file directly in a folder named `migrations`,
/// wherever that folder lies. `sqlx` records the checksum of every applied migration and refuses
/// to start when a file changes, so a migration moves byte-identical and keeps the spellings it
/// was applied with.
fn is_sql_migration(path: &str) -> bool {
    path.ends_with(MIGRATION_EXTENSION)
        && parent_folder(path).rsplit('/').next() == Some(MIGRATIONS_FOLDER_NAME)
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

/// The byte spans of the ticket's top-level `spec`, `plan` and `owns` assignments, a multi-line
/// array through its closing line.
pub(crate) fn checked_ticket_field_lines(text: &str) -> Vec<Range<usize>> {
    top_level_assignments(text)
        .into_iter()
        .filter(|assignment| CHECKED_TICKET_FIELDS.contains(&assignment.key))
        .map(|assignment| assignment.lines)
        .collect()
}

/// The string value of a top-level `key = "value"` line.
fn top_level_value<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    top_level_assignments(text)
        .into_iter()
        .find(|assignment| assignment.key == key)
        .map(|assignment| assignment.value.trim().trim_matches('"'))
}

/// One `key = value` assignment of a TOML document's root table.
struct TopLevelAssignment<'a> {
    key: &'a str,
    /// The value as written on the key's line.
    value: &'a str,
    /// The assignment's lines: the key's line, through the line that closes a multi-line array.
    lines: Range<usize>,
}

/// The root table's `key = value` assignments: every line before the first table header that
/// starts an assignment, skipping the bodies of multi-line strings; an array value left open on its
/// key's line extends the assignment through the line that closes it.
fn top_level_assignments(text: &str) -> Vec<TopLevelAssignment<'_>> {
    let mut found: Vec<TopLevelAssignment<'_>> = Vec::new();
    let mut offset = 0;
    let mut inside_multiline_string = false;
    let mut open_brackets = 0usize;
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
        if open_brackets > 0 {
            open_brackets = bracket_balance(open_brackets, line);
            if let Some(assignment) = found.last_mut() {
                assignment.lines.end = span.end;
            }
            continue;
        }
        let trimmed = line.trim_start();
        if trimmed.starts_with('[') {
            break;
        }
        if let Some((key, value)) = trimmed.split_once('=') {
            if value.trim_start().starts_with('[') {
                open_brackets = bracket_balance(0, value);
            }
            found.push(TopLevelAssignment {
                key: key.trim(),
                value: value.trim_end(),
                lines: span,
            });
        }
    }
    found
}

/// The count of `[` still open after `text`, starting from `open`; brackets inside double-quoted
/// strings do not count.
fn bracket_balance(mut open: usize, text: &str) -> usize {
    let mut inside_string = false;
    let mut escaped = false;
    for character in text.chars() {
        match character {
            _ if escaped => escaped = false,
            '\\' if inside_string => escaped = true,
            '"' => inside_string = !inside_string,
            '[' if !inside_string => open += 1,
            ']' if !inside_string => open = open.saturating_sub(1),
            _ => {}
        }
    }
    open
}
