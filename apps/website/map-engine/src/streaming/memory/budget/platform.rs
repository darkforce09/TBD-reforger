//! Role: platform.
//! Position: `streaming/memory/budget` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use super::*;

/// The budget for this boot, in bytes: `?memBudgetMb=NNN`, else `window.__memBudgetMb`, else [`DEFAULT_BUDGET_MB`].
#[cfg(target_arch = "wasm32")]
#[must_use]
pub fn configured_budget_bytes() -> u64 {
    let from_param = web_sys::window()
        .and_then(|w| w.location().search().ok())
        .and_then(|s| web_sys::UrlSearchParams::new_with_str(&s).ok())
        .and_then(|p| p.get("memBudgetMb"))
        .and_then(|v| v.trim().parse::<u64>().ok());
    let from_global = || {
        web_sys::window()
            .and_then(|w| {
                js_sys::Reflect::get(&w, &wasm_bindgen::JsValue::from_str("__memBudgetMb")).ok()
            })
            .and_then(|v| v.as_f64())
            .filter(|v| *v > 0.0 && v.is_finite())
            .map(|v| v as u64)
    };
    from_param
        .or_else(from_global)
        .filter(|mb| *mb > 0)
        .unwrap_or(DEFAULT_BUDGET_MB)
        .saturating_mul(MIB)
}

/// The budget for this boot, in bytes: `?memBudgetMb=NNN`, else `window.__memBudgetMb`, else [`DEFAULT_BUDGET_MB`].
#[cfg(not(target_arch = "wasm32"))]
#[must_use]
pub fn configured_budget_bytes() -> u64 {
    DEFAULT_BUDGET_MB * MIB
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
