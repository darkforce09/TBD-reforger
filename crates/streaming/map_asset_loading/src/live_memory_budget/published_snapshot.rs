//! **Role:** the live ledger's readouts: the debug HUD tail and the `window.__t9386` snapshot.
//! **Position:** `live_memory_budget` in `map_asset_loading`; the accounting calls publish after
//! every change, the Mission Creator's frame pump reads the HUD tail.
//! **Signals & state:** writes the `window.__t9386` global on wasm32; nothing on native.
//! **Invariants:** the snapshot's keys are the asset names of
//! `map_streaming_model::memory_budget::Asset::name`.

use super::*;
#[cfg(target_arch = "wasm32")]
use map_streaming_model::memory_budget::Asset;

/// The live ledger's debug-HUD tail. See
/// [`map_streaming_model::memory_budget::Ledger::hud_suffix`].
#[must_use]
pub fn hud_suffix() -> String {
    with_ledger(|l| l.hud_suffix())
}

/// Install the live ledger's snapshot as `window.__t9386`: budget, held, peak, heap, the
/// satellite floor and one row per asset.
#[cfg(target_arch = "wasm32")]
pub fn publish() {
    use wasm_bindgen::JsValue;
    let Some(win) = web_sys::window() else {
        return;
    };
    let obj = js_sys::Object::new();
    let set = |o: &js_sys::Object, k: &str, v: f64| {
        let _ = js_sys::Reflect::set(o, &JsValue::from_str(k), &JsValue::from_f64(v));
    };
    with_ledger(|l| {
        set(&obj, "budget", l.budget() as f64);
        set(&obj, "held", l.held_total() as f64);
        set(&obj, "peak", l.total_peak() as f64);
        set(&obj, "heap", heap_bytes() as f64);
        set(
            &obj,
            "satFloor",
            l.satellite_floor().map_or(-1.0, |f| f as f64),
        );
        set(&obj, "satRaised", f64::from(l.satellite_raised()));
        let assets = js_sys::Object::new();
        for a in Asset::ALL {
            let e = l.entry(a);
            let row = js_sys::Object::new();
            set(&row, "held", e.held as f64);
            set(&row, "peak", e.peak as f64);
            set(&row, "growth", e.growth as f64);
            let _ = js_sys::Reflect::set(&assets, &JsValue::from_str(a.name()), &row);
        }
        let _ = js_sys::Reflect::set(&obj, &JsValue::from_str("assets"), &assets);
    });
    let _ = js_sys::Reflect::set(&win, &JsValue::from_str("__t9386"), &obj);
}

/// A native build has no page to publish to.
#[cfg(not(target_arch = "wasm32"))]
pub fn publish() {}
