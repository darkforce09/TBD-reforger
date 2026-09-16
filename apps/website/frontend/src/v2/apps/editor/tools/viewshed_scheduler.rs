//! Role: drive the engine's viewshed job scheduler from the browser's frame loop.
//! Position: `editor/tools` in the frontend editor.
//! Signals & state: one `requestAnimationFrame` closure at a time, however many placements arrive.
//! Invariants: the scheduling policy — one live job per tool, budgeted batches, cancel on new
//! placement, cap refusals — is the engine's. What lives here is the browser's half of it: the
//! clock, the frame pump, the operator-facing log line, and the object-wash restart that follows a
//! completed terrain disc.

// The wasm host installs these services; on native the engine's own defaults apply and nothing
// here has a caller.
#![allow(dead_code)]

use website_map_engine::editing::tools::viewshed_scheduler::{install_host, SchedulerHost};

#[cfg(target_arch = "wasm32")]
thread_local! {
    /// One rAF closure at a time, however many placements arrive.
    static PUMPING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Milliseconds from an arbitrary epoch — the clock every batch budget is measured against.
fn now_ms() -> f64 {
    #[cfg(target_arch = "wasm32")]
    {
        js_sys::Date::now()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0.0, |d| d.as_secs_f64() * 1000.0)
    }
}

/// Surface a cap refusal to the operator: which cap, and the measured value that broke it.
fn report_refusal(msg: &str) {
    leptos::logging::warn!("{}", msg);
}

/// The terrain disc is complete and published. Restart the object wash so its merged upload is over
/// the FINAL raster: the canvas starts one right after the placement, which — the disc being sliced
/// — began over the first batch only. Re-starting retires that generation and rebuilds the pass over
/// the finished terrain.
fn on_terrain_finished() {
    #[cfg(target_arch = "wasm32")]
    super::los_world_wasm::start_object_wash();
}

/// Install this host's services into the engine's scheduler. Called once at boot.
pub fn install_scheduler_host() {
    install_host(SchedulerHost {
        now_ms,
        report_refusal,
        request_pump: start_pump,
        on_terrain_finished,
    });
}

/// This module's OWN self-rescheduling `requestAnimationFrame` closure, started when the engine asks
/// for frames and dropped as soon as no job is live. One at a time, however many placements arrive.
#[cfg(target_arch = "wasm32")]
type PumpClosure =
    std::rc::Rc<std::cell::RefCell<Option<wasm_bindgen::prelude::Closure<dyn FnMut()>>>>;

#[cfg(target_arch = "wasm32")]
fn start_pump() {
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;
    use wasm_bindgen::prelude::Closure;
    use wasm_bindgen::JsCast;

    use website_map_engine::editing::tools::viewshed_scheduler::pump_terrain_once;

    if PUMPING.with(Cell::get) {
        return;
    }
    PUMPING.with(|p| p.set(true));
    let f: PumpClosure = Rc::new(RefCell::new(None));
    let g = f.clone();
    *g.borrow_mut() = Some(Closure::wrap(Box::new(move || {
        if !pump_terrain_once() {
            PUMPING.with(|p| p.set(false));
            f.borrow_mut().take(); // drop the loop closure — no further frames
            return;
        }
        let cb_ref = f.borrow();
        if let (Some(cb), Some(win)) = (cb_ref.as_ref(), web_sys::window()) {
            let _ = win.request_animation_frame(cb.as_ref().unchecked_ref());
        }
    }) as Box<dyn FnMut()>));
    let cb_ref = g.borrow();
    if let (Some(cb), Some(win)) = (cb_ref.as_ref(), web_sys::window()) {
        let _ = win.request_animation_frame(cb.as_ref().unchecked_ref());
    } else {
        PUMPING.with(|p| p.set(false));
    }
}

/// Native builds have no frame loop — the engine drains a terrain job on submit, so there is nothing
/// to pump. Kept as a peer of the wasm arm so the submit path reads the same on both.
#[cfg(not(target_arch = "wasm32"))]
fn start_pump() {}
