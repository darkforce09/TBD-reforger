//! The combined background load of a board.
//!
//! **Role:** `LoadBundle` and `spawn_load`: the corpus, the wave lock, the run receipts, the scope
//! vocabulary and the raw estimate files, read together on one worker thread.
//! **Position:** called by the desktop application at start, on root adoption and on every watch
//! reload; reads through `crate::ticket_registry`, `crate::wave_plan`, `crate::execution_metrics`
//! and `crate::ticket_browser::services::scope_facets`; `WorkspaceState::new` consumes the bundle.
//! **Signals & state:** one worker thread per load, reporting over an `mpsc` channel; `on_done`
//! fires after the send.
//! **Invariants:** the UI thread never touches the disk; only the corpus can refuse the load, while
//! lock, receipt, estimate and vocabulary failures ride inside the bundle as local states.

use crate::{
    execution_metrics::{
        estimated::{self as estimates, RawEstimates},
        measured::{self as metrics, MetricsState},
    },
    ticket_browser::services::scope_facets::load_vocabulary,
    ticket_registry::services::corpus_loading::{LoadResult, load_corpus},
    wave_plan::services::lock_file::{self as wavelock, LockState},
};
use std::{path::PathBuf, sync::mpsc, thread};
use ticket_model::ScopeVocab;
/// Corpus + wave.lock + run receipts, loaded together on the worker thread
/// . The lock rides alongside because its refusals are
/// Waves-view-local: a missing lock is a PLAN refusal (the DidNotRun text), not
/// a corpus refusal — the board must still render. The metrics state rides for
/// the same reason (its empty/error states are Metrics-tab-local), and because
/// `.ai/tickets/metrics/` sits inside the watched tree, so the same debounced
/// watch fires refresh receipts with corpus + lock.
pub struct LoadBundle {
    /// The corpus load: every ticket, or the refusal naming the first bad file.
    pub corpus: LoadResult,
    /// The wave lock: loaded, missing or refused, local to the Waves tab.
    pub lock: LockState,
    /// The run receipts: no receipts, or the measured model with its error rows.
    pub metrics: MetricsState,
    /// Scope-vocab tree for the facet dropdowns — DISPLAY-ONLY input;
    /// missing/broken file is `None` (facets fall back to corpus-present values),
    /// never a load refusal.
    pub vocab: Option<ScopeVocab>,
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
            vocab: load_vocabulary(&repo_root),
            estimates: estimates::load_raw(&repo_root),
        };
        let _ = tx.send(bundle);
        on_done();
    });
    rx
}
