//! The visit with the API down behind the proxy: the page is served, every `/api/` request
//! answers `502 Bad Gateway`, and the calculator still solves from the saved copy.
//!
//! **Role:** marks the corpus's API down, proves the server answers `502` for the catalog list,
//! reloads `/tools/mortar`, and holds the page to four facts: the service worker answers the
//! catalog list from its cache (marked as a saved copy), the page words the offline copy with its
//! date, the re-run pack stays `ready` with its refresh kept on the saved copy, and a typed
//! mission solves to the native solution.
//! **Position:** runs between the online visit and the listener stop of `super::run`; marks the
//! API up again before it returns.
//! **Signals & state:** the process-wide down mark of
//! [`crate::browser_testing::server::api_fixture_corpus::set_api_down`] for the gate's corpus.
//! **Invariants:** a `502` from the proxy is never shown as "No offline copy"; the catalog list
//! the page fetches is `200` with the saved-copy marker, so a worker that falls back only on
//! network errors fails `catalog_list_from_worker_cache`; the pack never ends `failed` because a
//! refresh the proxy could not answer.

use std::path::Path;
use std::time::Duration;

use anyhow::{Result, bail};
use map_engine::data::scenario::ballistics::catalog::BallisticsCatalog;
use serde_json::{Value, json};

use super::mission_entry::{enter_mission, solution_matches_native};
use super::page_driver::{wait_for_pack_ready, wait_true};
use super::{MORTAR_PATH, PACK_BUDGET, StepProgress};
use crate::browser_testing::cdp::{Browser, Page};
use crate::browser_testing::server::api_fixture_corpus::set_api_down;

/// The header the offline worker adds to a saved copy it answers with (the offline worker's
/// `network_fallback::SAVED_COPY_HEADER`).
const SAVED_COPY_HEADER: &str = "x-served-from-offline-cache";

/// The page script that fetches the catalog list through the service worker and reports the
/// status, the saved-copy marker and whether a worker controls the page.
fn catalog_list_through_worker() -> String {
    format!(
        "fetch('/api/v1/ballistics-catalogs', {{ credentials: 'omit' }}).then((r) => ({{ \
         status: r.status, marker: r.headers.get('{SAVED_COPY_HEADER}'), \
         controlled: !!navigator.serviceWorker.controller }}))"
    )
}

/// The text of the calculator's catalog block.
const CATALOG_TEXT: &str =
    "(document.querySelector('[data-mortar-catalog]') || {}).innerText || ''";

/// The text of the offline pack line and the refresh attribute of the document element.
const PACK_LINE: &str = "({ text: (document.querySelector('[data-mortar-offline]') || {}).innerText \
     || '', refresh: document.documentElement.getAttribute('data-offline-refresh') })";

/// Runs the visit with the API behind `corpus` down; `steps` records each step that passes.
pub async fn api_down_visit(
    browser: &Browser,
    page: &Page,
    origin: &str,
    corpus: &Path,
    catalog: &BallisticsCatalog,
    steps: &StepProgress,
) -> Result<()> {
    set_api_down(corpus, true);
    let outcome = steps_with_api_down(browser, page, origin, catalog, steps).await;
    set_api_down(corpus, false);
    outcome
}

async fn steps_with_api_down(
    browser: &Browser,
    page: &Page,
    origin: &str,
    catalog: &BallisticsCatalog,
    steps: &StepProgress,
) -> Result<()> {
    let direct = browser
        .http
        .get(format!("{origin}/api/v1/ballistics-catalogs"))
        .send()
        .await?;
    if direct.status().as_u16() != 502 {
        bail!(
            "the proxy answered {} for the catalog list, not 502",
            direct.status()
        );
    }
    steps.pass("api_down_behind_proxy");

    page.navigate(&format!("{origin}{MORTAR_PATH}")).await?;
    let list = page
        .evaluate_with_timeout(
            &catalog_list_through_worker(),
            true,
            Duration::from_secs(30),
        )
        .await?;
    if list["controlled"] != Value::Bool(true)
        || list["status"] != json!(200)
        || list["marker"] != json!("1")
    {
        bail!(
            "the catalog list was not answered from the worker's cache with {SAVED_COPY_HEADER}: \
             {list}"
        );
    }
    steps.pass("catalog_list_from_worker_cache");

    wait_true(
        page,
        "document.querySelectorAll('[data-mortar-input=\"weapon\"] option').length > 0",
        60,
        "the catalog loading from the saved copy",
    )
    .await?;
    let notice = page.evaluate(CATALOG_TEXT, false).await?;
    let notice = notice.as_str().unwrap_or_default();
    if !notice.contains("Offline copy from ") || notice.contains("No offline copy") {
        bail!("the page does not word the dated offline copy: {notice:?}");
    }
    steps.pass("catalog_from_saved_copy");

    wait_for_pack_ready(page, PACK_BUDGET).await?;
    let line = page.evaluate(PACK_LINE, false).await?;
    let text = line["text"].as_str().unwrap_or_default();
    if line["refresh"] != json!("kept-saved-copy")
        || !text.contains("Refresh failed, using the saved copy from ")
    {
        bail!("the re-run pack does not say it kept the saved copy: {line}");
    }
    steps.pass("pack_kept_ready");

    wait_true(
        page,
        "!!document.querySelector('[data-mortar-map-state=\"ready\"]')",
        90,
        "the map mounting with the API down",
    )
    .await?;
    let mission = enter_mission(page, catalog).await?;
    solution_matches_native(page, catalog, &mission).await?;
    steps.pass("api_down_solution_matches_native");
    Ok(())
}
