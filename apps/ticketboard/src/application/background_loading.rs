use crate::{
    execution_metrics::{
        estimated::{self as estimates, RawEstimates},
        measured::{self as metrics, MetricsState},
    },
    ticket_browser::services::scope_facets::VocabTree,
    ticket_registry::services::corpus_loading::{LoadResult, load_corpus},
    wave_plan::services::lock_file::{self as wavelock, LockState},
};
use std::{path::PathBuf, sync::mpsc, thread};
/// Corpus + wave.lock + run receipts, loaded together on the worker thread
/// . The lock rides alongside because its refusals are
/// Waves-view-local: a missing lock is a PLAN refusal (the DidNotRun text), not
/// a corpus refusal — the board must still render. The metrics state rides for
/// the same reason (its empty/error states are Metrics-tab-local), and because
/// `.ai/tickets/metrics/` sits inside the watched tree, so the same debounced
/// watch fires refresh receipts with corpus + lock.
pub struct LoadBundle {
    pub corpus: LoadResult,
    pub lock: LockState,
    pub metrics: MetricsState,
    /// Scope-vocab tree for the facet dropdowns — DISPLAY-ONLY input;
    /// missing/broken file is `None` (facets fall back to corpus-present values),
    /// never a load refusal.
    pub vocab: Option<VocabTree>,
    /// Token-estimate files — raw per-file parse results off
    /// `.ai/tickets/estimates/` (inside the watched tree, hence the shared
    /// load). The per-class/per-domain aggregation joins the corpus later, in
    /// `WorkspaceState::new` (`estimates::build_state`). Estimated figures NEVER
    /// enter the metrics receipts model — structurally separate trees.
    pub estimates: RawEstimates,
}

/// Run `load_corpus` + the wave.lock load on a worker thread; the UI thread never
/// touches the disk. `on_done` fires after the result is sent (the app passes
/// `egui::Context::request_repaint`).
pub fn spawn_load(
    repo_root: PathBuf,
    on_done: impl FnOnce() + Send + 'static,
) -> mpsc::Receiver<LoadBundle> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let bundle = LoadBundle {
            corpus: load_corpus(&repo_root),
            lock: wavelock::load_lock(&repo_root),
            metrics: metrics::load_metrics(&repo_root),
            vocab: VocabTree::load(&repo_root),
            estimates: estimates::load_raw(&repo_root),
        };
        let _ = tx.send(bundle);
        on_done();
    });
    rx
}
