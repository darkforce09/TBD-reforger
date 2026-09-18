use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub(crate) enum TicketCmd {
    Sync,
    Check {
        #[arg(long)]
        strict: bool,
    },
    Brief {
        id: String,
    },
    Prompt {
        id: String,
        #[arg(long, default_value = "")]
        slice: String,
        #[arg(long)]
        header: bool,
    },
    Show {
        id: String,
    },
    Next,
    List,
    Milestone {
        milestone: String,
    },
    #[command(name = "plan-batch")]
    PlanBatch,
    #[command(name = "sparse-paths")]
    SparsePaths {
        id: String,
    },
    #[command(name = "gap-round-trip")]
    GapRoundTrip,
    Add {
        title: String,
        #[arg(long, default_value = "eden")]
        program: String,
        #[arg(long, default_value = "MAP")]
        surfaces: String,
        #[arg(long, default_value = "ui")]
        impact: String,
        #[arg(long, default_value = "")]
        summary: String,
    },
    /// T-916.2: mint the next free dotted child under an existing parent.
    #[command(name = "add-child")]
    AddChild {
        parent: String,
        title: String,
        #[arg(long, default_value = "")]
        summary: String,
        /// Required for a `kind = "work"` parent: atomically rewrites it work→program while
        /// adding the first child (its `[scope]` is dropped — programs forbid scope).
        #[arg(long)]
        promote: bool,
    },
    Remove {
        id: String,
        /// Required to remove a program: cascade-deletes every descendant ticket file.
        #[arg(long)]
        force: bool,
    },
    Reorder {
        id: String,
        after: String,
    },
    Ship {
        id: String,
        /// T-946: skip the wave.lock refresh so a whole wave can be shipped and then repacked
        /// ONCE — a wave repacked per-id dissolves before any repack sees it fully landed, and
        /// never forms the pending entry `wave --close` needs. Run `cargo xtask wave repack`
        /// after the last id of the wave.
        #[arg(long)]
        no_repack: bool,
    },
    /// T-917.6: step 3 of the ship lifecycle — after the landing commit exists, write
    /// its SHA onto the shipped ticket (`shipped_at`, both storage arms) and close the
    /// token accounting (generates the diff_loc estimate when neither a receipt nor an
    /// estimate exists; cohort_median at zero included LOC). Re-stamping the same sha
    /// is a no-op; a different sha refuses (shipped_at is never overwritten). Flow:
    /// `ticket ship <id>` → commit → `ticket stamp-sha <id> $(git rev-parse --short HEAD)`.
    #[command(name = "stamp-sha")]
    StampSha {
        id: String,
        sha: String,
    },
    #[command(name = "mark-ready")]
    MarkReady {
        id: String,
        spec: Option<String>,
        /// T-917.6 plan ready-gate: path to this ticket's own plan document; defaults
        /// to docs/plans/<id-lowercased-dots-to-underscores>_plan.md and must exist
        /// on disk (copy docs/plans/TEMPLATE.md).
        plan: Option<String>,
    },
    #[command(name = "advance-slice")]
    AdvanceSlice {
        id: String,
    },
    #[command(name = "ready-ids")]
    ReadyIds {
        #[arg(long)]
        limit: Option<usize>,
        #[arg(long, default_value = "")]
        stream: String,
    },
    #[command(name = "set-status")]
    SetStatus {
        id: String,
        status: String,
    },
    Get {
        id: String,
        field: Option<String>,
    },
    Config {
        key: String,
    },
    Run {
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        stream: Option<String>,
    },
    Done {
        id: String,
    },
    Clean {
        id: String,
    },
    /// T-913.2: report per-run receipts from `.ai/tickets/metrics/` (elapsed + token
    /// sums come from the real files; a broken file is an ERROR, never `tokens=0`).
    Metrics {
        /// Group sums (`agent` is the only supported key)
        #[arg(long)]
        by: Option<String>,
    },
    /// T-917.2: THE schema-v2 cutover — one-shot v1→v2 rewrite of every ticket file
    /// (flat scope, class triage, estimated markers), kept for corroboration.
    #[command(name = "migrate-v2")]
    MigrateV2,
    /// T-917.2: per-domain/layer/component/surface counts + surface-empty honesty
    /// counters + class distribution, from the typed corpus (read-only).
    #[command(name = "scope-histogram")]
    ScopeHistogram,
    /// T-917.3: wall quarantine pass 1 — move every work-ticket summary over the
    /// 40-word cap verbatim into migration_legacy[] (byte-reversible, proved per
    /// file), summary := title. Idempotent by emptiness; regenerates the sync surface.
    #[command(name = "quarantine-walls")]
    QuarantineWalls,
    /// T-917.4: stamp backfill — mine created_at/completed_at/shipped_at for every
    /// shipped ticket from exact-id boundary-matched commit subjects (UTC-normalized),
    /// id-interpolation fallback where no subjects exist; every derived stamp marked
    /// in estimated[]. One-shot, idempotent by emptiness.
    #[command(name = "backfill-stamps")]
    BackfillStamps,
    /// T-917.5: token estimates — every SHIPPED ticket with neither a run receipt
    /// under metrics/<id>/ nor an estimates/<id>.json gets one: diff_loc (LOC changed
    /// across its subject commits × the documented factor, bookkeeping paths
    /// excluded) with cohort_median fallback. Writes .ai/tickets/estimates/<id>.json
    /// + the "tokens" estimated[] marker. One-shot, idempotent by emptiness.
    #[command(name = "estimate-tokens")]
    EstimateTokens,
    /// T-920.1: one-shot user_story → main_goal on-disk migration (load parses the
    /// alias, write_back emits main_goal in the same canonical slot) plus the
    /// same-land live-ready body fills, derived from each ticket's plan document.
    /// Idempotent: fills only all-empty targets, migrates only raw carriers.
    #[command(name = "migrate-main-goal")]
    MigrateMainGoal,
}
