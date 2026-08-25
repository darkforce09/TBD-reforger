//! T-934.12 — the Mission Creator VIEWPORT / frame-timing belt, split out of `mission_editor.rs`
//! (Phase B; audit §4 Phase 1 item 4): [`device_size`] (CSS→device-pixel rounding), [`start_raf`]
//! (the rAF render loop with the ~1 Hz debug-HUD sample and the T-670 guarded scale publish), the
//! three window-gate registrars ([`register_self_checks`] — which also installs the T-173
//! `__editorBench` — [`register_editor_cam`], [`register_slot_stats`]), the T-750
//! [`mark_registry_fetch_failed`] failure writer and the T-245 [`registry_session`] SPA-session
//! cache.
//!
//! Bodies are byte-identical to their `mission_editor.rs` originals, and `mission_editor`
//! re-exports every name here, so the page's bare call sites and the evacuated pins' `super::…`
//! imports (`t245_registry_session`, `t750_registry_fetch_failure_signal`, `t670_scale_signal`)
//! all keep their exact spelling. ITEM ORDER IS LOAD-BEARING for the Class-R scrubs:
//! `registry_session` holds this file's only `#[cfg(test)]` (its `clear_for_test` helper) and
//! sits LAST, so a whole-file `live_code` — which cuts from the first such literal to EOF —
//! keeps every item above it (`t670`'s `start_raf` pins, `t750`'s body pin).
// The same gate `mission_editor.rs` carries, for the same reason: everything above
// `mark_registry_fetch_failed` is only reached from `#[cfg(target_arch = "wasm32")]` mount
// closures, and the native build sees the rest as test-pin-only.
#![allow(dead_code)]

use leptos::prelude::*;

/// Round CSS px → device-pixel backing size (≥1), matching the React oracle's `deviceSize`.
#[cfg(target_arch = "wasm32")]
pub(crate) fn device_size(css_w: f64, css_h: f64, dpr: f64) -> (u32, u32) {
    let r = |v: f64| ((v * dpr + 0.5).floor().max(1.0)) as u32;
    (r(css_w), r(css_h))
}

/// The rAF render loop. Each frame renders then polls the device (see `RenderEngine::poll`) so
/// readback `map_async` callbacks drain on the WebGL2-fallback + cull-counter path. (The timer
/// double-map that panicked the 15.0 loop is handled upstream by `disable_frame_timing`.) Stops
/// (and drops itself) once `disposed` is set.
#[cfg(target_arch = "wasm32")]
pub(crate) fn start_raf(
    engine: std::rc::Rc<std::cell::RefCell<Option<map_engine_render::RenderEngine>>>,
    disposed: std::sync::Arc<std::sync::atomic::AtomicBool>,
    debug_hud: RwSignal<String>,
    scale_mpp: RwSignal<f64>,
) {
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::sync::atomic::Ordering;
    use wasm_bindgen::prelude::*;
    use wasm_bindgen::JsCast;

    // T-172 B9 — ~1 Hz debug readout sample (screen-05 bottom-right HUD): zoom, drawn world
    // chunks, tree glyphs, FPS. Counting frames between samples measures real rAF cadence.
    let mut frames = 0u32;
    let mut last_sample = 0.0f64;
    // T-670 — last PUBLISHED scale readout. The camera zoom is only reachable from inside this
    // per-frame closure, so this is the guard that keeps a 60 fps read from becoming a 60 fps
    // Leptos write: the frame formats the scale and calls `set` ONLY when the formatted string
    // differs from what the status bar is already showing. Without it every frame would dirty
    // `scale_mpp`, re-rendering the status bar (and the scale bar) 60×/s for a value that changes a
    // handful of times per wheel gesture and never at all while panning or idle. Empty until the
    // first frame publishes, so the seeded default is replaced as soon as the engine is live.
    let mut last_scale_text = String::new();

    let f: Rc<RefCell<Option<Closure<dyn FnMut()>>>> = Rc::new(RefCell::new(None));
    let g = f.clone();
    *g.borrow_mut() = Some(Closure::wrap(Box::new(move || {
        if disposed.load(Ordering::Relaxed) {
            f.borrow_mut().take(); // drop the loop closure — no further frames
            return;
        }
        // T-631 — the double-panic fix. This was `engine.borrow_mut()`, which PANICS if the cell
        // is already borrowed. When `e.render()` panicked (the observed `createBuffer size too
        // large` → wasm `unreachable`), the abort re-entered the editor while the first panic was
        // unwinding; the next frame's `borrow_mut` then found the cell still held and panicked a
        // SECOND time with "RefCell already borrowed", and that second panic — not the render
        // failure — is what surfaced, burying the real cause. `try_borrow_mut` makes a contended
        // frame a no-op instead of a panic, so a re-entrant borrow can never overwrite the first,
        // true panic. It is also the correct steady-state behaviour: a frame that cannot get the
        // engine simply waits for the next rAF rather than taking the tab down.
        let Ok(mut guard) = engine.try_borrow_mut() else {
            // Contended: skip this frame, keep the loop alive.
            let cb_ref = f.borrow();
            if let (Some(cb), Some(win)) = (cb_ref.as_ref(), web_sys::window()) {
                let _ = win.request_animation_frame(cb.as_ref().unchecked_ref());
            }
            return;
        };
        if let Some(e) = guard.as_mut() {
            let _ = e.render();
            e.poll(); // ★ T-159.15.1: drain readback map_async so the next submit can't double-map
            frames += 1;
            // T-670 — publish the screen scale for the status-bar readout (and, through it, the
            // T-667 scale bar). Read every frame so a wheel-zoom shows on the very next frame
            // rather than waiting up to a second for the ~1 Hz HUD sample below; WRITTEN only when
            // the displayed string changes, so an idle or panning camera costs zero re-renders.
            // The `m_per_px`/`format_m_per_px` pair is `eden_toolbelt`'s — the same conversion the
            // scale bar uses and the same `2^(−deckZoom)` convention T-639's contour ladder takes.
            {
                let mpp = crate::editor::panels::toolbelt::m_per_px(e.zoom());
                let text = crate::editor::panels::toolbelt::format_m_per_px(mpp);
                if text != last_scale_text {
                    last_scale_text = text;
                    scale_mpp.set(mpp);
                }
            }
            {
                // js_sys::Date over web_sys Performance — no extra web-sys feature needed, and
                // ms precision is plenty for a 1 Hz FPS sample.
                let now = js_sys::Date::now();
                if last_sample == 0.0 {
                    last_sample = now;
                } else if now - last_sample >= 1000.0 {
                    let fps = (f64::from(frames) * 1000.0 / (now - last_sample)).round();
                    let stats: serde_json::Value =
                        serde_json::from_str(&e.stats()).unwrap_or_default();
                    let chunks = stats["chunks"].as_u64().unwrap_or(0);
                    let glyphs = stats["tree_glyphs"].as_u64().unwrap_or(0);
                    // T-173 — frame-cost cell: submitted-frame CPU EMA + its FPS-equivalent
                    // (1000/ms — the off-vsync headroom number; rAF FPS stays vsync-capped).
                    let rf_ms = stats["render_cpu_ms_ema"].as_f64().unwrap_or(0.0);
                    let rf_eq = if rf_ms > 0.0 { 1000.0 / rf_ms } else { 0.0 };
                    debug_hud.set(format!(
                        "z {:.2} · c{chunks} · glyph {glyphs} · {fps:.0} FPS · rf {rf_ms:.2}ms ({rf_eq:.0} eq)",
                        e.zoom()
                    ));
                    frames = 0;
                    last_sample = now;
                }
            }
        }
        let cb_ref = f.borrow();
        if let (Some(cb), Some(win)) = (cb_ref.as_ref(), web_sys::window()) {
            let _ = win.request_animation_frame(cb.as_ref().unchecked_ref());
        }
    }) as Box<dyn FnMut()>));
    let cb_ref = g.borrow();
    if let (Some(cb), Some(win)) = (cb_ref.as_ref(), web_sys::window()) {
        let _ = win.request_animation_frame(cb.as_ref().unchecked_ref());
    }
}

/// Expose the byte-exact GPU readback self-checks on `window.__selfChecks` — the map-lane gate the
/// headless driver awaits (see [[wgpu-headless-gpu-verify]]). Both checks are scene-independent
/// (`self_check` renders its own fixed calibration probe scene; `texture_self_check` a synthetic
/// 2×2 texture) and `&self`: they clone their GPU handles up front, so the shared `borrow()` here is
/// released before the async readback runs — no contention with the rAF loop's `borrow_mut` (JS is
/// single-threaded). Each resolves to a JSON string with a `pass` field.
#[cfg(target_arch = "wasm32")]
pub(crate) fn register_self_checks(
    engine: std::rc::Rc<std::cell::RefCell<Option<map_engine_render::RenderEngine>>>,
) {
    use wasm_bindgen::prelude::*;

    let obj = js_sys::Object::new();

    let calibration = {
        let engine = engine.clone();
        Closure::wrap(Box::new(move || {
            engine
                .borrow()
                .as_ref()
                .map(|e| e.self_check())
                .unwrap_or_else(|| js_sys::Promise::reject(&JsValue::from_str("engine not ready")))
        }) as Box<dyn FnMut() -> js_sys::Promise>)
    };
    let texture = {
        let engine = engine.clone();
        Closure::wrap(Box::new(move || {
            engine
                .borrow()
                .as_ref()
                .map(|e| e.texture_self_check())
                .unwrap_or_else(|| js_sys::Promise::reject(&JsValue::from_str("engine not ready")))
        }) as Box<dyn FnMut() -> js_sys::Promise>)
    };

    let _ = js_sys::Reflect::set(
        &obj,
        &JsValue::from_str("calibration"),
        calibration.as_ref(),
    );
    let _ = js_sys::Reflect::set(&obj, &JsValue::from_str("texture"), texture.as_ref());
    // T-173 — `window.__editorBench(n)` off-vsync frame-cost bench (perf gates G-A/G-B): resolves
    // the engine's `render_bench` JSON. Registered here (not in `__selfChecks`) so the perf probe
    // and the operator console both reach it by one name.
    let bench = {
        let engine = engine.clone();
        Closure::wrap(Box::new(move |n: f64| {
            engine
                .borrow_mut()
                .as_mut()
                .map(|e| {
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    e.render_bench(n.max(1.0) as u32)
                })
                .unwrap_or_else(|| js_sys::Promise::reject(&JsValue::from_str("engine not ready")))
        }) as Box<dyn FnMut(f64) -> js_sys::Promise>)
    };
    if let Some(win) = web_sys::window() {
        let _ = js_sys::Reflect::set(&win, &JsValue::from_str("__selfChecks"), &obj);
        let _ = js_sys::Reflect::set(&win, &JsValue::from_str("__editorBench"), bench.as_ref());
    }
    // The harness reads these across the page lifetime; leak them (the engine leaks too).
    calibration.forget();
    texture.forget();
    bench.forget();
}

/// Expose the camera view-state on `window.__editorCam()` for the headless pan smoke (T-159.15.2 /
/// spec P6): a JSON string `{"tx","ty","z","backend"}` read from the `&self` getters `target_x()` /
/// `target_y()` / `zoom()` / `backend()`. (`#[wasm_bindgen(getter)]` fns are plain method calls from
/// Rust.) All are `&self` behind a shared `borrow()`, released before return — no contention with the
/// rAF loop's `borrow_mut` (JS is single-threaded). Registered once the engine is `Some`; the closure
/// leaks like the self-checks. The smoke drives pan via getter deltas (never `unproject_xy`, X-05).
///
/// T-166 — also installs `window.__editorCamSet(tx, ty, z)` so `smoke_fullmap` can Class-R probe
/// tree glyphs at zoom ≥ 0 without relying on CDP `mouseWheel` → DOM `wheel` delivery.
#[cfg(target_arch = "wasm32")]
pub(crate) fn register_editor_cam(
    engine: std::rc::Rc<std::cell::RefCell<Option<map_engine_render::RenderEngine>>>,
    map_host: crate::editor::world_assets::HostHandle,
) {
    use wasm_bindgen::prelude::*;

    let cam = Closure::wrap(Box::new({
        let engine = engine.clone();
        move || -> JsValue {
            engine
                .borrow()
                .as_ref()
                .map(|e| {
                    JsValue::from_str(&format!(
                        r#"{{"tx":{},"ty":{},"z":{},"backend":"{}"}}"#,
                        e.target_x(),
                        e.target_y(),
                        e.zoom(),
                        e.backend()
                    ))
                })
                .unwrap_or_else(|| JsValue::from_str("null"))
        }
    }) as Box<dyn FnMut() -> JsValue>);

    let cam_set = Closure::wrap(Box::new({
        let engine = engine.clone();
        let map_host = map_host.clone();
        move |tx: f64, ty: f64, z: f64| {
            if let Some(e) = engine.borrow_mut().as_mut() {
                e.set_view(tx, ty, z);
                e.on_camera_changed(); // T-172 H5
            }
            // Immediate flush so smoke_fullmap A_trees_on does not race the 120 ms debounce.
            crate::editor::world_assets::flush_viewport(map_host.clone(), engine.clone());
        }
    }) as Box<dyn FnMut(f64, f64, f64)>);

    if let Some(win) = web_sys::window() {
        let _ = js_sys::Reflect::set(&win, &JsValue::from_str("__editorCam"), cam.as_ref());
        let _ = js_sys::Reflect::set(&win, &JsValue::from_str("__editorCamSet"), cam_set.as_ref());
    }
    cam.forget();
    cam_set.forget();
}

/// T-172 B4 — automated slot-lane proof hook: `window.__wgpuSlotStats()` returns the engine's
/// `slot_stats_json` (atlas_ready / slot_len / cluster_mode / …) for the doc smoke.
#[cfg(target_arch = "wasm32")]
pub(crate) fn register_slot_stats(
    engine: std::rc::Rc<std::cell::RefCell<Option<map_engine_render::RenderEngine>>>,
) {
    use wasm_bindgen::prelude::*;

    let stats = Closure::wrap(Box::new(move || -> JsValue {
        engine
            .borrow()
            .as_ref()
            .map(|e| JsValue::from_str(&e.slot_stats_json()))
            .unwrap_or_else(|| JsValue::from_str("null"))
    }) as Box<dyn FnMut() -> JsValue>);
    if let Some(win) = web_sys::window() {
        let _ = js_sys::Reflect::set(&win, &JsValue::from_str("__wgpuSlotStats"), stats.as_ref());
    }
    stats.forget();
}

/// T-750 — apply the `/registry` fetch's terminal failure to the three signals the dock reads.
///
/// Lives OUTSIDE the wasm32 gate on purpose: the Favourites failure arm is host-testable via
/// source pins, and `live_code` kills `#[cfg(target_arch = "wasm32")]` bodies on the native
/// harness — a helper the Err arm *calls* is the only failure write that survives the scrub.
pub(crate) fn mark_registry_fetch_failed(
    catalog: RwSignal<crate::editor::arsenal::asset_catalog::CatalogState>,
    vehicle_catalog: RwSignal<crate::editor::arsenal::asset_catalog::CatalogState>,
    registry_failed: RwSignal<bool>,
) {
    use crate::editor::arsenal::asset_catalog::CatalogState;
    catalog.set(CatalogState::Failed);
    vehicle_catalog.set(CatalogState::Failed);
    registry_failed.set(true);
}

/// T-245 — SPA-session cache for the editor's `/registry` + `/registry/compat` payloads.
/// Survives `MissionEditorPage` remounts inside one SPA session so leaving
/// `/missions/:id/edit` and coming back does **not** re-issue the cold fetches or rebuild
/// the Arsenal compat feed / cargo seed map.
///
/// **T-427** moved the cold path off the unbounded dual dump: registry is assembled from
/// `?limit=` pages, Arsenal edges come from a filtered `edge_type=` list, and cargo seeds
/// come from `?view=cargo_defaults` (server-aggregated — no client walk of ~16k cargo edges).
/// This cache still stores the *assembled* result so remounts stay free.
pub(crate) mod registry_session {
    use std::cell::RefCell;
    use std::collections::HashMap;

    use crate::core::dto::RegistryItem;
    use crate::editor::arsenal::arsenal_rules::{CargoRow, CompatFeed};

    struct CachedCompat {
        feed: CompatFeed,
        cargo: HashMap<String, Vec<CargoRow>>,
    }

    thread_local! {
        static REGISTRY: RefCell<Option<Vec<RegistryItem>>> = const { RefCell::new(None) };
        static COMPAT: RefCell<Option<CachedCompat>> = const { RefCell::new(None) };
    }

    /// `true` when this mount must issue `GET /registry` (no SPA-session hit).
    #[must_use]
    pub fn must_fetch_registry() -> bool {
        REGISTRY.with(|c| c.borrow().is_none())
    }

    /// `true` when this mount must issue `GET /registry/compat` (no SPA-session hit).
    #[must_use]
    pub fn must_fetch_compat() -> bool {
        COMPAT.with(|c| c.borrow().is_none())
    }

    #[must_use]
    pub fn cached_registry() -> Option<Vec<RegistryItem>> {
        REGISTRY.with(|c| c.borrow().clone())
    }

    pub fn store_registry(items: Vec<RegistryItem>) {
        REGISTRY.with(|c| *c.borrow_mut() = Some(items));
    }

    /// Clone of the session-ready compat feed + cargo seed map, if any.
    #[must_use]
    pub fn cached_compat() -> Option<(CompatFeed, HashMap<String, Vec<CargoRow>>)> {
        COMPAT.with(|c| {
            c.borrow()
                .as_ref()
                .map(|hit| (hit.feed.clone(), hit.cargo.clone()))
        })
    }

    pub fn store_compat(feed: CompatFeed, cargo: HashMap<String, Vec<CargoRow>>) {
        COMPAT.with(|c| *c.borrow_mut() = Some(CachedCompat { feed, cargo }));
    }

    #[cfg(test)]
    pub fn clear_for_test() {
        REGISTRY.with(|c| *c.borrow_mut() = None);
        COMPAT.with(|c| *c.borrow_mut() = None);
    }
}
