//! Stats for the ORBAT manager.

use super::*;

#[cfg(target_arch = "wasm32")]
/// Publishes visible-row counts for diagnostics.
pub(super) fn set_orbat_stats(total: usize, rendered: usize) {
    use wasm_bindgen::JsValue;
    let Some(win) = web_sys::window() else {
        return;
    };
    let stats = match js_sys::Reflect::get(&win, &JsValue::from_str("__outlinerStats")) {
        Ok(v) if v.is_object() => v,
        _ => {
            let o = js_sys::Object::new();
            let _ = js_sys::Reflect::set(&win, &JsValue::from_str("__outlinerStats"), &o);
            o.into()
        }
    };
    let entry = js_sys::Object::new();
    let set = |k: &str, n: usize| {
        let _ = js_sys::Reflect::set(&entry, &JsValue::from_str(k), &JsValue::from_f64(n as f64));
    };
    set("total", total);
    set("rendered", rendered);
    set("threshold", VIRTUAL_SLOT_THRESHOLD);
    let _ = js_sys::Reflect::set(&stats, &JsValue::from_str("orbat"), &entry);
}

#[cfg(not(target_arch = "wasm32"))]
/// Publishes visible-row counts for diagnostics.
pub(super) fn set_orbat_stats(_total: usize, _rendered: usize) {}
