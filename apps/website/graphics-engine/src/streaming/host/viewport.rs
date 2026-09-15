//! Role: viewport.
//! Position: `streaming/host` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use super::*;

/// Set camera gesture.
pub fn set_camera_gesture(active: bool) {
    CAMERA_GESTURE.with(|g| g.set(active));
}

/// Camera gesture active.
pub(super) fn camera_gesture_active() -> bool {
    CAMERA_GESTURE.with(Cell::get)
}

/// Immediate viewport refresh (smoke probe / tests). Also used by the debounced settle path. Runs several passes so a soft-fail chunk re-fetch + ingest budget can settle.
pub fn flush_viewport(host: HostHandle, engine: EngineHandle) {
    wasm_bindgen_futures::spawn_local(async move {
        let no_progress = |_: crate::streaming::bridge::progress::BootEvent| {};
        for _ in 0..6 {
            let mut h = {
                let mut g = host.borrow_mut();
                match g.take() {
                    Some(h) => h,
                    None => return,
                }
            };
            let zoom = engine.borrow().as_ref().map(|e| e.zoom()).unwrap_or(-2.0);

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
            if let Some(e) = engine.borrow().as_ref() {
                publish_engine(&h.bridge, e);
            }
            *host.borrow_mut() = Some(h);
            if !did_world && !did_forest {
                break;
            }
        }
    });
}

/// Canonical settle debounce ms value.
pub(super) const SETTLE_DEBOUNCE_MS: f64 = 120.0;

/// Canonical settle max latency ms value.
pub(super) const SETTLE_MAX_LATENCY_MS: f64 = 250.0;

/// Schedule camera settle.
pub fn schedule_camera_settle(host: HostHandle, engine: EngineHandle) {
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
