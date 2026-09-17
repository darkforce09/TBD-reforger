//! Pointer move handler for editor canvas gestures.

use super::*;

/// Builds the pointer move event closure for the canvas.
pub(super) fn make_pointer_move_handler(
    ctx: &EditorGestureContext,
    z_drag: &Rc<RefCell<Option<ov::ZDrag>>>,
    vertex_pointer: &Rc<Cell<Option<i32>>>,
    left_pointer: &Rc<Cell<Option<i32>>>,
) -> Closure<dyn FnMut(web_sys::PointerEvent)> {
    let container = ctx.container.clone();
    let canvas = ctx.canvas.clone();
    let engine = ctx.engine.clone();
    let doc = ctx.doc.clone();
    let selection = ctx.selection.clone();
    let left = ctx.left.clone();
    let pan_px = ctx.pan_px.clone();
    let map_host = ctx.map_host.clone();
    let dem_grid = ctx.dem_grid.clone();
    let hover_state = ctx.hover_state.clone();
    let hover_points = ctx.hover_points.clone();
    let cursor = ctx.cursor;
    let tool_mode = ctx.tool_mode;
    let snap = ctx.snap;
    let widget_variant = ctx.widget_variant;
    let doc_tick = ctx.doc_tick;
    let onpointermove = Closure::<dyn FnMut(web_sys::PointerEvent)>::new({
        let z_drag = z_drag.clone();
        let vertex_pointer = vertex_pointer.clone();
        let left_pointer = left_pointer.clone();
        let pan_px = pan_px.clone();
        let engine = engine.clone();
        let left = left.clone();
        let doc = doc.clone();
        let selection = selection.clone();
        let container = container.clone();
        let dem_grid = dem_grid.clone();
        let map_host = map_host.clone();
        let hover_state = hover_state.clone();
        let hover_points = hover_points.clone();
        let canvas = canvas.clone();
        move |ev: web_sys::PointerEvent| {
            use selection::LeftGesture as LG;
            let rect = container.get_bounding_client_rect();
            let (px, py) = (
                ev.client_x() as f64 - rect.left(),
                ev.client_y() as f64 - rect.top(),
            );
            let hover_cam = {
                let g = engine.borrow();
                g.as_ref().map(|e| {
                    selection::frozen_camera(
                        rect.width(),
                        rect.height(),
                        e.target_x(),
                        e.target_y(),
                        e.zoom(),
                    )
                })
            };
            let world = hover_cam.as_ref().map(|c| c.unproject_xy(px, py));
            cursor.set(
                world
                    .filter(|c| c[0].is_finite() && c[1].is_finite())
                    .map(|c| {
                        let z = dem_grid.borrow().as_ref().and_then(|g| {
                            website_map_engine::world::terrain::dem::grid::sample_grid_meters(
                                g, c[0], c[1],
                            )
                        });
                        (c[0], c[1], z)
                    }),
            );
            if let Some((lx, ly)) = pan_px.get() {
                let (cx, cy) = (ev.client_x() as f64, ev.client_y() as f64);
                if let Some(e) = engine.borrow_mut().as_mut() {
                    e.pan(cx - lx, cy - ly);
                    e.on_camera_changed();
                }
                pan_px.set(Some((cx, cy)));
                website_map_engine::streaming::host::schedule_camera_settle(
                    map_host.clone(),
                    engine.clone(),
                );
                return;
            }
            let z_arm = z_drag.borrow().clone();
            if let Some(arm) = z_arm {
                if arm.pointer_id != ev.pointer_id() {
                    return;
                }
                let delta = ov::z_drag_elevation_delta(
                    py,
                    arm.start_y,
                    arm.scale,
                    ov::z_drag_snap_step(snap.get_untracked(), ev.shift_key()),
                );
                crate::v2::apps::editor::bridge::overlays::set_z_drag_readout(Some(
                    crate::v2::apps::editor::bridge::gizmo_z::format_height_readout(
                        arm.height(delta),
                    ),
                ));
                return;
            }
            if tactical_graphics_authoring::tactical_vertex_drag_active() {
                if vertex_pointer.get() != Some(ev.pointer_id()) {
                    return;
                }
                if let Some(c) = world.filter(|c| c[0].is_finite() && c[1].is_finite()) {
                    tactical_graphics_authoring::tactical_vertex_drag_move(c[0], c[1]);
                    mission_history::refresh_tactical_lane();
                }
                return;
            }
            if armed_placement::has_pending() {
                if let Some(c) = world.filter(|c| c[0].is_finite() && c[1].is_finite()) {
                    if let Some(e) = engine.borrow_mut().as_mut() {
                        e.set_place_preview(c[0] as f32, c[1] as f32);
                    }
                }
                return;
            }

            {
                let now_ms = js_sys::Date::now();
                let gesture_active = left.borrow().is_some();
                let prev = hover_state.get();
                if hover_suppressed(
                    gesture_active,
                    armed_placement::has_pending(),
                    tool_mode.get_untracked().captures_points(),
                ) {
                    if prev.pickable {
                        set_map_cursor(&canvas, false);
                    }
                    hover_state.set(HoverState::default());
                } else if hover_due(prev, now_ms) {
                    let hit = hover_cam.as_ref().is_some_and(|cam| {
                        hover_hit(
                            &mut hover_points.borrow_mut(),
                            doc_tick.get_untracked(),
                            &doc,
                            cam,
                            px,
                            py,
                        )
                    });
                    let next = hover_next(prev, hit, px, py, now_ms);
                    if next.pickable != prev.pickable {
                        set_map_cursor(&canvas, next.pickable);
                    }
                    hover_state.set(next);
                }
            }

            if left
                .borrow()
                .as_ref()
                .is_some_and(|g| matches!(g, LG::Pending(_)))
                && left_pointer.get() != Some(ev.pointer_id())
            {
                return;
            }
            let taken = left.borrow_mut().take();
            let Some(g0) = taken else { return };
            let active = match g0 {
                LG::Pending(p) => {
                    let moved = ((px - p.start_x).powi(2) + (py - p.start_y).powi(2)).sqrt();
                    if moved < selection::DRAG_THRESHOLD_PX {
                        *left.borrow_mut() = Some(LG::Pending(p));
                        return;
                    }
                    if !selection::may_promote_pending(ev.buttons()) {
                        return;
                    }
                    let _ = container.set_pointer_capture(ev.pointer_id());
                    let sw = p.cam.unproject_xy(p.start_x, p.start_y);
                    let on_ring = widget_variant.get_untracked().is_rotate()
                        && !selection.borrow().is_empty()
                        && read_widget_pivot()
                            .map(|(wx, wy)| p.cam.project([wx, wy, 0.0]))
                            .filter(|pv| pv[0].is_finite() && pv[1].is_finite())
                            .is_some_and(|pv| {
                                transform::press_on_ring(p.start_x, p.start_y, pv[0], pv[1])
                            });
                    if on_ring {
                        LG::Rotate {
                            start_x: p.start_x,
                            start_y: p.start_y,
                            cam: p.cam,
                        }
                    } else {
                        let mut z_arm_hit = false;
                        if widget_variant.get_untracked()
                            == crate::v2::apps::editor::mission_editor::transform::WidgetVariant::Translate
                            && !selection.borrow().is_empty()
                        {
                            if let Some(pv) = read_widget_pivot()
                                .map(|(wx, wy)| p.cam.project([wx, wy, 0.0]))
                                .filter(|pv| pv[0].is_finite() && pv[1].is_finite())
                            {
                                z_arm_hit = crate::v2::apps::editor::bridge::gizmo_z::hit_z_arm(
                                    p.start_x, p.start_y, pv[0], pv[1], 1.0,
                                );
                            }
                        }
                        if z_arm_hit {
                            let cur_sel = selection.borrow().clone();
                            *z_drag.borrow_mut() = doc.borrow().as_ref().and_then(|core| {
                                ov::ZDrag::begin(
                                    core,
                                    &cur_sel,
                                    ev.pointer_id(),
                                    p.start_y,
                                    p.cam.scale(),
                                )
                            });
                            let _ = container.set_pointer_capture(ev.pointer_id());
                            left.borrow_mut().take(); // consume the gesture
                            return;
                        }

                        let hit = doc.borrow().as_ref().and_then(|c| {
                            selection::pick_slot_or_vehicle(
                                &p.cam,
                                &map_render_slot_soa(c),
                                &engine_ops::vehicle_points(),
                                p.start_x,
                                p.start_y,
                            )
                        });
                        let hit = hit.or_else(|| {
                            let w = p.cam.unproject_xy(p.start_x, p.start_y);
                            let w2 = p.cam.unproject_xy(p.start_x + COMMENT_PICK_PX, p.start_y);
                            let tol = (w2[0] - w[0]).hypot(w2[1] - w[1]);
                            doc.borrow().as_ref().and_then(|c| {
                                pick_comment(&comment_points(&c.comments_json()), w[0], w[1], tol)
                            })
                        });
                        match hit {
                            Some(ref id)
                                if ev.shift_key() && selection.borrow().iter().any(|s| s == id) =>
                            {
                                LG::Rotate {
                                    start_x: p.start_x,
                                    start_y: p.start_y,
                                    cam: p.cam,
                                }
                            }
                            Some(id) => {
                                let cur = selection.borrow().clone();
                                let ids = selection::compute_move_ids(&id, &cur);
                                if !cur.iter().any(|s| *s == id) {
                                    *selection.borrow_mut() = ids.clone();
                                    if let Some(e) = engine.borrow_mut().as_mut() {
                                        let slot_ids: Vec<String> = ids
                                            .iter()
                                            .filter(|i| !engine_ops::is_vehicle_id(i))
                                            .cloned()
                                            .collect();
                                        e.set_selection(slot_ids);
                                    }
                                }
                                LG::Move {
                                    ids,
                                    start_wx: sw[0],
                                    start_wy: sw[1],
                                    cam: p.cam,
                                    dx: 0.0,
                                    dy: 0.0,
                                }
                            }
                            None => LG::Marquee {
                                start_x: p.start_x,
                                start_y: p.start_y,
                                start_wx: sw[0],
                                start_wy: sw[1],
                                cam: p.cam,
                            },
                        }
                    } // end `else` — non-ring drag (T-795 WIDGET-ROTATE-RING short-circuits above)
                }
                other => other,
            };
            let next = match active {
                LG::Move {
                    ids,
                    start_wx,
                    start_wy,
                    cam,
                    ..
                } => {
                    let (dx, dy) = selection::drag_delta(&cam, start_wx, start_wy, px, py);
                    if let Some(e) = engine.borrow_mut().as_mut() {
                        crate::v2::apps::editor::input::tools::select_tool::push_drag_preview(
                            e,
                            &ids,
                            &engine_ops::vehicle_points(),
                            dx,
                            dy,
                        );
                        let lane = doc.borrow().as_ref().map(|c| {
                            let cj = c.comments_json();
                            (
                                comment_drag_lane_xy(&cj, &ids, dx, dy),
                                comment_lane_ids(&cj),
                            )
                        });
                        if let Some((cxy, cids)) = lane {
                            e.comments_bind_ids(&cxy, cids);
                        }
                    }
                    LG::Move {
                        ids,
                        start_wx,
                        start_wy,
                        cam,
                        dx,
                        dy,
                    }
                }
                LG::Marquee {
                    start_x,
                    start_y,
                    start_wx,
                    start_wy,
                    cam,
                } => {
                    let end = cam.unproject_xy(px, py);
                    if end[0].is_finite() && end[1].is_finite() {
                        let (min_x, max_x) = (start_wx.min(end[0]), start_wx.max(end[0]));
                        let (min_y, max_y) = (start_wy.min(end[1]), start_wy.max(end[1]));
                        if let Some(e) = engine.borrow_mut().as_mut() {
                            e.upload_marquee(min_x, min_y, max_x, max_y, true);
                        }
                    }
                    LG::Marquee {
                        start_x,
                        start_y,
                        start_wx,
                        start_wy,
                        cam,
                    }
                }
                LG::Pending(p) => LG::Pending(p),
                LG::Ruler {
                    start_x,
                    start_y,
                    cam,
                } => LG::Ruler {
                    start_x,
                    start_y,
                    cam,
                },
                LG::Rotate {
                    start_x,
                    start_y,
                    cam,
                } => LG::Rotate {
                    start_x,
                    start_y,
                    cam,
                },
            };
            *left.borrow_mut() = Some(next);
        }
    });
    onpointermove
}
