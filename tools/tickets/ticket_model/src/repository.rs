//! The repository locations only the ticket domain names, spelled once each.
//!
//! **Role:** the handoff document of a slice ([`handoff_doc`]), the sparse-checkout set of each
//! ticket target ([`SPARSE_CHECKOUT_SETS`]), and, in [`documentation`], the documents a ticket
//! cites, a scan walks or skips, or only historical commits carry.
//! **Position:** the ticket crates' layout module, beside [`repository_layout`], which holds the
//! locations the tools share (the registry files, the artifact tree, the documentation root, the
//! two documents `ticket sync` rewrites) and finds the checkout root these paths join onto
//! ([`repository_layout::find_repository_root`]).
//! **Signals & state:** none; constants and pure functions.
//! **Invariants:** each path is spelled once, relative to the checkout root; relocating the
//! documentation tree rewrites [`documentation`] and nothing else in the crate, while the paths
//! that only historical commits carry keep the spelling those commits were made with.

use repository_layout::documentation::DOCUMENTATION_ROOT;
use repository_layout::{ARTIFACTS_DIR, TICKETS_DIR};

/// The handoff document an executing agent writes for one ticket or slice.
pub fn handoff_doc(slug: &str) -> String {
    format!("{ARTIFACTS_DIR}/{slug}_claude_code_handoff.md")
}

/// The directories and files a sparse checkout needs for each ticket target, keyed by the target
/// word a ticket's `[scope]` names.
///
/// `root` carries the whole tooling tree because `cargo xtask` is the task surface: a root slice
/// that cannot build xtask cannot run a single gate. `.cargo` rides with it for the command
/// aliases that make `cargo xtask` resolve at all.
///
/// A `website` slice carries the three website applications, everything they depend on through
/// `path =` dependencies (the shared crates under `crates/`), the deployment folder that builds
/// and serves them, and every `contracts/` file those crates compile in through `include_str!`
/// or `include_bytes!`: the JSON Schemas (which the API's typed models are also generated from),
/// the API golden responses, the ballistics
/// catalog and calibration fixtures, the mission and registry fixtures, the equipment matching
/// rules and the kit aliases. A `website` or `mod` slice checks out its documentation mirror
/// beside the code, because documentation ships with the code it describes.
pub const SPARSE_CHECKOUT_SETS: &[(&str, &[&str])] = &[
    (
        "website",
        &[
            "apps/api",
            "apps/frontend",
            "apps/offline_service_worker",
            "crates",
            "deploy",
            "contracts/definitions",
            "contracts/fixtures/api_goldens",
            "contracts/fixtures/ballistics",
            "contracts/fixtures/missions",
            "contracts/fixtures/registry",
            "contracts/catalogs/ballistics",
            "contracts/rules/equipment-gameplay",
            "contracts/rules/kit-aliases.json",
            documentation::APPS_DOCUMENTATION_DIR,
            documentation::CRATES_DOCUMENTATION_DIR,
        ],
    ),
    ("mod", &["apps/mod", documentation::MOD_DOCUMENTATION_DIR]),
    ("shared", &["contracts"]),
    (
        "root",
        &[
            TICKETS_DIR,
            ARTIFACTS_DIR,
            DOCUMENTATION_ROOT,
            "tools",
            ".cargo",
            "README.md",
            "CLAUDE.md",
        ],
    ),
];

/// The documents the ticket domain names.
///
/// Everything here is a document: a specification a ticket cites, a plan it must carry, the plan
/// skeleton, a document `ticket sync` writes into, a tree a scan walks or skips, or a path prefix
/// that only historical commits carry. Relocating the documentation tree rewrites exactly this
/// module and nothing else in the crate.
pub mod documentation {
    use repository_layout::QUEUE_JSON;
    use repository_layout::documentation::DOCUMENTATION_ROOT;

    /// The applications' documentation, which mirrors `apps/` (the game mod's documentation sits
    /// at [`MOD_DOCUMENTATION_DIR`]). A `website` slice checks it out beside the code.
    pub const APPS_DOCUMENTATION_DIR: &str = "documentation/apps";

    /// The library crates' documentation, which mirrors `crates/`. A `website` slice checks it out
    /// beside the code, because the website's applications depend on those crates.
    pub const CRATES_DOCUMENTATION_DIR: &str = "documentation/crates";

    /// The game mod's documentation, which mirrors `apps/mod`. A `mod` slice checks it out beside
    /// the code.
    pub const MOD_DOCUMENTATION_DIR: &str = "documentation/mod";

    /// One four-section plan document per ticket, named by [`plan_path`].
    pub const PLANS_DIR: &str = "documentation/tickets/plans";

    /// The plan skeleton a ticket copies when it goes ready without one. It sits with the other
    /// ticket templates in the registry folder, not in the documentation tree.
    pub const PLAN_TEMPLATE: &str = ".ai/tickets/plan_template.md";

    /// A ticket's own plan document: lowercase id, dots to underscores. `mark-ready` defaults an
    /// unset `plan` field to this path, and refuses while the file is absent.
    pub fn plan_path(id: &crate::TicketId) -> String {
        format!(
            "{PLANS_DIR}/{}_plan.md",
            id.as_str().to_lowercase().replace('.', "_")
        )
    }

    /// Ticket specifications — the documents most ticket `spec` fields name. One flat folder
    /// inside [`DOCUMENTATION_ROOT`].
    pub const SPECS_DIR: &str = "documentation/tickets/specs";

    /// The document of record for the tokens-per-line-changed factor. A test asserts the document
    /// quotes the compiled constant verbatim, so the two can never drift.
    pub const TOKEN_ESTIMATE_FACTOR_DOC: &str =
        "documentation/tools/tickets/token_estimate_factor.md";

    /// Path prefix of the Markdown queue views that historical commits carry. No command writes
    /// a file under it; the token estimator's `git log --numstat` walk still meets these paths in
    /// old commits, so [`NUMSTAT_EXCLUDED_PREFIXES`] keeps their churn out of a ticket's
    /// changed-line count. Relocating the documentation tree does not move it: a historical
    /// commit keeps the spelling it was made with.
    pub const RETIRED_QUEUE_VIEW_PREFIX: &str = "docs/TICKET_";

    /// Trees the stale-identifier scan walks past, each matched as a path prefix or anywhere in
    /// the path. Each is either generated output, an imported design corpus, or a frozen record:
    /// none of them is prose an author maintains, so an identifier inside one is data rather than
    /// a stale reference. The archive and the ticket specifications and plans are frozen records
    /// that quote retired identifiers by design; every `visual_references/` folder and the
    /// design-token exports hold imported design exports.
    pub const SCAN_EXEMPT_PREFIXES: &[&str] = &[
        ".ai/artifacts/eden-wiki/",
        "frontend/src/stitch-exports/",
        "documentation/archive/",
        "documentation/tickets/",
        "/visual_references/",
        "documentation/design_system/token_exports/",
        ".stitch-backup-exports/",
    ];

    /// Where the stale-identifier scan looks. Files first, then directories walked in full.
    /// [`SPECS_DIR`] sits inside [`DOCUMENTATION_ROOT`] and under an exempt prefix, so no root
    /// names it.
    pub const STALE_TICKET_ID_SCAN_ROOTS: &[&str] =
        &[DOCUMENTATION_ROOT, QUEUE_JSON, "CLAUDE.md", "README.md"];

    /// Path prefixes the token estimator drops from a commit's changed-line count: the `.ai/`
    /// tree (the ticket registry and the agent artifact tree) and the retired queue views
    /// ([`RETIRED_QUEUE_VIEW_PREFIX`], matched on `.md` files only). `Cargo.lock` files are not
    /// prefixes: `ticket_metrics`'s `estimates::is_excluded_path` drops them by
    /// file name. Counting any of them would charge a ticket for the bookkeeping its own landing
    /// performs.
    pub const NUMSTAT_EXCLUDED_PREFIXES: &[&str] = &[".ai/", RETIRED_QUEUE_VIEW_PREFIX];

    /// The two hand-kept wave plans the wave lock replaced.
    ///
    /// They are read at historical revisions through `git show`, where they still stand, and
    /// deleted from a working tree exactly once by the migrating repack. Relocating the
    /// documentation tree does not move them: a historical revision keeps the spelling it was
    /// committed with.
    pub const ARCHIVED_WAVE_PLANS: [&str; 2] =
        ["docs/platform/wave_plan.tsv", "docs/mod/wave_plan.tsv"];

    /// Paths where naming an archived wave plan is a statement about the past rather than a live
    /// reference, each with the reason it is here. Keep this list tight: a document that presents
    /// the archived plans as current truth gets rewritten, not listed.
    pub const ARCHIVED_WAVE_PLAN_READERS: &[(&str, &str)] = &[
        (
            ".ai/artifacts/",
            "pipeline output — frozen run reports and verify logs",
        ),
        (
            ".ai/tickets/",
            "ticket notes and summaries narrate the plan era; owns cells may name deleted paths",
        ),
        (
            "documentation/archive/shipped_history/shipped_history.md",
            "the shipped-history archive describes past states in past commits",
        ),
        (
            "documentation/tickets/specs/t911_ticket_registry_redesign.md",
            "an approved design document, written while the plans lived",
        ),
        (
            "documentation/tickets/specs/t912_wave_lockfile.md",
            "the specification of the lock that replaced them names the files it deletes",
        ),
        (
            "documentation/archive/factory_runs/grok_wave_130_handoff.md",
            "a kickoff document for a finished wave — a snapshot, not a runbook",
        ),
        (
            "documentation/archive/factory_runs/wave_209_grok_kickoff.md",
            "a kickoff document for a finished wave — a snapshot, not a runbook",
        ),
        (
            "tools/tickets/ticket_model/src/repository.rs",
            "the module that spells every repository path the ticket domain touches, this pair \
             included",
        ),
        (
            "crates/api/api_database/migrations/0011_events_server_modpack.sql",
            "committed migrations are checksum-frozen; rewording a comment in one breaks every \
             checkout that already applied it",
        ),
    ];
}

#[cfg(test)]
#[path = "tests/repository_layout_tests.rs"]
mod tests;
