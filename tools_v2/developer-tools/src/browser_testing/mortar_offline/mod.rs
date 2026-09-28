//! `gate mortar-offline`: the mortar calculator works with the server gone.
//!
//! **Role:** proves the offline pack end to end: the built app, the Everon map assets and the
//! recorded catalog reads are served from a real HTTP origin; the first `/tools/mortar` visit
//! downloads the pack; the API goes down behind the still-running proxy (every `/api/` request
//! answers `502`) and a reload still solves from the saved copy (`api_down_visit`); the listener
//! stops; a reload is answered by the service worker with cross-origin isolation intact; a fire
//! mission typed and placed on the map solves to the native solution; the map imagery drew.
//! **Position:** a `gate` subcommand (`super::cli`), run by `cargo xtask mk mortar-offline-gate`
//! after a release build; it drives Chromium through `super::cdp` against `super::server` with
//! [`super::server::ServeConfig::api_fixture_corpus`] set, and never bypasses the service worker.
//! **Signals & state:** the running server, the browser and the page for one run; responses the
//! page receives after the reload are collected from `Network.responseReceived`; a
//! [`StepProgress`] holds the last step that passed.
//! **Invariants:** the steps run in [`STEPS`] order; every step prints
//! `case mortar_offline_<step> ... ok`, or `case mortar_offline_<step> ... FAILED: <cause>` and
//! ends the run; a missing asset, a pack that ends anything but `ready`, a navigation not answered by
//! the service worker, a solution that differs from the native one or a blank map is a failure;
//! only a run with every step ok prints `mortar-offline: PASS` and exits 0.

mod api_down_visit;
mod expected_solution;
mod map_pixels;
mod mission_entry;
mod mission_plan;
mod page_driver;

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use anyhow::{Result, anyhow, bail};
use serde_json::{Value, json};
use tokio::sync::mpsc::UnboundedReceiver;
use website_map_engine::data::scenario::ballistics::catalog::BallisticsCatalog;

use crate::browser_testing::cdp::{self, Browser, Page};
use crate::browser_testing::server::{ServeConfig, repo_root, start_server};
use crate::repository_layout::MapAssetMounts;
use api_down_visit::api_down_visit;
use mission_entry::{enter_mission, solution_matches_native};
use mission_plan::{COMMITTED_CATALOG, catalog_reads, read_catalog, require_files, required_files};
use page_driver::{map_canvas_rect, wait_for_pack_ready, wait_true};

pub use mission_plan::API_CORPUS_DIR;

/// The calculator route, on the WebGL backend every gate pins (software WebGPU wedges headless).
pub(super) const MORTAR_PATH: &str = "/tools/mortar?force=webgl";

/// How long the first visit may take to download the pack (about 250 MB from the local server).
pub(super) const PACK_BUDGET: Duration = Duration::from_secs(600);

/// The page script that words the map card's state and message.
const MAP_STATE_DETAIL: &str = "(() => { const m = document.querySelector('[data-mortar-map-state]'); \
     return m ? m.getAttribute('data-mortar-map-state') + ': ' + m.innerText : 'absent'; })()";

/// The command line of `gate mortar-offline`.
pub struct MortarOfflineArgs {
    /// The built app (`trunk build --release` output).
    pub dist: PathBuf,
    /// The gate server's port.
    pub port: u16,
    /// Chromium's remote debugging port.
    pub debug_port: u16,
    /// The recorded API corpus served under `/api/`.
    pub api_corpus: PathBuf,
}

/// The gate's steps, in the order they run.
pub const STEPS: [&str; 15] = [
    "preflight",
    "pack_ready",
    "worker_active",
    "api_down_behind_proxy",
    "catalog_list_from_worker_cache",
    "catalog_from_saved_copy",
    "pack_kept_ready",
    "api_down_solution_matches_native",
    "listener_stopped",
    "navigation_from_service_worker",
    "cross_origin_isolated",
    "catalog_and_map_offline",
    "mission_entered",
    "solution_matches_native",
    "tiles_drew",
];

/// How far one run got: the count of [`STEPS`] that passed, so a failure names its own step.
#[derive(Debug, Default)]
pub struct StepProgress {
    passed: AtomicUsize,
}

impl StepProgress {
    /// Prints `step` as passed and records every step up to it as passed.
    ///
    /// # Panics
    ///
    /// When `step` is not one of [`STEPS`].
    pub fn pass(&self, step: &str) {
        let index = STEPS
            .iter()
            .position(|s| *s == step)
            .unwrap_or_else(|| panic!("{step} is not a mortar-offline step"));
        println!("case mortar_offline_{step} ... ok");
        self.passed.store(index + 1, Ordering::SeqCst);
    }

    /// The step that runs next, which is the one a failure belongs to.
    #[must_use]
    pub fn failing_step(&self) -> &'static str {
        STEPS
            .get(self.passed.load(Ordering::SeqCst))
            .copied()
            .unwrap_or("teardown")
    }

    /// Prints the failure of the step that runs next, with its cause.
    pub fn fail(&self, cause: &str) {
        println!(
            "case mortar_offline_{} ... FAILED: {cause}",
            self.failing_step()
        );
    }
}

/// Runs the gate: 0 when every step passes, 1 when a step fails.
///
/// # Errors
///
/// A driver failure (Chromium or the server would not start).
pub async fn run(args: &MortarOfflineArgs) -> Result<u8> {
    let root = repo_root();
    let dist = root.join(&args.dist);
    let corpus = root.join(&args.api_corpus);
    let catalog = read_catalog(&root.join(COMMITTED_CATALOG))?;
    let reads = catalog_reads(&catalog)?;
    let steps = StepProgress::default();
    if let Err(e) = require_files(&required_files(&root, &dist, &corpus, &reads)) {
        steps.fail(&format!("{e:#}"));
        println!("mortar-offline: FAIL");
        return Ok(1);
    }
    let served = read_catalog(&corpus.join(&reads.version_file))?;
    steps.pass("preflight");

    let server = start_server(
        ServeConfig {
            dir: dist,
            api_proxy: None,
            map_assets: Some(MapAssetMounts::from_root(&root)),
            api_fixture_corpus: Some(corpus.clone()),
        },
        args.port,
    )
    .await?;
    let origin = format!("http://localhost:{}", server.port);
    let browser = cdp::launch(args.debug_port, &[]).await?;
    let outcome = async {
        let page = cdp::new_page(&browser, None, &[]).await?;
        page.send("Network.enable", json!({})).await?;
        online_visit(&page, &origin, &steps).await?;
        api_down_visit(&browser, &page, &origin, &corpus, &served, &steps).await?;
        server.close().await;
        offline_visit(&browser, &page, &origin, &served, &steps).await
    }
    .await;
    browser.shutdown().await;
    match outcome {
        Ok(()) => {
            println!("mortar-offline: PASS");
            Ok(0)
        }
        Err(e) => {
            steps.fail(&format!("{e:#}"));
            println!("mortar-offline: FAIL");
            Ok(1)
        }
    }
}

/// The first visit: the pack downloads and the service worker activates; `steps` records each
/// step that passes.
async fn online_visit(page: &Page, origin: &str, steps: &StepProgress) -> Result<()> {
    page.navigate(&format!("{origin}{MORTAR_PATH}")).await?;
    wait_for_pack_ready(page, PACK_BUDGET).await?;
    steps.pass("pack_ready");
    let active = page
        .evaluate_with_timeout(
            "navigator.serviceWorker.ready.then((r) => !!r.active)",
            true,
            Duration::from_secs(30),
        )
        .await?;
    if active != Value::Bool(true) {
        bail!("no active service worker after the pack download ({active})");
    }
    steps.pass("worker_active");
    Ok(())
}

/// The reload with the listener gone, the typed mission and the map.
async fn offline_visit(
    browser: &Browser,
    page: &Page,
    origin: &str,
    catalog: &BallisticsCatalog,
    steps: &StepProgress,
) -> Result<()> {
    if browser.http.get(format!("{origin}/")).send().await.is_ok() {
        bail!("the gate server still answers {origin} after it was closed");
    }
    steps.pass("listener_stopped");

    let mut responses = page.on_event("Network.responseReceived").await;
    let mut failures = page.on_event("Network.loadingFailed").await;
    page.navigate(&format!("{origin}{MORTAR_PATH}")).await?;
    let mut received = drain(&mut responses);
    let document = received
        .iter()
        .find(|r| r["type"] == "Document")
        .ok_or_else(|| anyhow!("no document response after the reload"))?;
    if document["response"]["fromServiceWorker"] != Value::Bool(true)
        || document["response"]["status"] != json!(200)
    {
        bail!(
            "the reload was not answered by the service worker: {}",
            document["response"]
        );
    }
    steps.pass("navigation_from_service_worker");
    if page.evaluate("crossOriginIsolated", false).await? != Value::Bool(true) {
        bail!("crossOriginIsolated is not true offline");
    }
    steps.pass("cross_origin_isolated");

    wait_true(
        page,
        "document.querySelectorAll('[data-mortar-input=\"weapon\"] option').length > 0",
        60,
        "the offline catalog loading",
    )
    .await?;
    if let Err(e) = wait_true(
        page,
        "!!document.querySelector('[data-mortar-map-state=\"ready\"]')",
        90,
        "the map mounting offline",
    )
    .await
    {
        let map = page.evaluate(MAP_STATE_DETAIL, false).await?;
        let failed: Vec<String> = drain(&mut failures)
            .iter()
            .map(|f| format!("{} {}", f["type"], f["errorText"]))
            .collect();
        received.extend(drain(&mut responses));
        let refused: Vec<String> = received
            .iter()
            .filter(|r| r["response"]["status"].as_u64().is_none_or(|s| s >= 400))
            .map(|r| format!("{} {}", r["response"]["status"], r["response"]["url"]))
            .collect();
        bail!("{e}: map {map}; failed requests {failed:?}; refused responses {refused:?}");
    }
    steps.pass("catalog_and_map_offline");

    let mission = enter_mission(page, catalog).await?;
    steps.pass("mission_entered");

    solution_matches_native(page, catalog, &mission).await?;
    steps.pass("solution_matches_native");

    received.extend(drain(&mut responses));
    tiles_drew(page, &mut received, &mut responses).await?;
    steps.pass("tiles_drew");
    Ok(())
}

/// The map drew imagery the service worker served: `received` holds every response since the
/// reload, `responses` the ones still queued.
async fn tiles_drew(
    page: &Page,
    received: &mut Vec<Value>,
    responses: &mut UnboundedReceiver<Value>,
) -> Result<()> {
    cdp::sleep_ms(1500).await;
    received.extend(drain(responses));
    let from_worker = received
        .iter()
        .filter(|r| {
            let response = &r["response"];
            response["url"]
                .as_str()
                .is_some_and(|u| u.contains("/map-assets/everon/"))
                && response["fromServiceWorker"] == Value::Bool(true)
                && matches!(response["status"].as_u64(), Some(200 | 206))
        })
        .count();
    if from_worker == 0 {
        bail!("no Everon map asset was answered by the service worker after the reload");
    }
    let rect = map_canvas_rect(page).await?;
    cdp::sleep_ms(300).await;
    let stats = map_pixels::region_stats(&page.screenshot().await?, rect)?;
    if !map_pixels::imagery_drew(stats) {
        bail!("the map canvas looks blank offline: {stats:?}");
    }
    println!("mortar-offline: {from_worker} map assets from the service worker; canvas {stats:?}");
    Ok(())
}

/// Every event queued on `rx` so far.
fn drain(rx: &mut UnboundedReceiver<Value>) -> Vec<Value> {
    let mut out = Vec::new();
    while let Ok(v) = rx.try_recv() {
        out.push(v);
    }
    out
}

#[cfg(test)]
#[path = "../tests/mortar_offline/step_progress.rs"]
mod tests;
