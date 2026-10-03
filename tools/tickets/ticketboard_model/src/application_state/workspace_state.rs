//! The session state machine and the loaded board.
//!
//! **Role:** `State` (no repository, loading, refused, loaded) and `WorkspaceState`, the corpus with
//! every projection, filter, facet, sort and selection built over it, plus `ReloadPreferences`.
//! **Position:** built from a `LoadBundle`; owned by the desktop application's `TicketboardApp`,
//! which lends its parts to the feature views; `workspace_reload` rebuilds it on a watch reload.
//! **Signals & state:** `WorkspaceState` is the mutable board session; `refilter` recomputes the
//! facets, verdicts and visible rows after a filter change.
//! **Invariants:** projections are built once per load and filters once per change, never per
//! frame; selections are kept by ticket id, never by a stale corpus index.

use super::*;
/// The session state: no repository, loading, refused, or a loaded board.
pub enum State {
    /// No repo resolved (or an invalid one named in `note`) — full-window refusal.
    NoRepo {
        /// The reason the named root was rejected, when one was named.
        note: Option<String>,
    },
    /// The first load of a root is running.
    Loading,
    /// Fail-closed corpus refusal — full-window, file + verbatim error.
    Refused(LoadError),
    /// The loaded board.
    Board(Box<WorkspaceState>),
}

/// A loaded board: the corpus with every projection, filter, sort and selection built over it.
pub struct WorkspaceState {
    /// The loaded corpus; the mutation UI reads its tickets and paths for dialogs.
    pub corpus: Corpus,
    /// The status columns and their cards.
    pub board: BoardModel,
    /// wave.lock outcome — Waves-view-local; refusals never blank the board.
    pub lock: LockState,
    /// Built only when the lock loaded (rendered verbatim, never recomputed).
    pub waves: Option<WavesModel>,
    /// Run-receipt dashboard — Metrics-tab-local; NoReceipts is the
    /// explicit empty state, never zeros.
    pub metrics: MetricsState,
    /// Per-table sort selections for the metrics tables (survive reloads,
    /// carried like the filters).
    pub metrics_sort: SortPair,
    /// Estimated (historical) panel model — the estimate files joined
    /// against the corpus for class/domain buckets. STRUCTURALLY separate from
    /// `metrics`: no figure ever crosses between the two.
    pub estimates: EstimatesState,
    /// Sort selections for the two estimated tables (carried like
    /// `metrics_sort`).
    pub est_sort: EstimatedSortPair,
    /// The program tree.
    pub tree: TreeModel,
    /// The per-ticket filter facts.
    pub filter_index: FilterIndex,
    /// The current filter selections.
    pub filters: Filters,
    /// Scope-vocab tree — DISPLAY-ONLY facet-value source; `None`
    /// (missing/broken file) falls back to corpus-present values.
    pub vocab: Option<ScopeVocab>,
    /// Narrowed facet dropdown options — recomputed in `refilter`, never per
    /// frame.
    pub facet_options: FacetOptions,
    /// Per-corpus-index filter verdicts (all true when no filter is active).
    pub matches: Vec<bool>,
    /// How many tickets the filters let through.
    pub matched_count: usize,
    /// Board virtualization under filters: per column, the visible card rows.
    pub visible: [Vec<usize>; 8],
    /// Flattened tree rows for the virtualized tree view.
    pub tree_flat: Vec<tree::FlatRow>,
    /// Manual tree expansion by corpus index (filters force-expand match paths).
    pub tree_expanded: Vec<bool>,
    /// Wave 0 id list visibility — ALWAYS collapsed on load.
    pub wave0_expanded: bool,
    /// Quarantined `migration_legacy` lines expanded — collapsed by
    /// default; reset on every selection change, carried across watch reloads
    /// only while the same ticket stays selected.
    pub quarantine_expanded: bool,
    /// The selected ticket's corpus index.
    pub selected: Option<usize>,
    /// Second selection (shift-click) — the owns-collision explainer pair.
    pub compare: Option<usize>,
    /// Which status columns are expanded.
    pub expanded: [bool; 8],
    /// Precomputed footer base: the acceptance surface against
    /// `ls .ai/tickets/T-*.toml | wc -l`.
    pub footer_base: String,
    /// Rendered footer — prefixed with `matched/total` while filters are active.
    pub footer: String,
}

/// Filter + sort selections that survive a corpus reload (the watch
/// reloads on every registry change; raw indices go stale, preferences do not).
#[derive(Default)]
pub struct ReloadPreferences {
    /// The filter selections.
    pub filters: Filters,
    /// The measured tables' sorts.
    pub metrics_sort: SortPair,
    /// The estimated tables' sorts.
    pub est_sort: EstimatedSortPair,
}

impl WorkspaceState {
    /// Builds every projection over `corpus`, applies the carried filters and sorts, and computes
    /// the first filter pass.
    pub fn new(
        corpus: Corpus,
        lock: LockState,
        mut metrics: MetricsState,
        raw_estimates: estimates::RawEstimates,
        vocab: Option<ScopeVocab>,
        carried: ReloadPreferences,
    ) -> Self {
        let ReloadPreferences {
            filters,
            metrics_sort,
            est_sort,
        } = carried;
        let board = BoardModel::build(&corpus);
        let waves = match &lock {
            LockState::Loaded(l) => Some(WavesModel::build(&corpus, &board.id_to_index, l)),
            _ => None,
        };
        // Re-apply the carried sorts to the fresh aggregations (both models
        // load tokens-desc by default). Measured and estimated stay separate
        // models with separate sorts — never one table.
        if let MetricsState::Loaded(m) = &mut metrics {
            m.apply_sort(metrics_sort);
        }
        let mut est_state = estimates::build_state(raw_estimates, &corpus);
        if let EstimatesState::Loaded(e) = &mut est_state {
            e.apply_sort(est_sort);
        }
        let tree = TreeModel::build(&corpus, &board.id_to_index);
        let filter_index = FilterIndex::build(&corpus);
        let expanded = board::STATUS_ORDER.map(|s| !board::collapsed_by_default(s));
        let c = corpus.counts;
        let footer_base = format!(
            "{} ticket files — {} parents / {} children",
            c.total, c.parents, c.children
        );
        let total = corpus.tickets.len();
        let mut state = Self {
            corpus,
            board,
            lock,
            waves,
            metrics,
            metrics_sort,
            estimates: est_state,
            est_sort,
            tree,
            filter_index,
            filters,
            vocab,
            facet_options: FacetOptions::default(),
            matches: vec![true; total],
            matched_count: total,
            visible: Default::default(),
            tree_flat: Vec::new(),
            tree_expanded: vec![false; total],
            wave0_expanded: false,
            quarantine_expanded: false,
            selected: None,
            compare: None,
            expanded,
            footer: footer_base.clone(),
            footer_base,
        };
        state.refilter();
        state
    }

    /// Recompute every filter-derived surface: facet options (narrowed, stale
    /// lower selections cleared), verdicts, board rows, tree rows, footer. Runs
    /// on filter change only — never per frame.
    pub fn refilter(&mut self) {
        self.facet_options = facets::compute(
            self.vocab.as_ref(),
            &self.filter_index.rows,
            &mut self.filters.scope,
        );
        let (matches, matched_count) = self.filters.apply(&self.filter_index);
        self.matches = matches;
        self.matched_count = matched_count;
        for (col, column) in self.board.columns.iter().enumerate() {
            self.visible[col] = column
                .cards
                .iter()
                .enumerate()
                .filter(|(_, card)| self.matches[card.index])
                .map(|(row, _)| row)
                .collect();
        }
        self.reflatten();
        self.footer = if self.filters.is_active() {
            format!(
                "{}/{} tickets match · {}",
                self.matched_count, self.corpus.counts.total, self.footer_base
            )
        } else {
            self.footer_base.clone()
        };
    }

    /// Recompute the flattened tree rows (expansion toggle or filter change).
    pub fn reflatten(&mut self) {
        let filter = self.filters.is_active().then_some(self.matches.as_slice());
        self.tree_flat = tree::flatten(&self.tree, &self.tree_expanded, filter);
    }
}
