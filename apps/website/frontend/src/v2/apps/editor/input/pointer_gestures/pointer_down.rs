//! Pointer down handler for editor canvas gestures.

use super::*;

/// Builds the pointer down event closure for the canvas.
pub(super) fn make_pointer_down_handler(
    ctx: &EditorGestureContext,
    z_drag: &Rc<RefCell<Option<ov::ZDrag>>>,
    vertex_pointer: &Rc<Cell<Option<i32>>>,
    left_pointer: &Rc<Cell<Option<i32>>>,
) -> Closure<dyn FnMut(web_sys::PointerEvent)> {
    let container = ctx.container.clone();
    let engine = ctx.engine.clone();
    let selection = ctx.selection.clone();
    let left = ctx.left.clone();
    let pan_px = ctx.pan_px.clone();
    let ruler = ctx.ruler.clone();
    let tool_mode = ctx.tool_mode;
    let onpointerdown = Closure::<dyn FnMut(web_sys::PointerEvent)>::new({
        let z_drag = z_drag.clone();
        let vertex_pointer = vertex_pointer.clone();
        let left_pointer = left_pointer.clone();
        let pan_px = pan_px.clone();
        let container = container.clone();
        let engine = engine.clone();
        let left = left.clone();
        move |ev: web_sys::PointerEvent| {
            if z_drag.borrow().is_some() || vertex_pointer.get().is_some() {
                return;
            }
            if ev.button() == 1 {
                ev.prevent_default();
                let _ = container.set_pointer_capture(ev.pointer_id());
                pan_px.set(Some((ev.client_x() as f64, ev.client_y() as f64)));
                website_map_engine::streaming::host::set_camera_gesture(true);
            } else if ev.button() == 0 {
                if armed_placement::has_pending() {
                    return;
                }
                if tactical_graphics_authoring::tactical_draw_armed() {
                    if let Some(e) = engine.borrow().as_ref() {
                        let rect = container.get_bounding_client_rect();
                        let cam = selection::frozen_camera(
                            rect.width(),
                            rect.height(),
                            e.target_x(),
                            e.target_y(),
                            e.zoom(),
                        );
                        let w = cam.unproject_xy(
                            ev.client_x() as f64 - rect.left(),
                            ev.client_y() as f64 - rect.top(),
                        );
                        tactical_graphics_authoring::tactical_draw_push_vertex(w[0], w[1]);
                    }
                    return;
                }
                if let Some(e) = engine.borrow().as_ref() {
                    let rect = container.get_bounding_client_rect();
                    let cam = selection::frozen_camera(
                        rect.width(),
                        rect.height(),
                        e.target_x(),
                        e.target_y(),
                        e.zoom(),
                    );
                    let sx = ev.client_x() as f64 - rect.left();
                    let sy = ev.client_y() as f64 - rect.top();
                    let w = cam.unproject_xy(sx, sy);
                    let w2 = cam.unproject_xy(sx + TG_VERTEX_PICK_PX, sy);
                    let tol = (w2[0] - w[0]).hypot(w2[1] - w[1]);
                    if tactical_graphics_authoring::begin_tactical_vertex_drag(w[0], w[1], tol) {
                        vertex_pointer.set(Some(ev.pointer_id()));
                        let _ = container.set_pointer_capture(ev.pointer_id());
                        return;
                    }
                }
                if let Some(e) = engine.borrow().as_ref() {
                    let rect = container.get_bounding_client_rect();
                    let cam = selection::frozen_camera(
                        rect.width(),
                        rect.height(),
                        e.target_x(),
                        e.target_y(),
                        e.zoom(),
                    );
                    let sx = ev.client_x() as f64 - rect.left();
                    let sy = ev.client_y() as f64 - rect.top();
                    left_pointer.set(Some(ev.pointer_id()));
                    *left.borrow_mut() = Some(
                        if website_map_engine::editing::tools::ruler::should_begin_ruler(
                            tool_mode.get_untracked(),
                            ev.button(),
                        ) {
                            selection::LeftGesture::Ruler {
                                start_x: sx,
                                start_y: sy,
                                cam,
                            }
                        } else {
                            selection::LeftGesture::Pending(selection::PendingLeft {
                                start_x: sx,
                                start_y: sy,
                                cam,
                            })
                        },
                    );
                }
            }
        }
    });
    onpointerdown
}
