//! Starts document restoration and the render engine in parallel.
//!
//! **Role:** the Mission Creator restores the local draft, reconciles it with the server and arms
//! the draft writer; a review workspace restores exactly the reviewed version instead and arms
//! nothing ([`review_restore`]). In parallel the engine starts on the shared map seam
//! ([`crate::foundation::map_view::engine_mount::create_engine`]), gets the editor's lanes, and
//! boots every terrain and world layer.
//! **Position:** started by the canvas mount once the document is set up.
//! **Signals & state:** the boot phase, progress and map-disabled signals; two readiness cells
//! (`engine_mounted`, `world_ready`) rendezvous with the document restore.
//! **Invariants:** the boot overlay is handed over only when both the document and the world
//! have settled; nothing touches the engine after the mount is disposed.

use super::*;

#[path = "review_restore.rs"]
mod review_restore;
use crate::foundation::map_view::camera_fit::{ViewState, WorldBounds};
use crate::foundation::map_view::engine_mount::{create_engine, CanvasSize, EngineStartup};
use crate::foundation::map_view::handles::MapViewHandles;
use leptos::task::spawn_local;
use std::cell::Cell;
use std::rc::Rc;

/// Camera pan bounds of the Mission Creator: the 12.8 km square every built-in terrain fits in.
const CAMERA_BOUNDS: WorldBounds = WorldBounds {
    min_x: 0.0,
    min_y: 0.0,
    max_x: 12_800.0,
    max_y: 12_800.0,
};

/// The Mission Creator's first view: the square's centre at zoom -2.
const INITIAL_VIEW: ViewState = ViewState {
    target_x: 6_400.0,
    target_y: 6_400.0,
    zoom: -2.0,
};

/// Inputs shared by document restoration and engine startup.
pub(super) struct BootContext {
    pub doc: mission_doc::DocHandle,
    pub mission_id: String,
    pub auth: crate::foundation::auth::AuthStore,
    pub current_semver: RwSignal<Option<String>>,
    pub conflict: RwSignal<Option<ConflictInfo>>,
    pub boot: RwSignal<BootPhase>,
    pub progress: RwSignal<boot_progress::BootProgress>,
    pub map_disabled: RwSignal<Option<String>>,
    pub view: MapViewHandles,
    pub restore_settled: Rc<Cell<bool>>,
    pub canvas: web_sys::HtmlCanvasElement,
    pub force_webgl: bool,
    pub canvas_size: CanvasSize,
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
        view,
        restore_settled,
        canvas,
        force_webgl,
        canvas_size,
        debug_hud,
        scale_mpp,
    } = ctx;
    let engine_mounted = Rc::new(Cell::new(false));
    let world_ready = Rc::new(Cell::new(false));
    let report: boot_progress::ProgressFn = Rc::new(move |ev| progress.update(|p| p.apply(ev)));
    if let Some(reviewed) =
        crate::workspaces::editor::session::review_mode::reviewed_for(&mission_id)
    {
        review_restore::start(review_restore::ReviewRestore {
            doc: doc.clone(),
            reviewed,
            current_semver,
            boot,
            report: report.clone(),
            restore_settled: restore_settled.clone(),
            engine_mounted: engine_mounted.clone(),
            world_ready: world_ready.clone(),
        });
    } else {
        restore_authored_document(AuthoredRestore {
            doc: doc.clone(),
            mission_id,
            auth,
            current_semver,
            conflict,
            boot,
            report: report.clone(),
            restore_settled: restore_settled.clone(),
            engine_mounted: engine_mounted.clone(),
            world_ready: world_ready.clone(),
        });
    }
    start_engine(EngineStart {
        doc,
        boot,
        progress,
        map_disabled,
        view,
        restore_settled,
        canvas,
        force_webgl,
        canvas_size,
        debug_hud,
        scale_mpp,
        engine_mounted,
        world_ready,
        report,
    });
}

/// What the Mission Creator's document restore works with.
struct AuthoredRestore {
    doc: mission_doc::DocHandle,
    mission_id: String,
    auth: crate::foundation::auth::AuthStore,
    current_semver: RwSignal<Option<String>>,
    conflict: RwSignal<Option<ConflictInfo>>,
    boot: RwSignal<BootPhase>,
    report: boot_progress::ProgressFn,
    restore_settled: Rc<Cell<bool>>,
    engine_mounted: Rc<Cell<bool>>,
    world_ready: Rc<Cell<bool>>,
}

/// Restore the Mission Creator's document: the local draft first, then the server's version, and
/// arm the draft writer once both have settled.
fn restore_authored_document(restore: AuthoredRestore) {
    let AuthoredRestore {
        doc,
        mission_id,
        auth,
        current_semver,
        conflict,
        boot,
        report,
        restore_settled,
        engine_mounted,
        world_ready,
    } = restore;
    let persist_ready = Rc::new(Cell::new(false));
    let persist_loaded = Rc::new(Cell::new(false));
    yrs_persist::register_mission_persist(
        doc.clone(),
        mission_id.clone(),
        persist_ready.clone(),
        persist_loaded.clone(),
    );
    spawn_local({
        let (id, ready, loaded) = (mission_id, persist_ready, persist_loaded);
        async move {
            if let Some(blob) = yrs_persist::load_state(&id).await {
                if !blob.is_empty() {
                    let fresh = map_engine::data::store::MissionDocCore::new();
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
            crate::workspaces::editor::session::warm_session_marker::mark_ready(&id, n, None);
            yrs_persist::register_flush_on_hide(id.clone());
            yrs_persist::register_tab_sync(doc.clone(), id.clone());
            ready.set(true);
        }
    });
}

/// What the render engine's startup works with.
struct EngineStart {
    doc: mission_doc::DocHandle,
    boot: RwSignal<BootPhase>,
    progress: RwSignal<boot_progress::BootProgress>,
    map_disabled: RwSignal<Option<String>>,
    view: MapViewHandles,
    restore_settled: Rc<Cell<bool>>,
    canvas: web_sys::HtmlCanvasElement,
    force_webgl: bool,
    canvas_size: CanvasSize,
    debug_hud: RwSignal<String>,
    scale_mpp: RwSignal<f64>,
    engine_mounted: Rc<Cell<bool>>,
    world_ready: Rc<Cell<bool>>,
    report: boot_progress::ProgressFn,
}

/// Create the render engine, bind the document's lanes to it, and boot the world assets.
fn start_engine(start: EngineStart) {
    let EngineStart {
        doc,
        boot,
        progress,
        map_disabled,
        view,
        restore_settled,
        canvas,
        force_webgl,
        canvas_size,
        debug_hud,
        scale_mpp,
        engine_mounted,
        world_ready,
        report,
    } = start;
    let MapViewHandles {
        engine,
        map_host,
        dem_grid,
        heights,
        disposed,
    } = view.clone();
    let startup = EngineStartup {
        force_webgl,
        size: canvas_size,
        bounds: CAMERA_BOUNDS,
        view: INITIAL_VIEW,
    };
    spawn_local({
        async move {
            match create_engine(canvas, startup).await {
                Ok(mut eng) => {
                    if view.is_disposed() {
                        return;
                    }
                    {
                        let (rgba, width, height, uv) =
                            map_engine::overlay::symbology::markers::build_marker_slot_atlas();
                        if let Err(e) = eng.ensure_slot_atlas(&rgba, width, height, &uv) {
                            leptos::logging::error!("ensure_slot_atlas: {e:?}");
                        }
                    }
                    *engine.borrow_mut() = Some(eng);
                    register_self_checks(engine.clone());
                    register_editor_cam(engine.clone(), map_host.clone());
                    crate::workspaces::editor::input::tools::los_world_wasm::register_object_wash_hook();
                    register_slot_stats(engine.clone());
                    crate::workspaces::editor::bridge::world_assets::register_render_ctx(
                        engine.clone(),
                        map_host.clone(),
                    );
                    let soa = doc.borrow().as_ref().map(map_render_slot_soa);
                    let (vxy, valiases, vtints, vheadings) = mission_history::vehicle_lane_fields();
                    if let (Some(soa), Some(e)) = (soa.as_ref(), engine.borrow_mut().as_mut()) {
                        let tints =
                            map_engine::overlay::symbology::roles::classify::side_tints_rgba_bytes(
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
                        let boot_fut = crate::workspaces::editor::bridge::world_assets::bootstrap(
                            engine.clone(),
                            terrain,
                            host,
                            dem_grid.clone(),
                            heights.handle(),
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
                Err(reason) => {
                    leptos::logging::error!("RenderEngine::create: {reason}");
                    if view.is_disposed() {
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
