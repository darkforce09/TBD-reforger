//! Pointer up handler for editor canvas gestures.

use super::*;

mod special_drag_release;

/// Builds the pointer up event closure for the canvas.
pub(super) fn make_pointer_up_handler(
    ctx: &EditorGestureContext,
    z_drag: &Rc<RefCell<Option<ov::ZDrag>>>,
    vertex_pointer: &Rc<Cell<Option<i32>>>,
) -> Closure<dyn FnMut(web_sys::PointerEvent)> {
    let container = ctx.container.clone();
    let engine = ctx.engine.clone();
    let doc = ctx.doc.clone();
    let selection = ctx.selection.clone();
    let left = ctx.left.clone();
    let pan_px = ctx.pan_px.clone();
    let map_host = ctx.map_host.clone();
    let dem_grid = ctx.dem_grid.clone();
    let ruler = ctx.ruler.clone();
    let los = ctx.los.clone();
    let viewshed = ctx.viewshed.clone();
    let tool_mode = ctx.tool_mode;
    let los_mode = ctx.los_mode;
    let snap = ctx.snap;
    let selected_connection = ctx.selected_connection;
    let sync_ruler = make_sync_ruler(ctx);
    let sync_los = make_sync_los(ctx);
    let onpointerup = Closure::<dyn FnMut(web_sys::PointerEvent)>::new({
        let z_drag = z_drag.clone();
        let vertex_pointer = vertex_pointer.clone();
        let pan_px = pan_px.clone();
        let container = container.clone();
        let engine = engine.clone();
        let left = left.clone();
        let doc = doc.clone();
        let selection = selection.clone();
        let map_host = map_host.clone();
        let ruler = ruler.clone();
        let dem_grid = dem_grid.clone();
        let sync_ruler = sync_ruler.clone();
        let los = los.clone();
        let sync_los = sync_los;
        let viewshed = viewshed.clone();
        move |ev: web_sys::PointerEvent| {
            if special_drag_release::consume_special_drag(
                &ev,
                &z_drag,
                &vertex_pointer,
                &container,
                &doc,
                snap,
            ) {
                return;
            }
            // The ARMED state (`has_pending()`) is checked
            // before any gesture branch below so placement claims the release first.
            // Ctrl is OVERLOADED by gesture state: with a palette place armed, it keeps placement
            // armed for another click; without an armed place, Ctrl on a selected slot can regroup
            // that slot onto the clicked target. The armed branch takes priority over regrouping.
            if armed_placement::has_pending() {
                let _ = left.borrow_mut().take();

                let button = ev.button();
                if button == 1 {
                } else if button == 2 {
                    armed_placement::cancel_pending();
                    if let Some(e) = engine.borrow_mut().as_mut() {
                        e.clear_place_preview();
                    }
                    return;
                } else if button != 0 {
                    return;
                } else {
                    let rect = container.get_bounding_client_rect();
                    let (px, py) = (
                        ev.client_x() as f64 - rect.left(),
                        ev.client_y() as f64 - rect.top(),
                    );
                    let on_canvas = px >= crate::v2::apps::editor::shell::layout::dock_left_px()
                        && px
                            <= rect.width()
                                - crate::v2::apps::editor::shell::layout::dock_right_px()
                        && py >= crate::v2::apps::editor::shell::layout::strip_top_px()
                        && py
                            <= rect.height()
                                - crate::v2::apps::editor::shell::layout::toolbelt_band_px();
                    let world = if on_canvas {
                        let g = engine.borrow();
                        g.as_ref().map(|e| {
                            selection::frozen_camera(
                                rect.width(),
                                rect.height(),
                                e.target_x(),
                                e.target_y(),
                                e.zoom(),
                            )
                            .unproject_xy(px, py)
                        })
                    } else {
                        None
                    };
                    let world_ok = world.filter(|c| c[0].is_finite() && c[1].is_finite());
                    match armed_place::decide_armed_pointerup(button, world_ok.is_some()) {
                        armed_place::ArmedUp::Place => {
                            let ctrl_multi = ev.ctrl_key() || ev.meta_key();
                            let alt_empty = ev.alt_key();
                            let c = world_ok.expect("Place implies finite world");
                            if ctrl_multi {
                                armed_placement::place_at_keep(c[0], c[1], alt_empty);
                            } else {
                                armed_placement::place_at_alt(c[0], c[1], alt_empty);
                            }
                        }
                        armed_place::ArmedUp::KeepArmed => {}
                        armed_place::ArmedUp::FallThroughPan
                        | armed_place::ArmedUp::Disarm
                        | armed_place::ArmedUp::Ignore => {}
                    }
                    if let Some(e) = engine.borrow_mut().as_mut() {
                        e.clear_place_preview();
                    }
                    return;
                }
            }
            if pan_px.get().is_some() {
                pan_px.set(None);
                if container.has_pointer_capture(ev.pointer_id()) {
                    let _ = container.release_pointer_capture(ev.pointer_id());
                }
                website_map_engine::streaming::host::set_camera_gesture(false);
                website_map_engine::streaming::host::schedule_camera_settle(
                    map_host.clone(),
                    engine.clone(),
                );
            }
            let taken = left.borrow_mut().take();
            let Some(g) = taken else { return };
            use selection::LeftGesture as LG;
            if ev.button() != 0 {
                match &g {
                    LG::Move { .. } => {
                        if container.has_pointer_capture(ev.pointer_id()) {
                            let _ = container.release_pointer_capture(ev.pointer_id());
                        }
                        if let Some(e) = engine.borrow_mut().as_mut() {
                            crate::v2::apps::editor::input::tools::select_tool::clear_drag_preview(
                                e,
                                &engine_ops::vehicle_points(),
                            );
                            if let Some((cxy, cids)) = doc.borrow().as_ref().map(|c| {
                                (
                                    comment_lane_xy(&c.comments_json()),
                                    comment_lane_ids(&c.comments_json()),
                                )
                            }) {
                                e.comments_bind_ids(&cxy, cids);
                            }
                        }
                    }
                    LG::Marquee { .. } => {
                        if container.has_pointer_capture(ev.pointer_id()) {
                            let _ = container.release_pointer_capture(ev.pointer_id());
                        }
                        if let Some(e) = engine.borrow_mut().as_mut() {
                            e.upload_marquee(0.0, 0.0, 0.0, 0.0, false);
                        }
                    }
                    LG::Rotate { .. } => {
                        if container.has_pointer_capture(ev.pointer_id()) {
                            let _ = container.release_pointer_capture(ev.pointer_id());
                        }
                    }
                    _ => {}
                }
                return;
            }
            let rect = container.get_bounding_client_rect();
            let up_x = ev.client_x() as f64 - rect.left();
            let up_y = ev.client_y() as f64 - rect.top();
            match g {
                LG::Pending(p) => {
                    let moved = ((up_x - p.start_x).powi(2) + (up_y - p.start_y).powi(2)).sqrt();
                    if moved < selection::DRAG_THRESHOLD_PX {
                        let additive = ev.ctrl_key() || ev.meta_key();
                        let hit = doc.borrow().as_ref().and_then(|c| {
                            selection::pick_slot_or_vehicle(
                                &p.cam,
                                &map_render_slot_soa(c),
                                &engine_ops::vehicle_points(),
                                p.start_x,
                                p.start_y,
                            )
                        });
                        if engine_ops::pending_connect().is_some() {
                            if let Some(ref id) = hit {
                                let _ = engine_ops::complete_connect(id);
                            }
                        }
                        let hit = hit.or_else(|| {
                            let w = p.cam.unproject_xy(p.start_x, p.start_y);
                            let w2 = p.cam.unproject_xy(p.start_x + COMMENT_PICK_PX, p.start_y);
                            let tol = (w2[0] - w[0]).hypot(w2[1] - w[1]);
                            doc.borrow().as_ref().and_then(|c| {
                                pick_comment(&comment_points(&c.comments_json()), w[0], w[1], tol)
                            })
                        });
                        if hit.is_some() {
                            selected_connection.set(None);
                        } else if !additive {
                            let w = p.cam.unproject_xy(p.start_x, p.start_y);
                            let w2 = p.cam.unproject_xy(p.start_x + CONN_PICK_PX, p.start_y);
                            let tol = (w2[0] - w[0]).hypot(w2[1] - w[1]);
                            let edge = doc.borrow().as_ref().and_then(|c| {
                                pick_connection(&live_connection_segments(c), w[0], w[1], tol)
                            });
                            selected_connection.set(edge);
                        }
                        if hit.is_some() {
                            if tactical_graphics_authoring::clear_tactical_selection() {
                                mission_history::refresh_tactical_lane();
                            }
                        } else if !additive {
                            let w = p.cam.unproject_xy(p.start_x, p.start_y);
                            let w2 = p.cam.unproject_xy(p.start_x + TG_PICK_PX, p.start_y);
                            let tol = (w2[0] - w[0]).hypot(w2[1] - w[1]);
                            tactical_graphics_authoring::select_tactical_graphic_at(
                                w[0], w[1], tol,
                            );
                            mission_history::refresh_tactical_lane();
                        }
                        {
                            let mut sel = selection.borrow_mut();
                            let keep_multi = !additive
                                && sel.len() > 1
                                && hit.as_ref().is_some_and(|h| sel.iter().any(|s| s == h));
                            if !keep_multi {
                                selection::apply_click(&mut sel, hit, additive);
                            }
                        }
                        let ids = selection.borrow().clone();
                        if let Some(e) = engine.borrow_mut().as_mut() {
                            let slot_ids: Vec<String> = ids
                                .iter()
                                .filter(|i| !engine_ops::is_vehicle_id(i))
                                .cloned()
                                .collect();
                            e.set_selection(slot_ids); // tint lane (slots only)
                        }
                        mission_history::refresh_selection();
                    }
                }
                LG::Move {
                    ids, dx, dy, cam, ..
                } => {
                    if container.has_pointer_capture(ev.pointer_id()) {
                        let _ = container.release_pointer_capture(ev.pointer_id());
                    }
                    let single_comment_drag = ids.len() == 1
                        && doc.borrow().as_ref().is_some_and(|c| {
                            website_map_engine::data::store::operations::entity::comment_details(c)
                                .iter()
                                .any(|d| d.id == ids[0])
                        });
                    let regrouped = if (ev.ctrl_key() || ev.meta_key())
                        && ids.len() == 1
                        && !engine_ops::is_vehicle_id(&ids[0])
                        && !single_comment_drag
                    {
                        let target = doc.borrow().as_ref().and_then(|c| {
                            selection::pick(&cam, &map_render_slot_soa(c), up_x, up_y)
                        });
                        match target {
                            Some(tid) if tid != ids[0] => {
                                let ok = engine_ops::regroup_slot_onto(&ids[0], &tid);
                                if ok {
                                    if let Some(e) = engine.borrow_mut().as_mut() {
                                        crate::v2::apps::editor::input::tools::select_tool::clear_drag_preview(
                                            e,
                                            &engine_ops::vehicle_points(),
                                        );
                                    }
                                }
                                ok
                            }
                            _ => false,
                        }
                    } else {
                        false
                    };
                    if regrouped {
                        return;
                    }
                    if dx != 0.0 || dy != 0.0 {
                        let comment_ids: Vec<String> = doc
                            .borrow()
                            .as_ref()
                            .map(|c| {
                                let members: std::collections::HashSet<String> =
                                    website_map_engine::data::store::operations::entity::comment_details(c)
                                        .into_iter()
                                        .map(|d| d.id)
                                        .collect();
                                ids.iter()
                                    .filter(|id| members.contains(*id))
                                    .cloned()
                                    .collect()
                            })
                            .unwrap_or_default();
                        if !comment_ids.is_empty() {
                            let moves: Vec<(String, f64, f64)> = doc
                                .borrow()
                                .as_ref()
                                .map(|c| {
                                    dragged_comment_points(
                                        &comment_points(&c.comments_json()),
                                        &comment_ids,
                                    )
                                    .into_iter()
                                    .map(|p| (p.id, p.x + dx, p.y + dy))
                                    .collect()
                                })
                                .unwrap_or_default();
                            for (id, x, z) in moves {
                                engine_ops::move_comment(id, x, z);
                            }
                        }
                        let (veh_ids, slot_ids): (Vec<String>, Vec<String>) = ids
                            .iter()
                            .filter(|id| !comment_ids.iter().any(|c| c == *id))
                            .cloned()
                            .partition(|id| engine_ops::is_vehicle_id(id));
                        if !slot_ids.is_empty() || !veh_ids.is_empty() {
                            let mut guard = doc.borrow_mut();
                            let Some(core) = guard.as_mut() else {
                                return;
                            };
                            let z_rows = (!slot_ids.is_empty())
                                .then(|| attrs::keep_z_rows(core, Some(dx), Some(dy), None))
                                .flatten();
                            let zs: Vec<f64> = slot_ids
                                .iter()
                                .map(|id| {
                                    z_rows
                                        .as_ref()
                                        .and_then(|rows| attrs::slot_z(rows, id))
                                        .unwrap_or(0.0)
                                })
                                .collect();
                            core.begin_group();
                            core.move_entities_and_vehicles(slot_ids, &veh_ids, dx, dy, zs);
                            core.end_group();
                            drop(guard);
                            mission_history::after_local_edit();
                        }
                    } else if let Some(e) = engine.borrow_mut().as_mut() {
                        crate::v2::apps::editor::input::tools::select_tool::clear_drag_preview(
                            e,
                            &engine_ops::vehicle_points(),
                        );
                        if let Some((cxy, cids)) = doc.borrow().as_ref().map(|c| {
                            (
                                comment_lane_xy(&c.comments_json()),
                                comment_lane_ids(&c.comments_json()),
                            )
                        }) {
                            e.comments_bind_ids(&cxy, cids);
                        }
                    }
                }
                LG::Marquee {
                    start_x,
                    start_y,
                    start_wx,
                    start_wy,
                    cam,
                } => {
                    if container.has_pointer_capture(ev.pointer_id()) {
                        let _ = container.release_pointer_capture(ev.pointer_id());
                    }
                    if (up_x - start_x).abs() >= 1.0 && (up_y - start_y).abs() >= 1.0 {
                        let ids = doc
                            .borrow()
                            .as_ref()
                            .map(|c| {
                                selection::marquee_ids_with_vehicles(
                                    &cam,
                                    &map_render_slot_soa(c),
                                    &engine_ops::vehicle_points(),
                                    start_wx,
                                    start_wy,
                                    up_x,
                                    up_y,
                                )
                            })
                            .unwrap_or_default();
                        *selection.borrow_mut() = ids.clone();
                        if let Some(e) = engine.borrow_mut().as_mut() {
                            let slot_ids: Vec<String> = ids
                                .iter()
                                .filter(|i| !engine_ops::is_vehicle_id(i))
                                .cloned()
                                .collect();
                            e.set_selection(slot_ids);
                        }
                        mission_history::refresh_selection();
                    }
                    if let Some(e) = engine.borrow_mut().as_mut() {
                        e.upload_marquee(0.0, 0.0, 0.0, 0.0, false); // hide
                    }
                }
                LG::Ruler {
                    start_x,
                    start_y,
                    cam,
                } => {
                    let moved = ((up_x - start_x).powi(2) + (up_y - start_y).powi(2)).sqrt();
                    if moved < selection::DRAG_THRESHOLD_PX {
                        let w = cam.unproject_xy(start_x, start_y);
                        if w[0].is_finite() && w[1].is_finite() {
                            let z = dem_grid.borrow().as_ref().and_then(|g| {
                                website_map_engine::world::terrain::dem::grid::sample_grid_meters(
                                    g, w[0], w[1],
                                )
                            });
                            if tool_mode.get_untracked().is_los() {
                                if los_mode.get_untracked().is_viewshed() {
                                    viewshed.borrow_mut().place(w[0], w[1], z);
                                    if let Some(tex) = place_viewshed(w[0], w[1]) {
                                        if let Some(e) = engine.borrow_mut().as_mut() {
                                            let _ = e.viewshed_upload(
                                                tex.min_x,
                                                tex.min_y,
                                                tex.max_x,
                                                tex.max_y,
                                                tex.tex_w,
                                                tex.tex_h,
                                                &tex.rgba,
                                                tex.stride_bytes,
                                            );
                                        }
                                    }
                                    crate::v2::apps::editor::input::tools::los_world_wasm::start_object_wash();
                                } else {
                                    los.borrow_mut().click(w[0], w[1], z);
                                    sync_los();
                                }
                            } else {
                                ruler.borrow_mut().press(w[0], w[1], z);
                                sync_ruler();
                            }
                        }
                    }
                }
                LG::Rotate { cam, .. } => {
                    if container.has_pointer_capture(ev.pointer_id()) {
                        let _ = container.release_pointer_capture(ev.pointer_id());
                    }
                    let aim = cam.unproject_xy(up_x, up_y);
                    if aim[0].is_finite() && aim[1].is_finite() {
                        let rung = snap.get_untracked().effective_rotate_rung();
                        let acted =
                            selection_transform::rotate_selection_to_face(aim[0], aim[1], rung);
                        if acted {
                            mission_history::refresh_selection();
                        }
                    }
                }
            }
        }
    });
    onpointerup
}
