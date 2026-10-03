//! Editor frame loop readouts, debug bridges, and the registry session cache.
//!
//! **Role:** the Mission Creator's per-frame readouts (map scale, the debug HUD) on the shared
//! map seam's frame pump, the browser diagnostics hooks, and the registry and compatibility cache
//! kept across editor mounts.
//! **Position:** called by the canvas mount's boot tasks; canvas sizing and the pump itself live
//! in [`crate::foundation::map_view`].
//! **Signals & state:** the scale and HUD signals the pump hook writes; the thread-local registry
//! cache.
//! **Invariants:** the scale signal is written only when its readout text changes, never once
//! per frame.

use leptos::prelude::*;

/// Starts the damage-driven editor render loop.
#[cfg(target_arch = "wasm32")]
pub(crate) fn start_raf(
    engine: std::rc::Rc<std::cell::RefCell<Option<map_renderer::engine::RenderEngine>>>,
    disposed: std::sync::Arc<std::sync::atomic::AtomicBool>,
    debug_hud: RwSignal<String>,
    scale_mpp: RwSignal<f64>,
) {
    let mut frames_at_sample = 0u32;
    let mut last_sample = 0.0f64;
    let mut last_scale_text = String::new();

    crate::foundation::map_view::frame_pump::start_frame_pump(
        engine,
        disposed,
        move |e, frames| {
            crate::workspaces::editor::input::tools::los_world_wasm::tick_object_wash(e);
            {
                let mpp = crate::workspaces::editor::ui::docks::toolbelt::m_per_px(e.zoom());
                let text = crate::workspaces::editor::ui::docks::toolbelt::format_m_per_px(mpp);
                if text != last_scale_text {
                    last_scale_text = text;
                    scale_mpp.set(mpp);
                }
            }
            {
                let now = js_sys::Date::now();
                if last_sample == 0.0 {
                    last_sample = now;
                } else if now - last_sample >= 1000.0 {
                    let window = frames.wrapping_sub(frames_at_sample);
                    let fps = (f64::from(window) * 1000.0 / (now - last_sample)).round();
                    let stats: serde_json::Value =
                        serde_json::from_str(&e.stats()).unwrap_or_default();
                    let chunks = stats["chunks"].as_u64().unwrap_or(0);
                    let glyphs = stats["tree_glyphs"].as_u64().unwrap_or(0);
                    let rf_ms = stats["render_cpu_ms_ema"].as_f64().unwrap_or(0.0);
                    let rf_eq = if rf_ms > 0.0 { 1000.0 / rf_ms } else { 0.0 };
                    debug_hud.set(format!(
                        "z {:.2} · c{chunks} · glyph {glyphs} · {fps:.0} FPS · rf {rf_ms:.2}ms ({rf_eq:.0} eq){}{}",
                        e.zoom(),
                        crate::workspaces::editor::input::tools::los_world_wasm::hud_suffix(),
                        map_asset_loading::live_memory_budget::hud_suffix()
                    ));
                    frames_at_sample = frames;
                    last_sample = now;
                }
            }
        },
    );
}

/// Exposes GPU readback self-checks to the browser gate.
#[cfg(target_arch = "wasm32")]
pub(crate) fn register_self_checks(
    engine: std::rc::Rc<std::cell::RefCell<Option<map_renderer::engine::RenderEngine>>>,
) {
    use wasm_bindgen::prelude::*;

    let obj = js_sys::Object::new();

    let calibration = {
        let engine = engine.clone();
        Closure::wrap(Box::new(move || {
            engine
                .borrow()
                .as_ref()
                .map(map_render_diagnostics::readback::calibration::calibration_self_check)
                .unwrap_or_else(|| js_sys::Promise::reject(&JsValue::from_str("engine not ready")))
        }) as Box<dyn FnMut() -> js_sys::Promise>)
    };
    let texture = {
        let engine = engine.clone();
        Closure::wrap(Box::new(move || {
            engine
                .borrow()
                .as_ref()
                .map(map_render_diagnostics::readback::texture::texture_self_check)
                .unwrap_or_else(|| js_sys::Promise::reject(&JsValue::from_str("engine not ready")))
        }) as Box<dyn FnMut() -> js_sys::Promise>)
    };

    let _ = js_sys::Reflect::set(
        &obj,
        &JsValue::from_str("calibration"),
        calibration.as_ref(),
    );
    let _ = js_sys::Reflect::set(&obj, &JsValue::from_str("texture"), texture.as_ref());
    let bench = {
        let engine = engine.clone();
        Closure::wrap(Box::new(move |n: f64| {
            engine
                .borrow_mut()
                .as_mut()
                .map(|e| {
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    map_render_diagnostics::benchmark::frame_benchmark::render_bench(
                        e,
                        n.max(1.0) as u32,
                    )
                })
                .unwrap_or_else(|| js_sys::Promise::reject(&JsValue::from_str("engine not ready")))
        }) as Box<dyn FnMut(f64) -> js_sys::Promise>)
    };
    register_engine_diagnostics(&engine, &obj, bench.as_ref());
    if let Some(win) = web_sys::window() {
        let _ = js_sys::Reflect::set(&win, &JsValue::from_str("__selfChecks"), &obj);
        let _ = js_sys::Reflect::set(&win, &JsValue::from_str("__editorBench"), bench.as_ref());
    }
    calibration.forget();
    texture.forget();
    bench.forget();
}

/// The shared render engine cell every browser diagnostics hook reads.
#[cfg(target_arch = "wasm32")]
type EngineCell = std::rc::Rc<std::cell::RefCell<Option<map_renderer::engine::RenderEngine>>>;

/// Sets `name` on `target` to the JavaScript function behind `closure`, kept for the page's
/// lifetime.
#[cfg(target_arch = "wasm32")]
fn set_hook<T: ?Sized + wasm_bindgen::closure::WasmClosure>(
    target: &wasm_bindgen::JsValue,
    name: &str,
    closure: wasm_bindgen::closure::Closure<T>,
) {
    let _ = js_sys::Reflect::set(
        target,
        &wasm_bindgen::JsValue::from_str(name),
        closure.as_ref(),
    );
    closure.forget();
}

/// Adds the remaining engine diagnostics to the browser hooks, each entry named after the
/// diagnostics function or engine method it calls and returning what that call returns.
///
/// The readback self-checks and `readback_rgba` join `window.__selfChecks` and resolve to their
/// JSON reports; the stress pool (`seed_stress`, `clear_stress`) and the compute-cull instruments
/// (`compute_cull_enabled`, `compute_cull_cpu_count`, `compute_cull_gpu_count`,
/// `compute_cull_gpu_sampled`, `compute_cull_cpu_count_for_frustum`,
/// `set_compute_cull_debug_hud`) become properties of the `window.__editorBench` function, which
/// stays callable as before. Before the engine exists a promise entry rejects with
/// "engine not ready", a reading entry returns `undefined` and a command entry does nothing.
#[cfg(target_arch = "wasm32")]
fn register_engine_diagnostics(
    engine: &EngineCell,
    self_checks: &wasm_bindgen::JsValue,
    bench: &wasm_bindgen::JsValue,
) {
    use js_sys::Promise;
    use map_render_diagnostics::benchmark::stress_pool::{clear_stress, seed_stress};
    use map_render_diagnostics::readback::{
        compute_cull::compute_cull_self_check, marquee::marquee_self_check,
        road_centerline::road_centerline_self_check, scene::readback_rgba,
        sea_band::sea_band_self_check, text::text_self_check, tree_glyph::tree_glyph_self_check,
        world_building::world_building_self_check,
    };
    use map_renderer::engine::RenderEngine;
    use wasm_bindgen::prelude::*;

    /// A reading of the engine a hook entry calls with no argument.
    type EngineReading<T> = fn(&RenderEngine) -> T;

    fn not_ready() -> Promise {
        Promise::reject(&JsValue::from_str("engine not ready"))
    }

    let readback_checks: [(&str, EngineReading<Promise>); 7] = [
        ("text_self_check", text_self_check),
        ("world_building_self_check", world_building_self_check),
        ("marquee_self_check", marquee_self_check),
        ("road_centerline_self_check", road_centerline_self_check),
        ("compute_cull_self_check", compute_cull_self_check),
        ("sea_band_self_check", sea_band_self_check),
        ("tree_glyph_self_check", tree_glyph_self_check),
    ];
    for (name, check) in readback_checks {
        let engine = engine.clone();
        let hook = move || engine.borrow().as_ref().map_or_else(not_ready, check);
        set_hook(
            self_checks,
            name,
            Closure::wrap(Box::new(hook) as Box<dyn FnMut() -> Promise>),
        );
    }
    let readback = {
        let engine = engine.clone();
        move |x_px: u32, y_px: u32| match engine.borrow().as_ref() {
            Some(e) => readback_rgba(e, x_px, y_px),
            None => not_ready(),
        }
    };
    set_hook(
        self_checks,
        "readback_rgba",
        Closure::wrap(Box::new(readback) as Box<dyn FnMut(u32, u32) -> Promise>),
    );

    let seed = {
        let engine = engine.clone();
        move |n: u32, seed: u32| {
            if let Some(e) = engine.borrow_mut().as_mut() {
                seed_stress(e, n, seed);
            }
        }
    };
    set_hook(
        bench,
        "seed_stress",
        Closure::wrap(Box::new(seed) as Box<dyn FnMut(u32, u32)>),
    );
    let clear = {
        let engine = engine.clone();
        move || {
            if let Some(e) = engine.borrow_mut().as_mut() {
                clear_stress(e);
            }
        }
    };
    set_hook(
        bench,
        "clear_stress",
        Closure::wrap(Box::new(clear) as Box<dyn FnMut()>),
    );
    let debug_hud = {
        let engine = engine.clone();
        move |on: bool| {
            if let Some(e) = engine.borrow_mut().as_mut() {
                e.set_compute_cull_debug_hud(on);
            }
        }
    };
    set_hook(
        bench,
        "set_compute_cull_debug_hud",
        Closure::wrap(Box::new(debug_hud) as Box<dyn FnMut(bool)>),
    );

    let flags: [(&str, EngineReading<bool>); 2] = [
        ("compute_cull_enabled", RenderEngine::compute_cull_enabled),
        (
            "compute_cull_gpu_sampled",
            RenderEngine::compute_cull_gpu_sampled,
        ),
    ];
    for (name, read) in flags {
        let engine = engine.clone();
        let hook = move || engine.borrow().as_ref().map(read);
        set_hook(
            bench,
            name,
            Closure::wrap(Box::new(hook) as Box<dyn FnMut() -> Option<bool>>),
        );
    }
    let counts: [(&str, EngineReading<u32>); 2] = [
        (
            "compute_cull_cpu_count",
            RenderEngine::compute_cull_cpu_count,
        ),
        (
            "compute_cull_gpu_count",
            RenderEngine::compute_cull_gpu_count,
        ),
    ];
    for (name, read) in counts {
        let engine = engine.clone();
        let hook = move || engine.borrow().as_ref().map(read);
        set_hook(
            bench,
            name,
            Closure::wrap(Box::new(hook) as Box<dyn FnMut() -> Option<u32>>),
        );
    }
    let frustum = {
        let engine = engine.clone();
        move |min_x: f64, min_y: f64, max_x: f64, max_y: f64| {
            engine
                .borrow()
                .as_ref()
                .map(|e| e.compute_cull_cpu_count_for_frustum(min_x, min_y, max_x, max_y))
        }
    };
    set_hook(
        bench,
        "compute_cull_cpu_count_for_frustum",
        Closure::wrap(Box::new(frustum) as Box<dyn FnMut(f64, f64, f64, f64) -> Option<u32>>),
    );
}

/// Exposes editor camera state to browser diagnostics.
#[cfg(target_arch = "wasm32")]
pub(crate) fn register_editor_cam(
    engine: std::rc::Rc<std::cell::RefCell<Option<map_renderer::engine::RenderEngine>>>,
    map_host: map_streaming_host::HostHandle,
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
                e.on_camera_changed();
            }
            map_streaming_host::flush_viewport(map_host.clone(), engine.clone());
        }
    }) as Box<dyn FnMut(f64, f64, f64)>);

    if let Some(win) = web_sys::window() {
        let _ = js_sys::Reflect::set(&win, &JsValue::from_str("__editorCam"), cam.as_ref());
        let _ = js_sys::Reflect::set(&win, &JsValue::from_str("__editorCamSet"), cam_set.as_ref());
    }
    cam.forget();
    cam_set.forget();
}

/// Exposes slot rendering statistics to browser diagnostics.
#[cfg(target_arch = "wasm32")]
pub(crate) fn register_slot_stats(
    engine: std::rc::Rc<std::cell::RefCell<Option<map_renderer::engine::RenderEngine>>>,
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

/// Sets both catalog failure states and the retry signal.
pub(crate) fn mark_registry_fetch_failed(
    catalog: RwSignal<crate::workspaces::editor::arsenal::asset_catalog::CatalogState>,
    vehicle_catalog: RwSignal<crate::workspaces::editor::arsenal::asset_catalog::CatalogState>,
    registry_failed: RwSignal<bool>,
) {
    use crate::workspaces::editor::arsenal::asset_catalog::CatalogState;
    catalog.set(CatalogState::Failed);
    vehicle_catalog.set(CatalogState::Failed);
    registry_failed.set(true);
}

/// Caches registry and compatibility data across editor mounts.
pub(crate) mod registry_session {
    use std::cell::RefCell;
    use std::collections::HashMap;

    use crate::foundation::transport::dto::RegistryItem;
    use crate::workspaces::editor::arsenal::rules::{CargoRow, CompatFeed};

    struct CachedCompat {
        feed: CompatFeed,
        cargo: HashMap<String, Vec<CargoRow>>,
    }

    thread_local! {
        static REGISTRY: RefCell<Option<Vec<RegistryItem>>> = const { RefCell::new(None) };
        static COMPAT: RefCell<Option<CachedCompat>> = const { RefCell::new(None) };
    }

    /// Reports whether registry data is missing from the session cache.
    #[must_use]
    pub fn must_fetch_registry() -> bool {
        REGISTRY.with(|c| c.borrow().is_none())
    }

    /// Reports whether compatibility data is missing from the session cache.
    #[must_use]
    pub fn must_fetch_compat() -> bool {
        COMPAT.with(|c| c.borrow().is_none())
    }

    /// Returns cached registry rows when present.
    #[must_use]
    pub fn cached_registry() -> Option<Vec<RegistryItem>> {
        REGISTRY.with(|c| c.borrow().clone())
    }

    /// Stores registry rows for subsequent editor mounts.
    pub fn store_registry(items: Vec<RegistryItem>) {
        REGISTRY.with(|c| *c.borrow_mut() = Some(items));
    }

    /// Returns cached compatibility data when present.
    #[must_use]
    pub fn cached_compat() -> Option<(CompatFeed, HashMap<String, Vec<CargoRow>>)> {
        COMPAT.with(|c| {
            c.borrow()
                .as_ref()
                .map(|hit| (hit.feed.clone(), hit.cargo.clone()))
        })
    }

    /// Stores compatibility data for subsequent editor mounts.
    pub fn store_compat(feed: CompatFeed, cargo: HashMap<String, Vec<CargoRow>>) {
        COMPAT.with(|c| *c.borrow_mut() = Some(CachedCompat { feed, cargo }));
    }

    /// Clears the cache before an isolated test.
    #[cfg(test)]
    pub fn clear_for_test() {
        REGISTRY.with(|c| *c.borrow_mut() = None);
        COMPAT.with(|c| *c.borrow_mut() = None);
    }
}
