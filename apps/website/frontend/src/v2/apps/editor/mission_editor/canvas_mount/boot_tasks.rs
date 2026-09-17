//! Starts document restoration and the render engine in parallel.

use super::*;
use leptos::task::spawn_local;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

const TERRAIN_W: f64 = 12_800.0;
const TERRAIN_H: f64 = 12_800.0;
const INITIAL_TARGET: (f64, f64) = (6_400.0, 6_400.0);
const INITIAL_ZOOM: f64 = -2.0;

/// Inputs shared by document restoration and engine startup.
pub(super) struct BootContext {
    pub doc: mission_doc::DocHandle,
    pub mission_id: String,
    pub auth: crate::v2::core::auth::AuthStore,
    pub current_semver: RwSignal<Option<String>>,
    pub conflict: RwSignal<Option<ConflictInfo>>,
    pub boot: RwSignal<BootPhase>,
    pub progress: RwSignal<boot_progress::BootProgress>,
    pub map_disabled: RwSignal<Option<String>>,
    pub engine: Rc<RefCell<Option<website_map_engine::frame::engine::RenderEngine>>>,
    pub map_host: website_map_engine::streaming::host::HostHandle,
    pub dem_grid: website_map_engine::streaming::host::DemGridHandle,
    pub disposed: Arc<AtomicBool>,
    pub restore_settled: Rc<Cell<bool>>,
    pub canvas: web_sys::HtmlCanvasElement,
    pub force_webgl: bool,
    pub width: f64,
    pub height: f64,
    pub dpr0: f64,
    pub debug_hud: RwSignal<String>,
    pub scale_mpp: RwSignal<f64>,
}

/// Starts document and engine boot tasks with a shared readiness handshake.
pub(super) fn start(ctx: BootContext) {
    let BootContext {
        doc,
        mission_id,
        auth,
        current_semver,
        conflict,
        boot,
        progress,
        map_disabled,
        engine,
        map_host,
        dem_grid,
        disposed,
        restore_settled,
        canvas,
        force_webgl,
        width,
        height,
        dpr0,
        debug_hud,
        scale_mpp,
    } = ctx;
    let rect0 = (width, height);
    let persist_ready = Rc::new(Cell::new(false));
    let persist_loaded = Rc::new(Cell::new(false));
    yrs_persist::register_mission_persist(
        doc.clone(),
        mission_id.clone(),
        persist_ready.clone(),
        persist_loaded.clone(),
    );
    let engine_mounted = Rc::new(Cell::new(false));
    let world_ready = Rc::new(Cell::new(false));
    let report: boot_progress::ProgressFn = Rc::new(move |ev| progress.update(|p| p.apply(ev)));
    spawn_local({
        let doc = doc.clone();
        let id = mission_id.clone();
        let ready = persist_ready.clone();
        let loaded = persist_loaded.clone();
        let restore_settled = restore_settled.clone();
        let engine_mounted = engine_mounted.clone();
        let world_ready = world_ready.clone();
        let report = report.clone();
        async move {
            if let Some(blob) = yrs_persist::load_state(&id).await {
                if !blob.is_empty() {
                    let fresh = website_map_engine::data::store::MissionDocCore::new();
                    fresh.set_origin_init(true);
                    let ok = fresh.apply_update(&blob).is_ok();
                    fresh.set_origin_init(false);
                    if ok {
                        *doc.borrow_mut() = Some(fresh);
                        loaded.set(true);
                        mission_history::refresh_hud();
                        mission_history::set_dirty(true);
                    }
                }
            }
            mission_hydrate::hydrate_from_server(
                doc.clone(),
                id.clone(),
                auth,
                loaded.get(),
                current_semver,
                conflict,
                report.clone(),
            )
            .await;
            validation_panel::clear_compile_findings();
            report(boot_progress::BootEvent::Finish(
                boot_progress::BootSeg::Mission,
            ));
            restore_settled.set(true);
            if engine_mounted.get() {
                mission_history::rebind_engine_from_doc();
            }
            if world_ready.get() {
                hand_over(boot);
            } else {
                boot.update(|b| *b = b.clone().advance(BootPhase::LoadingMap));
            }
            {
                let doc_get = doc.clone();
                let doc_cancel = doc.clone();
                yrs_persist::save_state_debounced(
                    &id,
                    Box::new(move || {
                        doc_get
                            .borrow()
                            .as_ref()
                            .map(|c| c.encode_state())
                            .unwrap_or_default()
                    }),
                    Box::new(move || doc_cancel.borrow().is_none()),
                    yrs_persist::debounce_ms(),
                );
            }
            let n = doc
                .borrow()
                .as_ref()
                .map(|c| c.slot_count() as u32)
                .unwrap_or(0);
            crate::v2::apps::editor::shell::session::mark_ready(&id, n, None);
            yrs_persist::register_flush_on_hide(id.clone());
            yrs_persist::register_tab_sync(doc.clone(), id.clone());
            ready.set(true);
        }
    });

    spawn_local({
        let engine = engine.clone();
        let disposed = disposed.clone();
        let doc = doc.clone();
        let canvas = canvas.clone();
        let map_host = map_host.clone();
        let dem_grid = dem_grid.clone();
        let restore_settled = restore_settled.clone();
        let engine_mounted = engine_mounted.clone();
        let world_ready = world_ready.clone();
        let report = report.clone();
        let (cw, ch) = (rect0.0, rect0.1);
        async move {
            match website_map_engine::frame::engine::RenderEngine::create(canvas, force_webgl).await
            {
                Ok(mut eng) => {
                    if disposed.load(Ordering::Relaxed) {
                        return;
                    }
                    let _ = eng.resize(cw, ch, dpr0);
                    eng.set_camera_bounds(0.0, 0.0, TERRAIN_W, TERRAIN_H);
                    eng.set_view(INITIAL_TARGET.0, INITIAL_TARGET.1, INITIAL_ZOOM);
                    eng.hide_calibration();
                    eng.disable_frame_timing();
                    eng.set_continuous_render(false); // damage-driven, matches the prod oracle
                    {
                        let (rgba, width, height, uv) =
                            website_map_engine::overlay::symbology::markers::build_marker_slot_atlas();
                        if let Err(e) = eng.ensure_slot_atlas(&rgba, width, height, &uv) {
                            leptos::logging::error!("ensure_slot_atlas: {e:?}");
                        }
                    }
                    *engine.borrow_mut() = Some(eng);
                    register_self_checks(engine.clone());
                    register_editor_cam(engine.clone(), map_host.clone());
                    #[cfg(target_arch = "wasm32")]
                    crate::v2::apps::editor::input::tools::los_world_wasm::register_object_wash_hook();
                    register_slot_stats(engine.clone());
                    crate::v2::apps::editor::bridge::world_assets::register_render_ctx(
                        engine.clone(),
                        map_host.clone(),
                    );
                    let soa = doc.borrow().as_ref().map(map_render_slot_soa);
                    let (vxy, valiases, vtints, vheadings) = mission_history::vehicle_lane_fields();
                    if let (Some(soa), Some(e)) = (soa.as_ref(), engine.borrow_mut().as_mut()) {
                        let tints = website_map_engine::overlay::symbology::roles::classify::side_tints_rgba_bytes(
                            &soa.side_keys,
                        );
                        e.slots_bind_symbology(
                            soa.ids.clone(),
                            &soa.xy,
                            &tints,
                            mission_history::soa_roles(soa),
                            &soa.rotations,
                        );
                        e.vehicles_bind_symbology(&vxy, valiases, &vtints, &vheadings);
                    }
                    engine_mounted.set(true);
                    if restore_settled.get() {
                        mission_history::rebind_engine_from_doc();
                    }
                    start_raf(engine.clone(), disposed.clone(), debug_hud, scale_mpp);
                    {
                        let terrain = doc
                            .borrow()
                            .as_ref()
                            .and_then(|c| {
                                serde_json::from_str::<serde_json::Value>(&c.small_maps_json())
                                    .ok()?
                                    .get("meta")?
                                    .get("terrain")?
                                    .as_str()
                                    .map(str::to_string)
                            })
                            .unwrap_or_else(|| "everon".to_string());
                        let host = map_host.clone();
                        let boot_fut = crate::v2::apps::editor::bridge::world_assets::bootstrap(
                            engine.clone(),
                            terrain,
                            host,
                            dem_grid.clone(),
                            report.clone(),
                        );
                        let world_ready = world_ready.clone();
                        let restore_settled = restore_settled.clone();
                        spawn_local(async move {
                            boot_fut.await;
                            world_ready.set(true);
                            if restore_settled.get() {
                                hand_over(boot);
                            }
                        });
                    }
                }
                Err(e) => {
                    let reason = js_sys::Error::from(wasm_bindgen::JsValue::from(e))
                        .message()
                        .as_string()
                        .filter(|s| !s.is_empty())
                        .unwrap_or_else(|| "the render engine failed to start".to_string());
                    leptos::logging::error!("RenderEngine::create: {reason}");
                    if disposed.load(Ordering::Relaxed) {
                        return;
                    }
                    let seg = progress.get_untracked().stage();
                    map_disabled.set(Some(reason.clone()));
                    boot.update(|b| *b = b.clone().advance(BootPhase::Failed { seg, reason }));
                }
            }
        }
    });
}
