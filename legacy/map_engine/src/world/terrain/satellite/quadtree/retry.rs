//! Role: retry.
//! Position: `world/terrain/satellite/quadtree` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use super::JsFuture;
use super::RangeBody;
use super::RangeOutcome;
use super::fetch_range_outcome;

/// Canonical range attempts value.
pub(super) const RANGE_ATTEMPTS: usize = 5;

/// Canonical range backoff ms value.
pub(super) const RANGE_BACKOFF_MS: [i32; 4] = [100, 250, 600, 1_200];

/// Sleep ms.
pub(super) async fn sleep_ms(ms: i32) {
    let p = js_sys::Promise::new(&mut |resolve, _reject| {
        let scheduled = web_sys::window().and_then(|w| {
            w.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms)
                .ok()
        });
        if scheduled.is_none() {
            let _ = resolve.call0(&wasm_bindgen::JsValue::NULL);
        }
    });
    let _ = JsFuture::from(p).await;
}

/// Fetch range resilient.
pub(super) async fn fetch_range_resilient(url: &str, start: u64, end: u64) -> Option<RangeBody> {
    for attempt in 1..=RANGE_ATTEMPTS {
        let throttled = match fetch_range_outcome(url, start, end).await {
            RangeOutcome::Body(body) => {
                if attempt > 1 {
                    crate::diagnostics::platform::console::warn!(
                        "satellite: Range bytes={start}-{end} succeeded on attempt {attempt}"
                    );
                }
                return Some(body);
            }
            RangeOutcome::RateLimited { retry_after_s } => {
                crate::diagnostics::platform::console::warn!(
                    "satellite: Range bytes={start}-{end} throttled (429, Retry-After {:?}) \
                     — attempt {attempt}/{RANGE_ATTEMPTS}",
                    retry_after_s
                );
                true
            }
            RangeOutcome::Failed { status } => {
                crate::diagnostics::platform::console::warn!(
                    "satellite: Range bytes={start}-{end} failed (status {status}) — attempt \
                     {attempt}/{RANGE_ATTEMPTS}"
                );
                false
            }
        };

        let Some(&base_ms) = RANGE_BACKOFF_MS.get(attempt - 1) else {
            break;
        };

        let wait = if throttled { base_ms } else { base_ms / 2 };
        sleep_ms(wait).await;
    }
    None
}
