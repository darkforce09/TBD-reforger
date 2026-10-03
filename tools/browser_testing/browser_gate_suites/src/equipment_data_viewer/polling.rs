//! Background status checks preserve the visible document through success and failure.
//!
//! **Role:** the background status poll keeps the visible document through a success and a failure.
//! **Position:** a child of `equipment_data_viewer`, run on its page.
//! **Signals & state:** none beyond the page script it evaluates.
//! **Invariants:** the check reads the page script's own record, not timing.
use crate::error::{Result, ensure};
use chrome_devtools_protocol as cdp;
use serde_json::Value;

pub(super) async fn verify(page: &cdp::Page) -> Result<Value> {
    let result = page
        .evaluate(include_str!("polling_stability.js"), true)
        .await?;
    ensure!(
        result["polls"].as_u64().unwrap_or(0) >= 3,
        "background checks did not run: {result}"
    );
    ensure!(
        result["failure_observed"] == true,
        "failed status check was not exercised: {result}"
    );
    ensure!(
        result["recovered"] == true,
        "status checks did not recover: {result}"
    );
    ensure!(
        result["max_shift"].as_f64().unwrap_or(f64::MAX) < 1.0,
        "status polling shifts the page: {result}"
    );
    ensure!(
        result["content_preserved"] == true,
        "status polling replaces visible data: {result}"
    );
    ensure!(
        result["loading_flashes"] == 0,
        "background polling flashes loading state: {result}"
    );
    ensure!(
        result["slow_request_completed"] == true,
        "a periodic tick cancels the pending check: {result}"
    );
    ensure!(
        result["focus_preserved"] == true,
        "background polling loses input focus or text: {result}"
    );
    Ok(result)
}
