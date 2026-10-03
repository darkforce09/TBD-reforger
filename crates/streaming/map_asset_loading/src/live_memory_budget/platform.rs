//! **Role:** what the live ledger reads from the page: the configured budget and the heap size.
//! **Position:** `live_memory_budget` in `map_asset_loading`; the thread-local ledger is built from
//! [`configured_budget_bytes`], the accounting calls measure growth with [`heap_bytes`].
//! **Signals & state:** reads the page's query string, `window.__memBudgetMb` and the wasm
//! linear memory on wasm32; nothing on native.
//! **Invariants:** the settings resolve through
//! `map_streaming_model::memory_budget::budget_bytes_from_settings`; a native build takes the
//! default budget and measures no heap.

use map_streaming_model::memory_budget::budget_bytes_from_settings;

/// The budget for this boot, in bytes: `?memBudgetMb=NNN`, else `window.__memBudgetMb`, else
/// `map_streaming_model::memory_budget::DEFAULT_BUDGET_MB`.
#[cfg(target_arch = "wasm32")]
#[must_use]
pub fn configured_budget_bytes() -> u64 {
    let query_budget_mb = web_sys::window()
        .and_then(|w| w.location().search().ok())
        .and_then(|s| web_sys::UrlSearchParams::new_with_str(&s).ok())
        .and_then(|p| p.get("memBudgetMb"));
    let window_budget_mb = web_sys::window()
        .and_then(|w| {
            js_sys::Reflect::get(&w, &wasm_bindgen::JsValue::from_str("__memBudgetMb")).ok()
        })
        .and_then(|v| v.as_f64());
    budget_bytes_from_settings(query_budget_mb.as_deref(), window_budget_mb)
}

/// The budget for this boot, in bytes: a native build has no page settings, so the default.
#[cfg(not(target_arch = "wasm32"))]
#[must_use]
pub fn configured_budget_bytes() -> u64 {
    budget_bytes_from_settings(None, None)
}

/// Wasm linear memory currently claimed, in bytes.
#[cfg(target_arch = "wasm32")]
#[must_use]
pub fn heap_bytes() -> u64 {
    use wasm_bindgen::JsCast;
    wasm_bindgen::memory()
        .dyn_into::<js_sys::WebAssembly::Memory>()
        .ok()
        .map(|m| js_sys::ArrayBuffer::from(m.buffer()).byte_length())
        .map_or(0, u64::from)
}

/// Wasm linear memory currently claimed, in bytes.
#[cfg(not(target_arch = "wasm32"))]
#[must_use]
pub fn heap_bytes() -> u64 {
    0
}
