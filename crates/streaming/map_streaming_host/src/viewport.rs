//! **Role:** the camera settle: the gesture flag, the debounced settle schedule and the viewport
//! refresh passes that run the world, forest, relief and label loaders for the new view.
//! **Position:** `viewport` in `map_streaming_host`; the Mission Creator's pointer and wheel
//! gestures and the fly-to call it, and the `__editorCamSet` harness gate flushes directly.
//! **Signals & state:** the thread-local gesture flag of [`crate::render_context`]; the host's
//! settle timer and deadline; one spawned pass sequence per flush.
//! **Invariants:** a settle runs at most [`SETTLE_DEBOUNCE_MS`] after the last camera change and
//! at most [`SETTLE_MAX_LATENCY_MS`] after the first; a flush stops at the first pass that loads
//! nothing new, after six at most.

use std::cell::Cell;

use map_asset_loading::asset_statistics::publish_engine;
use map_asset_loading::browser_asset_sink::BrowserAssetSinkHandle;
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;

use crate::map_host::HostHandle;
use crate::render_context::CAMERA_GESTURE;

/// Set camera gesture.
pub fn set_camera_gesture(active: bool) {
    CAMERA_GESTURE.with(|g| g.set(active));
}

/// Camera gesture active.
pub(crate) fn camera_gesture_active() -> bool {
    CAMERA_GESTURE.with(Cell::get)
}

/// Immediate viewport refresh (smoke probe / tests). Also used by the debounced settle path. Runs several passes so a soft-fail chunk re-fetch + ingest budget can settle.
pub fn flush_viewport(host: HostHandle, engine: BrowserAssetSinkHandle) {
    wasm_bindgen_futures::spawn_local(async move {
        let no_progress = |_: map_streaming_model::boot_progress::BootEvent| {};
        for _ in 0..6 {
            let mut h = {
                let mut g = host.borrow_mut();
                match g.take() {
                    Some(h) => h,
                    None => return,
                }
            };
            let zoom = engine.borrow().sink().map(|e| e.zoom()).unwrap_or(-2.0);

            let gesture = camera_gesture_active();
            if !gesture {
                h.dem.sync(&engine, zoom);
            }

            let did_world = h.world.run_viewport(&engine, &h.bridge, &no_progress).await;

            let did_forest = if gesture && !h.forest.is_uploaded() {
                false
            } else {
                h.forest
                    .run_viewport(&engine, &h.bridge, &no_progress)
                    .await
            };

            {
                let prefs = (h.preferences.world_layers)();
                h.labels.push(&engine, zoom, &prefs);
            }
            if let Some(e) = engine.borrow().sink() {
                publish_engine(&h.bridge, &e.stats_json());
            }
            *host.borrow_mut() = Some(h);
            if !did_world && !did_forest {
                break;
            }
        }
    });
}

/// Canonical settle debounce ms value.
pub(crate) const SETTLE_DEBOUNCE_MS: f64 = 120.0;

/// Canonical settle max latency ms value.
pub(crate) const SETTLE_MAX_LATENCY_MS: f64 = 250.0;

/// Schedule camera settle.
pub fn schedule_camera_settle(host: HostHandle, engine: BrowserAssetSinkHandle) {
    let Some(win) = web_sys::window() else {
        return;
    };
    let (timer_slot, deadline_slot) = {
        let g = host.borrow();
        let Some(h) = g.as_ref() else {
            return;
        };
        (h.settle_timer.clone(), h.settle_deadline.clone())
    };
    let now = js_sys::Date::now();

    if deadline_slot.get() <= 0.0 {
        deadline_slot.set(now + SETTLE_MAX_LATENCY_MS);
    }
    let delay = (deadline_slot.get() - now).clamp(0.0, SETTLE_DEBOUNCE_MS);
    if let Some(id) = timer_slot.get() {
        win.clear_timeout_with_handle(id);
    }
    let host2 = host.clone();
    let eng2 = engine.clone();
    let slot2 = timer_slot.clone();
    let deadline2 = deadline_slot.clone();
    let cb = Closure::once_into_js(move || {
        slot2.set(None);
        deadline2.set(0.0);
        flush_viewport(host2, eng2);
    });
    #[allow(clippy::cast_possible_truncation)]
    if let Ok(id) = win.set_timeout_with_callback_and_timeout_and_arguments_0(
        cb.as_ref().unchecked_ref(),
        delay as i32,
    ) {
        timer_slot.set(Some(id));
    }
}
