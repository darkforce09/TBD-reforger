//! Which rewrites a tracked file may receive: live, frozen, a frozen area's README index, this
//! tool's own test sources, or none.
//!
//! **Role:** sorts every file into a [`FileTreatment`]. Frozen Markdown records (the ticket
//! documents and the archive) keep their prose and backticks as history and receive only link
//! destination rewrites; a `README.md` there is a live index of its folder, so the tree part of
//! its Contents block's lines (the root folder and each entry's name, never the roles) receives
//! every rewrite a live document receives as well; the legacy ticket data folder
//! ([`repository_layout::LEGACY_TICKETS_DIR`], records kept only until they are imported into the
//! central ticket manager) receives nothing; this tool's own test sources spell paths of
//! throwaway checkouts in their string literals and comments, so only their code is rewritten; the relocation manifests (the `.tsv` files of the manifests folder, whose
//! `from` columns name retired paths on purpose), the manifest being run, every SQL migration (a
//! `.sql` file directly in a `migrations` folder; `sqlx` refuses to boot on an applied migration
//! whose checksum changed) and every other file in a frozen area receive nothing. The rest of the
//! manifests folder, such as its README, is live, and so is every other `.sql` file and every other
//! file of a `migrations` folder.
//!
//! **Position:** consulted by the plan builder ([`super::relocation_plan`]) before any pass runs
//! and by the verification ([`super::retired_spellings`]), both with the areas where the
//! manifest's moves put them ([`TreatmentAreas::relocated`]) and each file at its path after the
//! moves, so both judge the same files the same way.
//!
//! **Signals & state:** none; pure functions over a path.
//!
//! **Invariants:** the frozen areas are the ones [`repository_layout`]
//! names; a file's treatment follows its destination, so a file a move puts into a frozen area is
//! frozen and one a move takes out of it is live, and the areas are found whether this build
//! spells them as they lie before or after the moves.

use super::path_mapping::{PathMapping, is_at_or_below, parent_folder};
use repository_layout::documentation::DOCUMENTATION_ROOT;
use repository_layout::{ARCHIVE_DIR, LEGACY_TICKETS_DIR, TICKET_DOCUMENTS_DIR};

/// The relocation manifests' folder below the documentation root.
const MANIFESTS_BELOW_DOCUMENTATION: &str = "relocation_manifests";

/// This tool's own test sources: their string literals and comments describe throwaway checkouts,
/// so a manifest that moves a real path of the same spelling never rewrites or judges them. The
/// spelling is a repository path, so a manifest that moves this crate rewrites it with the move.
const RELOCATION_TEST_SOURCES: &str = "tools/commands/repository_relocation/src/tests";

/// The file name of a folder's index document.
const README_NAME: &str = "README.md";

/// The extension of a relocation manifest.
const MANIFEST_EXTENSION: &str = ".tsv";

/// The folder name `sqlx` reads a crate's migrations from.
const MIGRATIONS_FOLDER_NAME: &str = "migrations";

/// The extension of a SQL migration.
const MIGRATION_EXTENSION: &str = ".sql";

/// How much of a file the rewrite passes may change.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FileTreatment {
    /// Every rewrite applies.
    Live,
    /// A frozen Markdown record: link destinations only.
    FrozenDocument,
    /// A frozen area's `README.md`, a live index of its folder: link destinations and the tree
    /// part of its Contents block's lines.
    FrozenIndex,
    /// This tool's own test sources: code only, never a string literal or a comment.
    FixtureSource,
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
    legacy_tickets: String,
    fixture_sources: String,
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
            legacy_tickets: LEGACY_TICKETS_DIR.to_string(),
            fixture_sources: RELOCATION_TEST_SOURCES.to_string(),
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
            legacy_tickets: moved(&self.legacy_tickets),
            fixture_sources: moved(&self.fixture_sources),
            run_manifest: self.run_manifest.as_deref().map(moved),
        }
    }

    /// The treatment of `path`.
    pub(crate) fn treatment_of(&self, path: &str) -> FileTreatment {
        let is_manifest =
            is_at_or_below(path, &self.manifests) && path.ends_with(MANIFEST_EXTENSION);
        if is_manifest
            || self.run_manifest.as_deref() == Some(path)
            || is_sql_migration(path)
            || is_at_or_below(path, &self.legacy_tickets)
        {
            return FileTreatment::Excluded;
        }
        if is_at_or_below(path, &self.archive) || is_at_or_below(path, &self.ticket_documents) {
            return if path.rsplit('/').next() == Some(README_NAME) {
                FileTreatment::FrozenIndex
            } else if is_markdown(path) {
                FileTreatment::FrozenDocument
            } else {
                FileTreatment::Excluded
            };
        }
        if is_at_or_below(path, &self.fixture_sources) {
            return FileTreatment::FixtureSource;
        }
        FileTreatment::Live
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
