//! The run of one member load: the plan's checks, the account file, the clients on a runtime of
//! their own, and the report.
//!
//! - **Role:** [`run`] turns a [`LoadRunPlan`] into a [`LoadReport`]. It checks the plan, reads the
//!   account file once, confirms that every source address belongs to this machine, drives one
//!   virtual client task per client on a runtime of its own, and folds their records into the
//!   report.
//! - **Position:** the crate's entry, re-exported at its root. The `staging-load` executable's
//!   command line calls it with the plan the xtask load procedure encoded: the committed
//!   workload, the target origin, the source addresses, the account file and the fixture events;
//!   the API behind the target origin is the only peer.
//! - **Signals & state:** the runtime, the per-address guards and the clients' account rings live
//!   for one call; rotated tokens stay in memory and are dropped with them.
//! - **Invariants:**
//!   - Every request leaves from a source address of the plan and passes that address's ceilings.
//!   - A client joins the member load only once it holds a token, and switches accounts through
//!     refreshes prefetched beside its member requests, so the harness's own pacing never delays
//!     a member request.
//!   - A client has at most one member request and one refresh in flight, and latency runs from
//!     the scheduled instant to the end of the body.
//!   - The report holds counts, latencies, addresses and template ids: never a token, a header or a
//!     body.

use std::sync::Arc;
use std::time::Duration;

use staging_load_plan::client_outcome::ClientOutcome;
use staging_load_plan::load_report;
use staging_load_plan::request_catalog::RequestCatalog;
use staging_load_plan::run_settings::RunSettings;
use staging_load_plan::{LoadReport, LoadRunPlan, verify_source_addresses};
use tokio::time::Instant;

use crate::account_rotation::{self, ClientAccounts};
use crate::error::{Error, Result};
use crate::source_address_pool::SourceAddressPool;
use crate::virtual_client::{RunContext, VirtualClient};

/// Time between building the clients and the run's first scheduled instant.
const START_LEAD: Duration = Duration::from_millis(200);
/// Worker threads of the run's runtime at most: the clients spend nearly all their time waiting.
const MAX_WORKER_THREADS: usize = 4;
/// How long the runtime may take to wind down once every client has returned.
const SHUTDOWN_GRACE: Duration = Duration::from_secs(2);

/// Run the member load `plan` describes and report what it measured.
///
/// Blocks the calling thread for the ramp, the measured window and the last in-flight requests.
/// It builds its own multi-threaded tokio runtime, so it is called from synchronous code, never
/// from inside another runtime.
///
/// # Errors
///
/// An invalid plan, an unreadable or mismatched account file, a source address that is not
/// assigned to this machine, or a runtime or HTTP client that cannot be built. A failed request is
/// never an error: the report counts it.
pub fn run(plan: &LoadRunPlan) -> Result<LoadReport> {
    let settings = plan.checked_settings()?;
    let origin = plan.normalized_origin()?;
    let catalog = RequestCatalog::compile(&plan.workload.request_mix)?;
    verify_source_addresses(&plan.source_addresses)?;
    let accounts = account_rotation::read_account_file(&plan.account_file, settings.accounts)?;
    let dealt = account_rotation::deal_accounts(
        accounts,
        &plan.fixture_events,
        settings.clients,
        catalog.template_count(),
    );
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(worker_threads())
        .thread_name("load-generation")
        .enable_all()
        .build()
        .map_err(|error| Error::RuntimeNotBuilt { error })?;
    let outcomes = runtime.block_on(drive(plan, settings, origin, catalog, dealt));
    runtime.shutdown_timeout(SHUTDOWN_GRACE);
    Ok(load_report::assemble(plan, &settings, &outcomes?))
}

fn worker_threads() -> usize {
    std::thread::available_parallelism()
        .map_or(1, usize::from)
        .min(MAX_WORKER_THREADS)
}

/// Build every client, start them against one shared clock, and gather what they recorded.
async fn drive(
    plan: &LoadRunPlan,
    settings: RunSettings,
    origin: String,
    catalog: RequestCatalog,
    dealt: Vec<ClientAccounts>,
) -> Result<Vec<ClientOutcome>> {
    let pool = SourceAddressPool::new(&plan.source_addresses, &settings);
    let mut clients = Vec::with_capacity(dealt.len());
    for (index, accounts) in (0u32..).zip(dealt) {
        clients.push(VirtualClient::new(
            index,
            &settings,
            &pool,
            plan.workload.seed,
            accounts,
        )?);
    }
    let start = Instant::now() + START_LEAD;
    let context = Arc::new(RunContext {
        origin,
        start,
        end: start + settings.ramp + settings.measured,
        settings,
        catalog,
        pool,
    });
    let tasks: Vec<_> = clients
        .into_iter()
        .map(|client| tokio::spawn(client.run(Arc::clone(&context))))
        .collect();
    let mut outcomes = Vec::with_capacity(tasks.len());
    for task in tasks {
        outcomes.push(
            task.await
                .map_err(|error| Error::ClientTaskStopped { error })?,
        );
    }
    Ok(outcomes)
}
