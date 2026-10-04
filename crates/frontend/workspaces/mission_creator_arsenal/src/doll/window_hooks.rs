//! The `window.__arsenalDoll` hooks: the browser smoke's proof surface over the live doll.
//!
//! **Role:** publishes four closures on `window.__arsenalDoll` — `backend()` (the renderer's
//! backend name), `anchor(idx)` (a region's anchor in css px), `pick(x, y)` (a CPU region pick)
//! and `doll_self_check()` (a promise of the engine's readback check) — each reading straight off
//! the live engine.
//! **Position:** called once by `ArsenalDoll` in the parent `doll.rs` after the engine is
//! created; read by the Arsenal browser smoke in
//! `tools/browser_testing/browser_gate_suites/src/editor_smoke_tests/arsenal.rs`.
//! **Signals & state:** the closures share the mounted component's engine handle and are leaked
//! with `forget`, so they stay callable for the page's lifetime.
//! **Invariants:** a hook reads the engine through a short `borrow()` and never holds it across
//! a spawn; with no engine, `backend`, `anchor` and `pick` return `null` and the self-check
//! promise rejects with "engine not ready".

use wasm_bindgen::prelude::*;

use super::EngineHandle;

/// `window.__arsenalDoll` — the smoke's proof surface: backend string, active-anchor px, a CPU
/// pick at css px and the `doll_self_check` readback (all straight off the live engine).
pub(super) fn register_doll_hooks(engine: &EngineHandle) {
    let obj = js_sys::Object::new();
    let backend = Closure::wrap(Box::new({
        let engine = engine.clone();
        move || -> JsValue {
            engine
                .borrow()
                .as_ref()
                .map(|e| JsValue::from_str(e.backend()))
                .unwrap_or(JsValue::NULL)
        }
    }) as Box<dyn FnMut() -> JsValue>);
    let anchor = Closure::wrap(Box::new({
        let engine = engine.clone();
        move |idx: i32| -> JsValue {
            engine
                .borrow()
                .as_ref()
                .map(|e| {
                    let arr = js_sys::Array::new();
                    if let Some((x, y)) = e.anchor_px(idx) {
                        arr.push(&JsValue::from_f64(x));
                        arr.push(&JsValue::from_f64(y));
                    }
                    arr.into()
                })
                .unwrap_or(JsValue::NULL)
        }
    }) as Box<dyn FnMut(i32) -> JsValue>);
    let pick = Closure::wrap(Box::new({
        let engine = engine.clone();
        move |x: f64, y: f64| -> JsValue {
            engine
                .borrow()
                .as_ref()
                .map(|e| JsValue::from_f64(f64::from(e.pick_region(x, y))))
                .unwrap_or(JsValue::NULL)
        }
    }) as Box<dyn FnMut(f64, f64) -> JsValue>);
    let self_check = Closure::wrap(Box::new({
        let engine = engine.clone();
        move || {
            // The check is started under one short borrow; the borrow ends before the promise
            // spawns the future that awaits it.
            let check = engine.borrow().as_ref().map(|e| e.self_check());
            match check {
                Some(check) => wasm_bindgen_futures::future_to_promise(async move {
                    check
                        .await
                        .map(|readout| JsValue::from_str(&readout))
                        .map_err(|error| JsValue::from_str(&error.to_string()))
                }),
                None => js_sys::Promise::reject(&JsValue::from_str("engine not ready")),
            }
        }
    }) as Box<dyn FnMut() -> js_sys::Promise>);
    let _ = js_sys::Reflect::set(&obj, &JsValue::from_str("backend"), backend.as_ref());
    let _ = js_sys::Reflect::set(&obj, &JsValue::from_str("anchor"), anchor.as_ref());
    let _ = js_sys::Reflect::set(&obj, &JsValue::from_str("pick"), pick.as_ref());
    let self_check_key = JsValue::from_str("doll_self_check");
    let _ = js_sys::Reflect::set(&obj, &self_check_key, self_check.as_ref());
    backend.forget();
    anchor.forget();
    pick.forget();
    self_check.forget();
    if let Some(win) = web_sys::window() {
        let _ = js_sys::Reflect::set(&win, &JsValue::from_str("__arsenalDoll"), &obj);
    }
}
