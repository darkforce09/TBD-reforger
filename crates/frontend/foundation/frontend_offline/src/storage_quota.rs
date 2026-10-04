//! Storage quota: whether the origin has room for the offline pack, and the request to keep it.
//!
//! **Role:** compares the bytes still to download with the free space `navigator.storage.estimate()`
//! reports ([`quota_verdict`]), and asks the browser to make the origin's storage persistent
//! (`navigator.storage.persist()`) so eviction under storage pressure does not drop the pack.
//! **Position:** [`crate::offline_pack`] calls `estimate` and [`quota_verdict`] before
//! downloading and `request_persistence` once the download is allowed to start.
//! **Signals & state:** none.
//! **Invariants:** a download that does not fit is refused before any byte is fetched; an
//! estimate the browser does not offer is [`QuotaVerdict::Unknown`], which never refuses (a
//! failed write then reports the pack failed); persistence is best effort and never blocks.

/// What `navigator.storage.estimate()` reported, in bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StorageEstimate {
    /// The origin's quota.
    pub quota: u64,
    /// The bytes the origin already uses.
    pub usage: u64,
}

/// Whether the files still to download fit the origin's free storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuotaVerdict {
    /// They fit.
    Enough,
    /// They do not fit.
    Short {
        /// Bytes still to download.
        needed: u64,
        /// Bytes the origin has free.
        available: u64,
    },
    /// The browser reports no estimate.
    Unknown,
}

/// The verdict for downloading `needed` more bytes under `estimate`.
pub fn quota_verdict(estimate: Option<StorageEstimate>, needed: u64) -> QuotaVerdict {
    let Some(estimate) = estimate else {
        return QuotaVerdict::Unknown;
    };
    let available = estimate.quota.saturating_sub(estimate.usage);
    if needed <= available {
        QuotaVerdict::Enough
    } else {
        QuotaVerdict::Short { needed, available }
    }
}

/// A byte count reported as a JavaScript number, or `None` when it is not a finite,
/// non-negative number.
pub fn bytes_from_js_number(value: Option<f64>) -> Option<u64> {
    value
        .filter(|number| number.is_finite() && *number >= 0.0)
        .map(|number| number as u64)
}

#[cfg(target_arch = "wasm32")]
fn storage_manager() -> Option<web_sys::StorageManager> {
    use wasm_bindgen::JsCast;
    let navigator = web_sys::window()?.navigator();
    let storage = js_sys::Reflect::get(&navigator, &"storage".into()).ok()?;
    storage.dyn_into::<web_sys::StorageManager>().ok()
}

/// The origin's storage estimate, or `None` when the browser offers none.
#[cfg(target_arch = "wasm32")]
pub async fn estimate() -> Option<StorageEstimate> {
    let promise = storage_manager()?.estimate().ok()?;
    let report = wasm_bindgen_futures::JsFuture::from(promise).await.ok()?;
    let field = |name: &str| {
        bytes_from_js_number(
            js_sys::Reflect::get(&report, &name.into())
                .ok()
                .and_then(|value| value.as_f64()),
        )
    };
    Some(StorageEstimate {
        quota: field("quota")?,
        usage: field("usage").unwrap_or(0),
    })
}

/// Asks for persistent storage; `Some(granted)`, or `None` when the browser offers no such request.
#[cfg(target_arch = "wasm32")]
pub async fn request_persistence() -> Option<bool> {
    let promise = storage_manager()?.persist().ok()?;
    wasm_bindgen_futures::JsFuture::from(promise)
        .await
        .ok()?
        .as_bool()
}

#[cfg(test)]
#[path = "tests/storage_quota.rs"]
mod tests;
