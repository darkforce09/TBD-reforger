//! The browser half of the gate: serve the built app, answer the bench's catalog reads with the
//! goldens, and read the bench's reading.
//!
//! **Role:** starts the static server over the built single-page app, launches headless
//! Chromium, bypasses the offline service worker (`Page::bypass_service_worker`), answers the two public catalog reads with the
//! golden bytes over `Fetch.requestPaused`, opens `/debug/ballistics-agreement`, waits for the
//! bench to leave `loading` and returns the text of its `<pre>`.
//! **Position:** called by [`super::run`] with the [`ServedGoldens`] that
//! [`super::golden_provenance`] proved; built on [`crate::server`] and
//! [`chrome_devtools_protocol`].
//! **Signals & state:** owns one server, one browser and one page for the length of a run; a
//! spawned task answers intercepted requests and records the API paths nothing answers.
//! **Invariants:** the page never reaches a live API: every `/api/v1/` request is answered here,
//! the two catalog reads with the goldens byte for byte and anything else with 404; the server
//! and the browser are shut down on every path out; a bench that ends `failed` or never leaves
//! `loading` is an error naming its status line.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::Result;
use crate::error::{bail, refusal};
use serde_json::{Value, json};

use super::golden_provenance::ServedGoldens;
use crate::server::{ServeConfig, start_server};
use chrome_devtools_protocol::{self as cdp, Page};

/// Where and what the browser half opens.
#[derive(Clone, Debug)]
pub struct BenchSession {
    /// The built single-page app.
    pub dist: PathBuf,
    /// Port of the static server.
    pub port: u16,
    /// Chromium's remote-debugging port.
    pub debug_port: u16,
    /// The bench's path and query, starting with `/debug/ballistics-agreement`.
    pub bench_path: String,
    /// Catalog id and version the document golden answers.
    pub catalog: (String, u32),
    /// Longest wait for the bench to finish.
    pub timeout: Duration,
}

/// The bench's text once it finished, and the API paths it asked for that nothing answers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BenchOutput {
    /// Text of `<pre data-ballistics-agreement>`.
    pub reading: String,
    /// `/api/v1/` paths answered with 404.
    pub unanswered: Vec<String>,
}

/// Runs the bench in headless Chromium against the goldens.
///
/// # Errors
///
/// A server, browser or protocol failure, or a bench that failed or did not finish in time.
pub async fn read_bench(session: &BenchSession, goldens: &ServedGoldens) -> Result<BenchOutput> {
    let server = start_server(
        ServeConfig {
            dir: session.dist.clone(),
            api_proxy: None,
            map_assets: None,
            api_fixture_corpus: None,
        },
        session.port,
    )
    .await?;
    let browser = match cdp::launch(session.debug_port, &[]).await {
        Ok(browser) => browser,
        Err(error) => {
            server.close().await;
            return Err(error.into());
        }
    };
    let outcome = drive(session, goldens, &browser, server.port).await;
    browser.shutdown().await;
    server.close().await;
    outcome
}

async fn drive(
    session: &BenchSession,
    goldens: &ServedGoldens,
    browser: &cdp::Browser,
    port: u16,
) -> Result<BenchOutput> {
    let page = Arc::new(cdp::new_page(browser, None, &[]).await?);
    page.bypass_service_worker().await?;
    page.send(
        "Fetch.enable",
        json!({ "patterns": [{ "urlPattern": "*/api/v1/*" }] }),
    )
    .await?;
    let unanswered = answer_catalog_reads(&page, goldens, &session.catalog).await;
    let url = format!("http://localhost:{port}{}", session.bench_path);
    page.navigate(&url).await?;

    const STATE: &str = "document.querySelector('[data-ballistics-agreement-state]')\
                         ?.getAttribute('data-ballistics-agreement-state') ?? 'absent'";
    let poll = Duration::from_millis(250);
    let mut waited = Duration::ZERO;
    let state = loop {
        let state = page.evaluate(STATE, false).await?;
        let state = state.as_str().unwrap_or("absent").to_string();
        if state != "loading" && state != "absent" {
            break state;
        }
        if waited >= session.timeout {
            break state;
        }
        cdp::sleep_ms(poll.as_millis() as u64).await;
        waited += poll;
    };
    let status = text_of(&page, "[data-ballistics-agreement-status]").await?;
    let unanswered = unanswered
        .lock()
        .map(|paths| paths.clone())
        .unwrap_or_default();
    match state.as_str() {
        "ready" => Ok(BenchOutput {
            reading: text_of(&page, "pre[data-ballistics-agreement]").await?,
            unanswered,
        }),
        "failed" => bail!("the bench failed: {status} (unanswered API paths: {unanswered:?})"),
        other => Err(refusal!(
            "the bench is still `{other}` after {} s: {status}",
            session.timeout.as_secs()
        )),
    }
}

async fn text_of(page: &Page, selector: &str) -> Result<String> {
    let expression = format!(
        "document.querySelector({})?.textContent ?? ''",
        serde_json::to_string(selector)?
    );
    Ok(page
        .evaluate(&expression, false)
        .await?
        .as_str()
        .unwrap_or_default()
        .to_string())
}

/// Answers every intercepted `/api/v1/` request: the list and the version document with the
/// goldens, anything else with 404 (recorded in the returned list).
async fn answer_catalog_reads(
    page: &Arc<Page>,
    goldens: &ServedGoldens,
    catalog: &(String, u32),
) -> Arc<Mutex<Vec<String>>> {
    let unanswered = Arc::new(Mutex::new(Vec::new()));
    let mut paused = page.on_event("Fetch.requestPaused").await;
    let page = Arc::clone(page);
    let goldens = goldens.clone();
    let document_path = format!(
        "/api/v1/ballistics-catalogs/{}/versions/{}",
        catalog.0, catalog.1
    );
    let recorded = Arc::clone(&unanswered);
    tokio::spawn(async move {
        while let Some(event) = paused.recv().await {
            let Some(request_id) = event["requestId"]
                .as_str()
                .map(chrome_devtools_protocol::InterceptedRequestId::new)
            else {
                continue;
            };
            let request_id = &request_id;
            let path = request_path(&event);
            let body = if path == "/api/v1/ballistics-catalogs" {
                Some(&goldens.list_body)
            } else if path == document_path {
                Some(&goldens.document_body)
            } else {
                None
            };
            let _ = match body {
                Some(body) => {
                    page.fulfill_raw(request_id, 200, "application/json", body)
                        .await
                }
                None => {
                    if let Ok(mut paths) = recorded.lock() {
                        paths.push(path);
                    }
                    page.fulfill_json(request_id, 404, &json!({})).await
                }
            };
        }
    });
    unanswered
}

/// The path of an intercepted request, without its origin, query or fragment.
fn request_path(event: &Value) -> String {
    let url = event["request"]["url"].as_str().unwrap_or_default();
    let after_scheme = url.split_once("://").map_or(url, |(_, rest)| rest);
    let path = after_scheme
        .find('/')
        .map_or("/", |start| &after_scheme[start..]);
    let end = path.find(['?', '#']).unwrap_or(path.len());
    path[..end].trim_end_matches('/').to_string()
}
