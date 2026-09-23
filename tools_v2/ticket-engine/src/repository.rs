//! Every repository location the ticket domain reads or writes, spelled once each.
//!
//! The ticket registry, its schemas, its receipts and the documents generated from it all live
//! outside this crate's own tree. A caller joins one of these repository-relative items onto a
//! checkout root — [`find_repo_root`] for a running command, the scratch root a test builds — so
//! that relocating any of them is one edit here plus the move itself, and so that reading a
//! module tells you which file it touches without a search.
//!
//! [`documentation`] holds the subset that lives under the documentation tree. Those are the
//! items a relocation of that tree rewrites, gathered in one place so the relocation is a single
//! edit rather than a survey.
//!
//! The crates that consume the ticket domain resolve these paths from here rather than declaring
//! their own: `xtask` and `ticketboard` both read the same registry, and two spellings of one
//! location is a way for them to disagree about where a file is.

use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};

/* ─────────────────────────────── the ticket registry ─────────────────────────────── */

/// The registry itself: one `T-<id>.toml` per ticket, parents and children alike, beside the
/// schemas and receipts that describe them.
pub const TICKETS_DIR: &str = ".ai/tickets";

/// The file whose presence marks a checkout root. Every root walk stops at it, so a worktree
/// under another checkout resolves to itself rather than to its parent.
pub const ROOT_MARKER: &str = ".ai/tickets/ROOT";

/// Draft 2020-12 schema every ticket file is validated against by `ticket check`.
pub const SCHEMA: &str = ".ai/tickets/schema.json";

/// The four-level domain → layer → component → surface word list a ticket's `[scope]` block is
/// resolved against at every corpus load.
pub const SCOPE_VOCAB: &str = ".ai/tickets/scope-vocab.toml";

/// The corpus facts no ticket file states: the ids that must never be minted, and which ticket
/// implements an editor gap row the ticket itself does not claim. Read by
/// [`crate::corpus_pins::load`].
pub const CORPUS_PINS: &str = ".ai/tickets/corpus-pins.toml";

/// The wave plan, compiled from the ticket files by `cargo xtask wave repack` — the ONE writer.
pub const WAVE_LOCK: &str = ".ai/tickets/wave.lock";

/// The dispatch queue `ticket sync` regenerates: batch size, concurrency, worktree base, and the
/// ready tickets in order.
pub const QUEUE_JSON: &str = ".ai/tickets/queue.json";

/// Run receipts, one `<ticket id>/` subtree each. Deliberately outside the ticket files and the
/// wave lock: parallel lands touch disjoint subtrees and never a shared file.
pub const METRICS_DIR: &str = ".ai/tickets/metrics";

/// The committed schema every run receipt must satisfy.
pub const METRICS_SCHEMA: &str = ".ai/tickets/metrics.schema.json";

/// Token estimates, one file per ticket — deliberately outside [`METRICS_DIR`], because an
/// estimate is written before the work and a receipt after it.
pub const ESTIMATES_DIR: &str = ".ai/tickets/estimates";

/// The committed schema every estimate file must satisfy.
pub const ESTIMATES_SCHEMA: &str = ".ai/tickets/estimates.schema.json";

/// Pipeline output: run reports, verify logs, handoff documents and the worktree base. Nothing
/// here is an input to a gate; everything is a record of a run.
pub const ARTIFACTS_DIR: &str = ".ai/artifacts";

/// Where a parallel ticket's git worktree is created, one directory per ticket id.
pub const WORKTREES_DIR: &str = ".ai/artifacts/worktrees";

/// Marker file holding the commit the last verifier examined, so the wave gate can report how
/// many commits of unverified debt stand behind the tip.
pub const LAST_VERIFIED_MARKER: &str = ".ai/artifacts/last-verified";

/// Recorded gate verdicts, one file per gate run.
pub const VERDICTS_DIR: &str = ".ai/artifacts/verdicts";

/// The handoff document an executing agent writes for one ticket or slice.
pub fn handoff_doc(slug: &str) -> String {
    format!("{ARTIFACTS_DIR}/{slug}_claude_code_handoff.md")
}

/// Walk up from the current directory until a checkout root is found.
///
/// Answering from the cwd rather than from a compile-time constant is what makes a command run
/// inside a slice worktree read that worktree's files: worktrees share one build directory, so
/// the binary may have been compiled from a sibling checkout whose data is not the data at hand.
pub fn find_repo_root() -> Result<PathBuf> {
    let mut current = std::env::current_dir().context("cwd")?;
    loop {
        if current.join(ROOT_MARKER).is_file() {
            return Ok(current);
        }
        if !current.pop() {
            bail!("could not find repo root ({ROOT_MARKER})");
        }
    }
}

/// `true` when `candidate` is a checkout root — the probe callers use when they already hold a
/// directory and only need to confirm it, rather than walking up from the cwd.
pub fn is_repo_root(candidate: &Path) -> bool {
    candidate.join(ROOT_MARKER).is_file()
}

/// The directories and files a sparse checkout needs for each ticket target, keyed by the target
/// word a ticket's `[scope]` names.
///
/// `root` carries the whole tooling tree because `cargo xtask` is the task surface: a root slice
/// that cannot build xtask cannot run a single gate. `.cargo` rides with it for the command
/// aliases that make `cargo xtask` resolve at all.
pub const SPARSE_CHECKOUT_SETS: &[(&str, &[&str])] = &[
    ("website", &["apps/website"]),
    ("mod", &["apps/mod"]),
    ("shared", &["contracts_v2"]),
    (
        "root",
        &[
            TICKETS_DIR,
            ARTIFACTS_DIR,
            documentation::TREE_DIR,
            "tools_v2",
            ".cargo",
            "README.md",
            "CLAUDE.md",
        ],
    ),
];

/// Paths under the documentation tree.
///
/// Everything here is a document: a specification a ticket cites, a plan it must carry, a
/// document `ticket sync` writes into, a tree a scan walks or skips, or a path prefix that only
/// historical commits carry. Relocating the documentation tree rewrites exactly this module and
/// nothing else in the crate.
pub mod documentation {
    /// Root of the committed documentation tree.
    pub const TREE_DIR: &str = "docs";

    /// One four-section plan document per ticket, named by [`plan_path`].
    pub const PLANS_DIR: &str = "docs/plans";

    /// The plan skeleton a ticket copies when it goes ready without one.
    pub const PLAN_TEMPLATE: &str = "docs/plans/TEMPLATE.md";

    /// A ticket's own plan document: lowercase id, dots to underscores. `mark-ready` defaults an
    /// unset `plan` field to this path, and refuses while the file is absent.
    pub fn plan_path(id: &str) -> String {
        format!(
            "{PLANS_DIR}/{}_plan.md",
            id.to_lowercase().replace('.', "_")
        )
    }

    /// Program specifications — the shared authority a ticket's `spec` field points into.
    pub const SPECS_DIR: &str = "docs/specs";

    /// The architecture roadmap carrying the auto-generated "recommended next work" block that
    /// `ticket sync` injects between its markers.
    pub const ROADMAP: &str = "docs/specs/Mission_Creator_Architecture/ROADMAP.md";

    /// The gap-analysis table whose ticket column `ticket sync` keeps in step with the registry.
    pub const GAP_ANALYSIS: &str = "docs/specs/Mission_Creator_Architecture/eden/gap_analysis.md";

    /// The document of record for the tokens-per-line-changed factor. A test asserts the document
    /// quotes the compiled constant verbatim, so the two can never drift.
    pub const TOKEN_ESTIMATE_FACTOR_DOC: &str = "docs/platform/token_estimate_factor.md";

    /// Path prefix of the Markdown queue views that historical commits carry. No command writes
    /// a file under it; the token estimator's `git log --numstat` walk still meets these paths in
    /// old commits, so [`NUMSTAT_EXCLUDED_PREFIXES`] keeps their churn out of a ticket's
    /// changed-line count. Relocating the documentation tree does not move it: a historical
    /// commit keeps the spelling it was made with.
    pub const RETIRED_QUEUE_VIEW_PREFIX: &str = "docs/TICKET_";

    /// Trees the stale-identifier scan walks past. Each is either generated output, an imported
    /// design corpus, or a frozen pipeline record: none of them is prose an author maintains, so
    /// an identifier inside one is data rather than a stale reference.
    pub const SCAN_EXEMPT_PREFIXES: &[&str] = &[
        ".ai/artifacts/eden-wiki/",
        "frontend/src/stitch-exports/",
        "docs/specs/macOS_Blueprints/",
        "docs/specs/Mission_Creator_Mock_Up/",
        ".stitch-backup-exports/",
    ];

    /// Where the stale-identifier scan looks. Files first, then directories walked in full.
    pub const STALE_TICKET_ID_SCAN_ROOTS: &[&str] = &[
        TREE_DIR,
        SPECS_DIR,
        super::QUEUE_JSON,
        "CLAUDE.md",
        "README.md",
    ];

    /// Path prefixes the token estimator drops from a commit's changed-line count: the `.ai/`
    /// tree (the ticket registry and the agent artifact tree) and the retired queue views
    /// ([`RETIRED_QUEUE_VIEW_PREFIX`], matched on `.md` files only). `Cargo.lock` files are not
    /// prefixes: [`is_excluded_path`](crate::metrics::estimates::is_excluded_path) drops them by
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
            "docs/platform/SHIPPED_HISTORY.md",
            "the shipped-history archive describes past states in past commits",
        ),
        (
            "docs/platform/t911_ticket_registry_redesign.md",
            "an approved design document, written while the plans lived",
        ),
        (
            "docs/platform/t912_wave_lockfile.md",
            "the specification of the lock that replaced them names the files it deletes",
        ),
        (
            "docs/platform/GROK_WAVE_130_HANDOFF.md",
            "a kickoff document for a finished wave — a snapshot, not a runbook",
        ),
        (
            "docs/platform/WAVE209_GROK_KICKOFF.md",
            "a kickoff document for a finished wave — a snapshot, not a runbook",
        ),
        (
            "tools_v2/ticket-engine/src/repository.rs",
            "the module that spells every repository path the ticket domain touches, this pair \
             included",
        ),
        (
            "apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/Loaders/TBD_MissionValidator.c",
            "a lane note in an Enfusion comment; mod scripts compile through the Workbench and \
             are not edited from a platform slice",
        ),
        (
            "apps/website/api_v2/migrations/0011_events_server_modpack.sql",
            "committed migrations are checksum-frozen; rewording a comment in one breaks every \
             checkout that already applied it",
        ),
    ];
}

#[cfg(test)]
#[path = "tests/repository_layout_tests.rs"]
mod tests;
