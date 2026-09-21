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
    /// Mint the next free dotted child under an existing parent.
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
        /// Skip the wave.lock refresh so a whole wave can be shipped and then repacked
        /// ONCE — a wave repacked per-id dissolves before any repack sees it fully landed, and
        /// never forms the pending entry `wave --close` needs. Run `cargo xtask wave repack`
        /// after the last id of the wave.
        #[arg(long)]
        no_repack: bool,
    },
    /// Step 3 of the ship lifecycle — after the landing commit exists, write
    /// its SHA onto the shipped ticket (`shipped_at`, both storage arms) and close the
    /// token accounting (estimates from the landed line count when neither a receipt nor an
    /// estimate exists; cohort_median at zero included LOC). Re-stamping the same sha
    /// is a no-op; another sha refuses (shipped_at is never overwritten). Flow:
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
        /// Plan ready-gate: path to this ticket's own plan document; defaults
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
    /// Report per-run receipts from `.ai/tickets/metrics/` (elapsed + token
    /// sums come from the real files; a broken file is an ERROR, never `tokens=0`).
    Metrics {
        /// Group sums (`agent` is the only supported key)
        #[arg(long)]
        by: Option<String>,
    },
    /// Read-only census of the typed corpus: per-domain/layer/component/surface counts,
    /// surface-empty honesty counters, and the work-ticket class distribution.
    #[command(name = "scope-histogram")]
    ScopeHistogram,
}
