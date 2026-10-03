//! Browser host of the ballistics agreement bench: the catalog reads and the paced solves.
//!
//! **Role:** parses the URL, reads the public catalog list and the chosen version's document
//! without credentials, solves the drawn cases one per macrotask and writes the finished
//! reading into the bench's signals.
//! **Position:** the `wasm32` half of [`super`]; builds on [`super::bench_query`] and
//! [`super::agreement_report`] and on the anonymous
//! [`crate::foundation::transport::client::public_reads::public_get`].
//! **Signals & state:** writes [`Signals`]; owns nothing else.
//! **Invariants:** the state leaves `loading` exactly once; a document that is not the version
//! requested is refused, never solved against; the page yields to the event loop between cases
//! so the DevTools protocol stays answerable during a long run.

use leptos::prelude::*;

use super::agreement_report::{assemble_report, case_report};
use super::bench_query::{choose_catalog_version, parse_bench_query};
use super::BenchState;
use crate::foundation::transport::client::public_reads::public_get;
use crate::foundation::transport::dto::ballistics_catalogs::{
    BallisticsCatalog, BallisticsCatalogList,
};
use map_engine::data::scenario::ballistics::agreement_cases::agreement_cases;

/// The reactive cells the host writes.
pub(super) struct Signals {
    /// The bench state behind `data-ballistics-agreement-state`.
    pub(super) state: RwSignal<BenchState>,
    /// The status line: progress, or the cause of a failure.
    pub(super) status: RwSignal<String>,
    /// The finished reading's JSON.
    pub(super) reading: RwSignal<String>,
}

/// Starts the one run of the bench.
pub(super) fn run(signals: Signals) {
    leptos::task::spawn_local(async move {
        match read_and_solve(&signals).await {
            Ok(json) => {
                signals.reading.set(json);
                signals.status.set("done".to_string());
                signals.state.set(BenchState::Ready);
            }
            Err(cause) => {
                signals.status.set(cause);
                signals.state.set(BenchState::Failed);
            }
        }
    });
}

async fn read_and_solve(signals: &Signals) -> Result<String, String> {
    let search = web_sys::window()
        .and_then(|window| window.location().search().ok())
        .unwrap_or_default();
    let query = parse_bench_query(&search)?;
    let abort = web_sys::AbortController::new().map_err(|_| "no request controller".to_string())?;
    signals.status.set("reading the catalog list…".to_string());
    let list = public_get::<BallisticsCatalogList>("/ballistics-catalogs", &abort.signal()).await?;
    let choice = choose_catalog_version(&list, &query)?;
    signals.status.set(format!(
        "reading {} v{}…",
        choice.catalog_id, choice.catalog_version
    ));
    let catalog = public_get::<BallisticsCatalog>(&choice.document_path(), &abort.signal()).await?;
    if catalog.catalog_id != choice.catalog_id || catalog.catalog_version != choice.catalog_version
    {
        return Err(format!(
            "the server answered catalog {} v{} for {} v{}",
            catalog.catalog_id, catalog.catalog_version, choice.catalog_id, choice.catalog_version
        ));
    }
    let cases = agreement_cases(&catalog, query.seed, query.count);
    let mut reports = Vec::with_capacity(cases.len());
    for (index, case) in cases.iter().enumerate() {
        signals
            .status
            .set(format!("solving case {} of {}…", index + 1, cases.len()));
        gloo_timers::future::TimeoutFuture::new(0).await;
        reports.push(case_report(&catalog, case));
    }
    let report = assemble_report(&catalog, query.seed, query.count, reports);
    serde_json::to_string(&report)
        .map_err(|error| format!("the reading does not serialise: {error}"))
}
