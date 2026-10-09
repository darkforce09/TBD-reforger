//! The documents and documentation areas the commands read, walk or name in what they print.
//!
//! **Role:** the runbooks a refusal cites, the factory's pack marker, the API readiness register
//! and the tree its fingerprint covers, and the roots, frozen areas and exemptions of the
//! documentation gates, each spelled once.
//! **Position:** read by the `xtask` command groups and verifications (deploy, setup, platform,
//! mod, the API readiness, licensing and documentation checks, the relocation tool). Relocating
//! the documentation tree rewrites this module, not the help texts and refusals that name a
//! document. The tree root and the two documents `ticket sync` rewrites are in
//! [`crate::documentation`].
//! **Signals & state:** none; constants.
//! **Invariants:** every item is classified as a location a checkout holds or as an exemption with
//! its reason (`tests/command_locations_tests.rs`); the areas lie under
//! [`crate::documentation::DOCUMENTATION_ROOT`].

/// A one-line marker an operator drops in while the factory packs a wave, so the wave gate
/// can report which wave is being packed without being told.
pub const FACTORY_PACK_WAVE: &str = ".ai/factory_pack_wave";

/// Installing and operating the website host: units, Caddy, backups.
pub const HOME_SERVER_RUNBOOK: &str = "documentation/runbooks/website_deployment.md";

/// Standing up and operating the dedicated game server.
pub const STAGING_SERVER_RUNBOOK: &str = "documentation/runbooks/game_server_staging/README.md";

/// The slice worktree lifecycle the mod wave driver automates.
pub const SLICE_WORKFLOW_RUNBOOK: &str = "documentation/runbooks/mod_slice_workflow.md";

/// The platform wave lifecycle `cargo xtask platform wave` automates.
pub const PLATFORM_FACTORY_RUNBOOK: &str = "documentation/runbooks/factory_waves/README.md";

/// The mod's design authority, including the upstream-code oracle lanes.
pub const MOD_DESIGN: &str = "documentation/mod/tbd-framework/mod_design.md";

/// How to run the spawn-determinism gate, which needs a live Workbench.
pub const SPAWN_DETERMINISM_RUNBOOK: &str = "documentation/runbooks/spawn_determinism.md";

/// The API readiness tree: the acceptance register and the design notes beside it.
/// `cargo xtask verify api-readiness` fingerprints every source file under it, so an edit
/// here invalidates recorded evidence. The fingerprint matches this prefix with
/// `starts_with`; the trailing slash keeps a sibling whose name merely begins the same way
/// out of the inputs.
pub const API_READINESS_EVIDENCE_PREFIX: &str =
    "documentation/crates/api/api_server/verification_evidence/";

/// The API acceptance register: every requirement, the implementation paths it rests on and
/// the checks that prove it. `cargo xtask verify api-readiness` reads and validates it before
/// it judges any evidence. It sits under [`API_READINESS_EVIDENCE_PREFIX`], so the source
/// fingerprint covers it.
pub const API_READINESS_REGISTER: &str =
    "documentation/crates/api/api_server/verification_evidence/requirements.json";

/// Archived documents, one folder per topic. Frozen: never reworded, and exempt from the size
/// limit.
pub const ARCHIVE_DIR: &str = "documentation/archive";

/// Ticket specifications and plans, the records the ticket registry cites. Frozen, and exempt
/// from the size limit.
pub const TICKET_DOCUMENTS_DIR: &str = "documentation/tickets";

/// Source documents waiting to be merged into live documents, one folder per writer. The
/// documentation gates skip it, and it is absent whenever no merge is pending.
pub const PENDING_MERGE_DIR: &str = "documentation/pending_merge";

/// The Cursor rule folders: agent instructions that name documents and commands. The
/// repository holds one `.cursor` folder, at its root, so the list has one entry.
/// `cargo xtask verify link-check` judges the links of their Markdown and `.mdc` files.
pub const CURSOR_RULE_DIRS: &[&str] = &[".cursor/rules"];

/// The project instructions at the repository root: the laws, the directory atlas and the
/// canonical commands every agent reads first. `cargo xtask verify link-check` judges its
/// links.
pub const PROJECT_INSTRUCTIONS: &str = "CLAUDE.md";

/// The top-level folders the repository retired, each with where its contents live now: `docs`,
/// whose documents live under [`crate::documentation::DOCUMENTATION_ROOT`], and `apps`, whose
/// Enfusion mod lives under [`crate::workspace_folders::ENFUSION_MOD_DIR`] and whose applications
/// are crates. None may exist: `cargo xtask verify markdown-placement` fails while one holds a
/// tracked file, and the README and code-tree rules leave them to that verdict. `cargo xtask
/// verify link-check` reads a backticked path under one as a repository path whether or not the
/// folder still holds files, so a live document that names a retired tree breaks.
pub const RETIRED_TOP_LEVEL_FOLDERS: &[(&str, &str)] = &[
    ("docs", "every document lives under documentation/"),
    (
        "apps",
        "the Enfusion mod lives under mod/ and every application is a crate",
    ),
];

/// Whether `folder` is the name of one of the [`RETIRED_TOP_LEVEL_FOLDERS`].
pub fn is_retired_top_level_folder(folder: &str) -> bool {
    RETIRED_TOP_LEVEL_FOLDERS
        .iter()
        .any(|(retired, _)| *retired == folder)
}

/// Repository paths a live document names on purpose although nothing is tracked or ignored
/// there, each with the reason it is named. `cargo xtask verify link-check` passes a
/// backticked path listed here; every other backticked repository path in a live document
/// must name a tracked file, a folder that holds one, or a path git ignores.
pub const HISTORICAL_PATH_SPELLINGS: &[(&str, &str)] = &[];

/// The prefix of a GitHub permalink into this repository: the commit and the repository-
/// relative path follow it, as `<prefix><commit>/<path>`. `cargo xtask verify link-check`
/// looks the object of every blob or tree view pinned to a full commit up in the local
/// history, and refuses a blob or tree view of a branch, a tag or an abbreviated commit.
pub const PERMALINK_BASE: &str = "https://github.com/darkforce09/TBD-reforger/blob/";

#[cfg(test)]
#[path = "tests/command_locations_tests.rs"]
mod tests;
