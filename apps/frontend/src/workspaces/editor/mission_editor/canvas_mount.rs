//! Mounts the Mission Creator's canvas on the shared map seam and installs editor host state.
//!
//! **Role:** on canvas load, sizes the canvas and creates the per-mount map handles through
//! [`crate::foundation::map_view`], installs the editor's tools, document context and chrome
//! effects, then starts the boot tasks and the input listeners.
//! **Position:** called once by the Mission Creator page; the seam owns sizing, engine creation,
//! the frame pump and resize tracking, and this module owns everything editor-specific.
//! **Signals & state:** the page's signals arrive in [`PageMountSignals`]; the tool states
//! (selection, ruler, line of sight, viewshed) are created here and live for the mount.
//! **Invariants:** document setup completes before the boot tasks start; every handle the tasks
//! and listeners share comes from one [`crate::foundation::map_view::handles::MapViewHandles`].

#[path = "canvas_mount/document_setup.rs"]
mod document_setup;
#[path = "canvas_mount/signals.rs"]
mod signals;

#[path = "canvas_mount/registry_effects.rs"]
mod registry_effects;

#[path = "canvas_mount/input_listeners.rs"]
mod input_listeners;

#[path = "canvas_mount/boot_tasks.rs"]
mod boot_tasks;

#[path = "canvas_mount/dock_reflow.rs"]
mod dock_reflow;

use super::*;

pub(super) use signals::PageMountSignals;

/// Initializes the canvas host and attaches editor lifecycle tasks.
pub(super) fn install_canvas_mount(signals: PageMountSignals) {
    let PageMountSignals {
        container_ref,
        canvas_ref,
        can_undo,
        can_redo,
        obj_count,
        sel_count,
        cursor,
        debug_hud,
        debug_hud_shown,
        scale_mpp,
        tool_mode,
        los_mode,
        ruler_status,
        ruler_tick,
        los_tick,
        snap,
        widget_variant,
        boot,
        map_disabled,
        progress,
        outliner_nodes,
        orbat_nodes,
        selected_ids,
        active_layer,
        active_side,
        objects_mode,
        catalog,
        vehicle_catalog,
        attrs_open,
        attrs_tab,
        doc_tick,
        selected_connection,
        context_menu,
        asset_picker,
        comment_editor,
        connections_panel,
        chrome_hidden,
        dock_left_collapsed,
        dock_right_collapsed,
        registry_items,
        registry_failed,
        registry_fetch_gen,
        compat,
        dirty,
        conflict,
        current_semver,
        mission_id,
    } = signals;
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    let mission_id = mission_id.clone();

    let auth = expect_context::<crate::foundation::auth::AuthStore>();

    registry_effects::install(
        auth,
        registry_fetch_gen,
        registry_items,
        registry_failed,
        vehicle_catalog,
        catalog,
        compat,
    );

    canvas_ref.on_load(move |canvas: web_sys::HtmlCanvasElement| {
        let Some(container) = container_ref.get_untracked() else {
            return;
        };
        let container: web_sys::HtmlDivElement = container;
        let force_webgl = crate::foundation::map_view::engine_mount::force_webgl_from_location();
        let canvas_size = crate::foundation::map_view::engine_mount::size_canvas(&container, &canvas);
        let view = crate::foundation::map_view::handles::MapViewHandles::new();
        let engine = view.engine.clone();
        let dem_grid = view.dem_grid.clone();

        let (doc, doc_ver) = document_setup::initialize(auth, mission_id.clone(), current_semver);

        let selection: selection::SelectionHandle = Rc::new(RefCell::new(Vec::new()));
        let left: Rc<RefCell<Option<selection::LeftGesture>>> = Rc::new(RefCell::new(None));
        let ruler: Rc<RefCell<map_engine::editing::tools::ruler::RulerChain>> = Rc::new(
            RefCell::new(map_engine::editing::tools::ruler::RulerChain::new()),
        );
        let sync_ruler = {
            let ruler = ruler.clone();
            move || {
                ruler_status.set(ruler.borrow().status_readout());
                ruler_tick.update(|t| *t = t.wrapping_add(1));
            }
        };
        {
            let ruler = ruler.clone();
            let sync_ruler = sync_ruler.clone();
            Effect::new(move |_| {
                if !tool_mode.get().is_ruler() && !ruler.borrow().is_empty() {
                    ruler.borrow_mut().clear();
                    sync_ruler();
                }
            });
        }
        crate::workspaces::editor::input::tools::ruler_tool::register_ruler_chain(ruler.clone());

        let los: Rc<RefCell<LosState>> = Rc::new(RefCell::new(LosState::new()));
        let sync_los = {
            move || {
                los_tick.update(|t| *t = t.wrapping_add(1));
            }
        };
        let viewshed: Rc<RefCell<ViewshedState>> = Rc::new(RefCell::new(ViewshedState::new()));
        {
            let los = los.clone();
            let viewshed = viewshed.clone();
            let engine = engine.clone();
            Effect::new(move |_| {
                let is_los = tool_mode.get().is_los();
                let viewshed_active = is_los && los_mode.get().is_viewshed();
                if !is_los && !los.borrow().is_empty() {
                    los.borrow_mut().clear();
                    sync_los();
                }
                if !viewshed_active && !viewshed.borrow().is_empty() {
                    viewshed.borrow_mut().clear();
                    if let Some(e) = engine.borrow_mut().as_mut() {
                        e.viewshed_clear();
                    }
                    crate::workspaces::editor::input::tools::los_world_wasm::cancel_object_wash();
                }
            });
        }
        crate::workspaces::editor::input::tools::viewshed_scheduler::install_scheduler_host();
        crate::workspaces::editor::input::tools::los_tool::register_los_state(los.clone());
        crate::workspaces::editor::input::tools::los_tool::register_viewshed_state(viewshed.clone());
        {
            let dem_grid = dem_grid.clone();
            crate::workspaces::editor::input::tools::los_tool::register_los_sampler(
                std::rc::Rc::new(move |x: f64, y: f64| {
                    dem_grid.borrow().as_ref().and_then(|g| {
                        terrain_elevation::grid::sample_grid_meters(g, x, y)
                    })
                }),
            );
        }

        crate::workspaces::editor::input::tools::select_tool::register_editor_selection(
            selection.clone(),
            doc.clone(),
            engine.clone(),
            container.clone(),
        );

        {
            let doc = doc.clone();
            let selection = selection.clone();
            register_widget_pivot(std::rc::Rc::new(move || {
                let sel = selection.borrow();
                if sel.is_empty() {
                    return None;
                }
                let d = doc.borrow();
                let core = d.as_ref()?;
                let soa = core.materialize();
                let veh = serde_json::from_str::<serde_json::Value>(&core.small_maps_json()).ok();
                let (mut sx, mut sy, mut n) = (0.0f64, 0.0f64, 0.0f64);
                for id in sel.iter() {
                    if let Some(row) = soa.ids.iter().position(|s| s == id) {
                        sx += f64::from(soa.xs[row]);
                        sy += f64::from(soa.ys[row]);
                        n += 1.0;
                    } else if let Some(pos) = veh
                        .as_ref()
                        .and_then(|r| r.get("vehiclesById")?.get(id)?.get("position").cloned())
                    {
                        if let (Some(vx), Some(vy)) = (
                            pos.get("x").and_then(serde_json::Value::as_f64),
                            pos.get("y").and_then(serde_json::Value::as_f64),
                        ) {
                            sx += vx;
                            sy += vy;
                            n += 1.0;
                        }
                    }
                }
                if n == 0.0 {
                    None
                } else {
                    Some((sx / n, sy / n))
                }
            }));
        }

        {
            let container = container.clone();
            register_editor_toolbar_dispatch(std::rc::Rc::new(EditorToolbarDispatch {
                set_widget: Box::new(move |d: u8| {
                    widget_variant.set(widget_variant.get_untracked().select_digit(d));
                }),
                toggle_snap: Box::new(move || {
                    snap.set(snap.get_untracked().toggled());
                }),
                snap_step: Box::new(move |delta: i32| {
                    let axis = widget_variant.get_untracked().snap_axis();
                    snap.set(snap.get_untracked().stepped(axis, delta));
                }),
                select_all: Box::new(move || {
                    let rect = container.get_bounding_client_rect();
                    crate::workspaces::editor::bridge::host_state::entity_selection::select_all_in_view(rect.width(), rect.height());
                }),
                widget_digit: Box::new(move || widget_variant.get().to_digit()),
                snap_enabled: Box::new(move || snap.get().enabled),
            }));
            on_cleanup(unregister_editor_toolbar_dispatch);
        }

        {
            let doc = doc.clone();
            validation_panel::register_payload_source(std::rc::Rc::new(move || {
                let d = doc.borrow();
                let core = d.as_ref()?;
                let payload = map_engine::data::scenario::compile::compile_payload(
                    &core.small_maps_json(),
                    &core.slots_json(),
                    false,
                );
                let known_asset_ids = registry_items
                    .get_untracked()
                    .map(|items| validation_panel::known_asset_ids_from_registry(&items));
                Some(validation_panel::PayloadSource {
                    payload,
                    known_asset_ids,
                })
            }));
        }

        {
            let doc = doc.clone();
            let selection = selection.clone();
            let engine = engine.clone();
            let resolve: SubjectResolver = std::rc::Rc::new(move |subject_id: &str| {
                let d = doc.borrow();
                let core = d.as_ref()?;
                let soa = core.materialize();
                let slot_row = soa.ids.iter().position(|s| s == subject_id);
                let root = serde_json::from_str::<serde_json::Value>(&core.small_maps_json())
                    .unwrap_or(serde_json::Value::Null);
                let target = route_target(&root, subject_id, &|_| slot_row.is_some())?;
                let (cx, cy) = match target {
                    RouteTarget::Slot => {
                        let row = slot_row.expect("Slot arm implies the SoA matched");
                        (f64::from(soa.xs[row]), f64::from(soa.ys[row]))
                    }
                    RouteTarget::Vehicle { x, y }
                    | RouteTarget::Entity { x, y }
                    | RouteTarget::Zone { x, y }
                    | RouteTarget::Comment { x, y } => (x, y),
                };
                Some((target, cx, cy))
            });
            let available: SubjectResolver = {
                let resolve = std::rc::Rc::clone(&resolve);
                std::rc::Rc::new(move |subject_id: &str| {
                    route_availability(resolve(subject_id), &|| !chrome_hidden.get())
                })
            };
            {
                let probe = std::rc::Rc::clone(&available);
                validation_panel::register_route_probe(std::rc::Rc::new(
                    move |subject_id: &str| probe(subject_id).is_some(),
                ));
            }
            validation_panel::register_select_by_id(std::rc::Rc::new(move |subject_id: &str| {
                let Some((target, cx, cy)) = available(subject_id) else {
                    return false;
                };
                if matches!(target, RouteTarget::Zone { .. }) {
                    if !crate::workspaces::editor::ui::docks::dock_right::route_select_zone(
                        subject_id,
                    ) {
                        return false;
                    }
                } else {
                    *selection.borrow_mut() = vec![subject_id.to_string()];
                    let ids = selection.borrow().clone();
                    if let Some(e) = engine.borrow_mut().as_mut() {
                        e.set_selection(ids);
                    }
                    mission_history::refresh_selection();
                }
                if let Some(e) = engine.borrow_mut().as_mut() {
                    e.set_view(cx, cy, e.zoom()); // centre on the offender
                    e.on_camera_changed();
                }
                true
            }));
        }

        let restore_settled = Rc::new(Cell::new(false));
        map_engine::editing::host::install(doc.clone(), selection.clone());
        mission_history::set_ctx(
            doc.clone(),
            engine.clone(),
            selection.clone(),
            doc_ver.clone(),
            mission_id.clone(),
            can_undo,
            can_redo,
            obj_count,
            sel_count,
            dirty,
            restore_settled.clone(),
        );
        editor_context::install(
            doc.clone(),
            engine.clone(),
            selection.clone(),
            active_layer,
            active_side,
            objects_mode,
            outliner_nodes,
            orbat_nodes,
            selected_ids,
            attrs_open,
            attrs_tab,
            doc_tick,
        );
        crate::workspaces::editor::ui::docks::context_menu::set_menu_signal(context_menu);
        editor_context::set_asset_picker_signal(asset_picker);
        editor_context::set_comment_editor_signal(comment_editor);
        editor_context::set_connections_panel_signal(connections_panel);
        editor_context::set_connection_selection_signal(selected_connection);

        mission_history::register_editor_history();
        crate::workspaces::editor::input::window_keydown::register_key_handler();
        mission_history::register_unload_guard();
        on_cleanup(mission_history::unregister_unload_guard);

        {
            let doc = doc.clone();
            let engine = engine.clone();
            Effect::new(move |_| {
                let _ = doc_tick.get();
                let selected = selected_connection.get();
                let segs = doc
                    .borrow()
                    .as_ref()
                    .map_or_else(Vec::new, live_connection_segments);
                let verts = connection_lane_verts(&segs, selected.as_deref());
                #[allow(clippy::cast_possible_truncation)]
                let count = segs.len() as u32;
                if let Some(e) = engine.borrow_mut().as_mut() {
                    e.connections_bind(&verts, count);
                }
            });
        }

        mission_history::refresh_hud();

        dock_reflow::install(
            engine.clone(),
            container.clone(),
            chrome_hidden,
            dock_left_collapsed,
            dock_right_collapsed,
        );

        boot_tasks::start(boot_tasks::BootContext {
            doc: doc.clone(),
            mission_id: mission_id.clone(),
            auth,
            current_semver,
            conflict,
            boot,
            progress,
            map_disabled,
            view: view.clone(),
            restore_settled: restore_settled.clone(),
            canvas: canvas.clone(),
            force_webgl,
            canvas_size,
            debug_hud,
            scale_mpp,
        });

        input_listeners::attach(input_listeners::InputContext {
            container,
            canvas,
            view,
            doc,
            selection,
            left,
            ruler,
            los,
            viewshed,
            cursor,
            tool_mode,
            los_mode,
            snap,
            widget_variant,
            selected_connection,
            doc_tick,
            ruler_status,
            ruler_tick,
            los_tick,
            chrome_hidden,
            dock_left_collapsed,
            dock_right_collapsed,
            debug_hud_shown,
        });
    });
}
