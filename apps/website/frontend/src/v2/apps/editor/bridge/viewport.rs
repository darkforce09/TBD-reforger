//! Editor viewport sizing, frame loop, and debug bridges.
#![allow(dead_code)]

use leptos::prelude::*;

/// Converts CSS dimensions and device pixel ratio to canvas pixels.
#[cfg(target_arch = "wasm32")]
pub(crate) fn device_size(css_w: f64, css_h: f64, dpr: f64) -> (u32, u32) {
    let r = |v: f64| ((v * dpr + 0.5).floor().max(1.0)) as u32;
    (r(css_w), r(css_h))
}

/// Starts the damage-driven editor render loop.
#[cfg(target_arch = "wasm32")]
pub(crate) fn start_raf(
    engine: std::rc::Rc<
        std::cell::RefCell<Option<website_map_engine::frame::engine::RenderEngine>>,
    >,
    disposed: std::sync::Arc<std::sync::atomic::AtomicBool>,
    debug_hud: RwSignal<String>,
    scale_mpp: RwSignal<f64>,
) {
    use website_map_engine::frame::RafPump;

    let mut frames_at_sample = 0u32;
    let mut last_sample = 0.0f64;
    let mut last_scale_text = String::new();

    RafPump::new(engine, disposed)
        .after_frame(move |e, frames| {
            crate::v2::apps::editor::input::tools::los_world_wasm::tick_object_wash(e);
            {
                let mpp = crate::v2::apps::editor::ui::docks::toolbelt::m_per_px(e.zoom());
                let text = crate::v2::apps::editor::ui::docks::toolbelt::format_m_per_px(mpp);
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
                        crate::v2::apps::editor::input::tools::los_world_wasm::hud_suffix(),
                        website_map_engine::streaming::memory::budget::hud_suffix()
                    ));
                    frames_at_sample = frames;
                    last_sample = now;
                }
            }
        })
        .start();
}

/// Exposes GPU readback self-checks to the browser gate.
#[cfg(target_arch = "wasm32")]
pub(crate) fn register_self_checks(
    engine: std::rc::Rc<
        std::cell::RefCell<Option<website_map_engine::frame::engine::RenderEngine>>,
    >,
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
    calibration.forget();
    texture.forget();
    bench.forget();
}

/// Exposes editor camera state to browser diagnostics.
#[cfg(target_arch = "wasm32")]
pub(crate) fn register_editor_cam(
    engine: std::rc::Rc<
        std::cell::RefCell<Option<website_map_engine::frame::engine::RenderEngine>>,
    >,
    map_host: website_map_engine::streaming::host::HostHandle,
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
            website_map_engine::streaming::host::flush_viewport(map_host.clone(), engine.clone());
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
    engine: std::rc::Rc<
        std::cell::RefCell<Option<website_map_engine::frame::engine::RenderEngine>>,
    >,
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
    catalog: RwSignal<crate::v2::apps::editor::arsenal::asset_catalog::CatalogState>,
    vehicle_catalog: RwSignal<crate::v2::apps::editor::arsenal::asset_catalog::CatalogState>,
    registry_failed: RwSignal<bool>,
) {
    use crate::v2::apps::editor::arsenal::asset_catalog::CatalogState;
    catalog.set(CatalogState::Failed);
    vehicle_catalog.set(CatalogState::Failed);
    registry_failed.set(true);
}

/// Caches registry and compatibility data across editor mounts.
pub(crate) mod registry_session {
    use std::cell::RefCell;
    use std::collections::HashMap;

    use crate::v2::apps::editor::arsenal::rules::{CargoRow, CompatFeed};
    use crate::v2::core::api::dto::RegistryItem;

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
