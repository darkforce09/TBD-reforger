//! Locations inside a checkout that xtask commands read, written once each.
//!
//! These are repository-relative paths. A caller joins one onto the checkout root it already
//! holds — [`crate::core::repository_root::find_repo_root`] for a running command, the scratch
//! root a test builds. Spelling each location once means a relocation is one edit here plus the
//! moves, and a command that reads a file can be traced to the file without a search.

/// Deployment configuration and unit templates for the website and game servers.
pub const DEPLOY_DIR: &str = "tools_v2/xtask/deploy";

/// Host secrets and remote paths every `cargo xtask deploy` subcommand loads. Gitignored: it
/// holds credentials, and the deploy excludes it from the rsync so a dev PC cannot overwrite the
/// server's copy.
pub const DEPLOY_ENV: &str = "tools_v2/xtask/deploy/deploy.env";

/// The committed template an operator copies to [`DEPLOY_ENV`] and fills in.
pub const DEPLOY_ENV_EXAMPLE: &str = "tools_v2/xtask/deploy/deploy.env.example";

/// Caddy reverse proxy serving the SPA and proxying `/api` to the API port. `cargo xtask deploy
/// website` names it in the reload instruction it prints; `forwarded_for_trust` pins its
/// loopback upstream.
pub const CADDYFILE: &str = "tools_v2/xtask/deploy/Caddyfile.website";

/// systemd unit templates an operator installs into `~/.config/systemd/user`.
pub const SYSTEMD_UNITS_DIR: &str = "tools_v2/xtask/deploy/systemd";

/// The website API unit. `cargo xtask deploy website` renders this template's repository
/// placeholder for the remote and restarts the installed unit by name.
pub const WEBSITE_API_UNIT: &str = "tools_v2/xtask/deploy/systemd/tbd-website-api.service";

/// Dedicated-server configuration profiles the mod commands launch a server with.
pub const DEDICATED_SERVER_PROFILES_DIR: &str = "tools_v2/xtask/dedicated_server_profiles";

/// The development server profile `cargo xtask mod playtest` and `cargo xtask mod world-boot`
/// load: local addons, one scenario, headless ports.
pub const DEV_SERVER_PROFILE: &str =
    "tools_v2/xtask/dedicated_server_profiles/tbd-dev-server.config.json";

/// Recorded MCP server transcripts. `cargo xtask mcp selftest` replays them through
/// `cargo xtask mcp consume` to pin the exit code of every response shape without a Workbench.
pub const MCP_TRANSCRIPT_FIXTURES_DIR: &str = "tools_v2/xtask/fixtures/mcp";

/* ───────────────────────── the ticket domain's own locations ───────────────────────── */

// The registry, the wave lock and the artifact tree belong to the ticket domain, which spells
// them once in `ticket_engine::repository`. They are re-exported here so every xtask path
// resolves through this one module, without a second spelling of any of them existing.
#[allow(unused_imports)] // each is the one spelling of its path for the whole crate
pub use ticket_engine::repository::{
    LAST_VERIFIED_MARKER, ROOT_MARKER, TICKETS_DIR, VERDICTS_DIR, WAVE_LOCK, WORKTREES_DIR,
};

/// Documents xtask reads, walks or names in what it prints.
///
/// Relocating the documentation tree rewrites this module and nothing else in the crate; a
/// runbook that moves is one edit here rather than a hunt through help text and refusals.
pub mod documentation {
    /// A one-line marker an operator drops in while the factory packs a wave, so the wave gate
    /// can report which wave is being packed without being told.
    pub const FACTORY_PACK_WAVE: &str = "docs/platform/factory_pack_wave";

    /// Where documentation belongs, named by the refusal that fires when markdown is committed
    /// inside an application or asset tree instead.
    pub const LAYOUT_TARGET_DIR: &str = "docs/website/";

    /// Installing and operating the website host: units, Caddy, backups.
    pub const HOME_SERVER_RUNBOOK: &str = "docs/website/HOME_SERVER.md";

    /// Standing up and operating the dedicated game server.
    pub const STAGING_SERVER_RUNBOOK: &str = "docs/mod/STAGING-SERVER.md";

    /// The slice worktree lifecycle the mod wave driver automates.
    pub const SLICE_WORKFLOW_RUNBOOK: &str = "docs/mod/SLICE_WORKFLOW.md";

    /// The platform wave lifecycle `cargo xtask platform wave` automates.
    pub const PLATFORM_FACTORY_RUNBOOK: &str = "docs/platform/PLATFORM_FACTORY.md";

    /// The mod's design authority, including the upstream-code oracle lanes.
    pub const MOD_DESIGN: &str = "docs/mod/TBD_MOD_DESIGN.md";

    /// How to run the spawn-determinism gate, which needs a live Workbench.
    pub const SPAWN_DETERMINISM_RUNBOOK: &str = "docs/mod/SPAWN_DETERMINISM.md";

    /// The API readiness tree: the acceptance register and the design notes beside it.
    /// `cargo xtask verify api-readiness` fingerprints every source file under it, so an edit
    /// here invalidates recorded evidence. The fingerprint matches this prefix with
    /// `starts_with`; the trailing slash keeps a sibling whose name merely begins the same way
    /// out of the inputs.
    pub const API_READINESS_EVIDENCE_PREFIX: &str = "docs/verification/api_v2/";

    /// The API acceptance register: every requirement, the implementation paths it rests on and
    /// the checks that prove it. `cargo xtask verify api-readiness` reads and validates it before
    /// it judges any evidence. It sits under [`API_READINESS_EVIDENCE_PREFIX`], so the source
    /// fingerprint covers it.
    pub const API_READINESS_REGISTER: &str = "docs/verification/api_v2/requirements.json";

    /// The top-level folders that hold code. Every tracked folder in them carries a README.md,
    /// and README.md is the only Markdown they hold; `cargo xtask verify readme-coverage` and
    /// `cargo xtask verify markdown-placement` enforce both.
    pub const CODE_TREES: &[&str] = &["apps", "tools_v2", "contracts_v2", "assets_v2"];

    /// Root of the documentation tree: the deeper documents that code READMEs link to. Every
    /// tracked folder in it carries a README.md, and every live document in it stays at or under
    /// the size limit.
    pub const DOCUMENTATION_ROOT: &str = "documentation_v2";

    /// Archived documents, one folder per topic. Frozen: never reworded, and exempt from the size
    /// limit.
    pub const ARCHIVE_DIR: &str = "documentation_v2/archive";

    /// Ticket specifications and plans, the records the ticket registry cites. Frozen, and exempt
    /// from the size limit.
    pub const TICKET_DOCUMENTS_DIR: &str = "documentation_v2/tickets";

    /// Source documents waiting to be merged into live documents, one folder per writer. The
    /// documentation gates skip it, and it is absent whenever no merge is pending.
    pub const PENDING_MERGE_DIR: &str = "documentation_v2/pending_merge";

    /// The shared name prefix of the documentation program's own records at the documentation
    /// root: the `refactor_` files and the manifest folder beside them. They are working records
    /// rather than live documents, so the size limit skips them while they sit at the root.
    pub const PROGRAM_RECORDS_PREFIX: &str = "documentation_v2/refactor_";

    /// The Cursor rule folders: agent instructions that name documents and commands.
    /// `cargo xtask verify link-check` judges the links of their Markdown and `.mdc` files.
    pub const CURSOR_RULE_DIRS: &[&str] = &[".cursor/rules", "apps/mod/.cursor/rules"];

    /// The project instructions at the repository root: the laws, the directory atlas and the
    /// canonical commands every agent reads first. `cargo xtask verify link-check` judges its
    /// links.
    pub const PROJECT_INSTRUCTIONS: &str = "CLAUDE.md";

    /// A documentation root that must not exist: every document lives under
    /// [`DOCUMENTATION_ROOT`], and `cargo xtask verify markdown-placement` fails while this folder
    /// holds a tracked file.
    pub const RETIRED_DOCS_ROOT: &str = "docs";

    /// The prefix of a GitHub permalink into this repository: the commit and the repository-
    /// relative path follow it, as `<prefix><commit>/<path>`. `cargo xtask verify link-check`
    /// looks the object of every blob or tree view pinned to a full commit up in the local
    /// history, and refuses a blob or tree view of a branch, a tag or an abbreviated commit.
    pub const PERMALINK_BASE: &str = "https://github.com/darkforce09/TBD-reforger/blob/";

    // The agent artifact tree, spelled once in `ticket_engine::repository`: run reports, handoff
    // notes and research dumps rather than documentation, so `cargo xtask verify link-check`
    // judges none of its files, its README.md included.
    pub use ticket_engine::repository::ARTIFACTS_DIR;

    // The ticket domain's documentation locations, spelled once in
    // `ticket_engine::repository::documentation`: the tree root, the specification and plan
    // folders, and the two documents `cargo xtask ticket sync` rewrites between markers, whose
    // sync-managed tables stay in one file whatever their length.
    #[allow(unused_imports)] // each is the one spelling of its path for the whole crate
    pub use ticket_engine::repository::documentation::{
        GAP_ANALYSIS, PLANS_DIR, ROADMAP, SPECS_DIR, TREE_DIR,
    };
}

#[cfg(test)]
#[path = "../tests/repository_layout_tests.rs"]
mod tests;
