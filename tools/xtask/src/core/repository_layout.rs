//! Locations inside a checkout that xtask commands read, written once each.
//!
//! These are repository-relative paths. A caller joins one onto the checkout root it already
//! holds — [`repository_layout::find_repository_root`] for a running command, the scratch
//! root a test builds. Spelling each location once means a relocation is one edit here plus the
//! moves, and a command that reads a file can be traced to the file without a search.

/// Deployment configuration and unit templates for the website and game servers.
pub const DEPLOY_DIR: &str = "deploy";

/// Host secrets and remote paths every `cargo xtask deploy` subcommand loads. Gitignored: it
/// holds credentials, and the deploy excludes it from the rsync so a dev PC cannot overwrite the
/// server's copy.
pub const DEPLOY_ENV: &str = "deploy/deploy.env";

/// The committed template an operator copies to [`DEPLOY_ENV`] and fills in.
pub const DEPLOY_ENV_EXAMPLE: &str = "deploy/deploy.env.example";

/// Caddy reverse proxy serving the SPA and proxying `/api` to the API port. The staging compose
/// file's `caddy` service runs it, and every `cargo xtask deploy website` starts that service and
/// reloads the file; `forwarded_for_trust` pins its loopback upstream.
pub const CADDYFILE: &str = "deploy/caddy/Caddyfile";

/// The local development stack: the Postgres container `tbd_reforger_db` on host port 5434.
/// `cargo xtask db up`, `down`, `logs` and `seed` pass it to compose with `-f` and run compose in
/// its folder, which is the folder its relative paths resolve against.
pub const DEVELOPMENT_COMPOSE_FILE: &str = "deploy/compose.dev.yml";

/// systemd unit templates an operator installs into `~/.config/systemd/user`.
pub const SYSTEMD_UNITS_DIR: &str = "deploy/systemd";

/// The website API unit. `cargo xtask deploy website` renders this template's repository
/// placeholder for the remote and restarts the installed unit by name.
pub const WEBSITE_API_UNIT: &str = "deploy/systemd/tbd-website-api.service";

/// Dedicated-server configuration profiles the mod commands launch a server with.
pub const DEDICATED_SERVER_PROFILES_DIR: &str = "tools/xtask/dedicated_server_profiles";

/// The development server profile `cargo xtask mod playtest` and `cargo xtask mod world-boot`
/// load: local addons, one scenario, headless ports.
pub const DEV_SERVER_PROFILE: &str =
    "tools/xtask/dedicated_server_profiles/tbd-dev-server.config.json";

/// Recorded MCP server transcripts. `cargo xtask mcp selftest` replays them through
/// `cargo xtask mcp consume` to pin the exit code of every response shape without a Workbench.
pub const MCP_TRANSCRIPT_FIXTURES_DIR: &str = "tools/xtask/fixtures/mcp";

// The reference folder and its Coalition Reforger Framework and vanilla lanes are shared with
// `developer_tools` and spelled once, in `repository_layout`; `cargo xtask verify no-crf-leak`
// checks that none of them reaches the shipped addons.

/// The PlayableSelector checkout, which carries no licence: design mirror only.
pub const PLAYABLE_SELECTOR_REFERENCE: &str = "apps/mod/References/playable_selector";

/// An environment variable naming another PlayableSelector checkout. When it is set and not
/// empty it replaces [`PLAYABLE_SELECTOR_REFERENCE`] as the lane's source.
pub const PLAYABLE_SELECTOR_OVERRIDE_ENV: &str = "TBD_PS_ORACLE";

/// The locations the workspace laws (`cargo xtask verify crate-tiers` and its siblings) read.
///
/// The laws themselves live in `repository_laws::workspace_laws` and know no
/// path that moves with the tree; a stage that moves a folder rewrites these constants.
pub mod workspace_laws {
    use repository_laws::workspace_laws::frontend_layering::{
        FrontendCrateLayers, FrontendLayer, FrontendLayerRow, SubAreaOrder,
    };

    /// Folders whose every `Cargo.toml` (outside test trees, fixtures and build output) must be a
    /// workspace member. A folder that does not exist yet holds no manifest.
    pub const MANIFEST_SWEEP_ROOTS: &[&str] = &["apps", "crates", "tools", "legacy"];

    /// The app stylesheet whose `@source` lines must cover every leptos crate.
    pub const TAILWIND_STYLESHEET: &str = "apps/frontend/style/aegis.css";

    /// The frontend crate's layer table: `src/foundation` is the foundation, `src/features` the
    /// shared features, `src/pages` the pages (one area per section), `src/workspaces` the
    /// workspaces (one area per workspace), and the entry point, the render form of the route
    /// table, the platform frame and the crate-level tests the shell. Inside the foundation the
    /// sub-areas follow [`FOUNDATION_SUB_AREA_ORDER`].
    pub const FRONTEND_LAYERS: &[FrontendCrateLayers] = &[FrontendCrateLayers {
        crate_path: "apps/frontend",
        rows: &[
            shell("src/main.rs"),
            shell("src/app_routes.rs"),
            shell("src/tests"),
            shell("src/shell"),
            row("src/foundation", FrontendLayer::Foundation, false),
            row("src/features", FrontendLayer::Features, false),
            row("src/pages", FrontendLayer::Pages, true),
            row("src/workspaces", FrontendLayer::Workspaces, true),
        ],
        sub_area_orders: &[FOUNDATION_SUB_AREA_ORDER],
    }];

    /// The strict order of the foundation's sub-areas, lowest first: `ui` < `utils` <
    /// `transport` < `route_table` < `auth` < {`offline`, `map_view`}. A sub-area imports only
    /// sub-areas strictly before it; `offline` and `map_view` are peers that never import each
    /// other; `test_support` sits outside the order and only test files import it.
    pub const FOUNDATION_SUB_AREA_ORDER: SubAreaOrder = SubAreaOrder {
        parent: "src/foundation",
        tiers: &[
            &["ui"],
            &["utils"],
            &["transport"],
            &["route_table"],
            &["auth"],
            &["offline", "map_view"],
        ],
        test_only: &["test_support"],
    };

    const fn shell(path: &'static str) -> FrontendLayerRow {
        row(path, FrontendLayer::Shell, false)
    }

    const fn row(path: &'static str, layer: FrontendLayer, has_areas: bool) -> FrontendLayerRow {
        FrontendLayerRow {
            path,
            layer,
            has_areas,
        }
    }
}

// The registry, the wave lock, the artifact tree and the root marker are shared with the other
// tools and spelled once, in `repository_layout`.

/// Documents xtask reads, walks or names in what it prints.
///
/// Relocating the documentation tree rewrites this module and nothing else in the crate; a
/// runbook that moves is one edit here rather than a hunt through help text and refusals.
pub mod documentation {
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
    pub const API_READINESS_EVIDENCE_PREFIX: &str = "documentation/apps/api/verification_evidence/";

    /// The API acceptance register: every requirement, the implementation paths it rests on and
    /// the checks that prove it. `cargo xtask verify api-readiness` reads and validates it before
    /// it judges any evidence. It sits under [`API_READINESS_EVIDENCE_PREFIX`], so the source
    /// fingerprint covers it.
    pub const API_READINESS_REGISTER: &str =
        "documentation/apps/api/verification_evidence/requirements.json";

    // The documentation tree root, the artifact tree and the two documents `cargo xtask ticket
    // sync` rewrites are shared with the other tools and spelled once, in `repository_layout`.

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

    /// A documentation root that must not exist: every document lives under
    /// [`repository_layout::documentation::DOCUMENTATION_ROOT`], and
    /// `cargo xtask verify markdown-placement` fails while this folder holds a tracked file. `cargo xtask verify link-check` reads a backticked path under it as a
    /// repository path whether or not the folder still holds files, so a live document that names
    /// the retired tree breaks.
    pub const RETIRED_DOCS_ROOT: &str = "docs";

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
}

#[cfg(test)]
#[path = "../tests/repository_layout_tests.rs"]
mod tests;
