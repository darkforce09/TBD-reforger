//! Double click handler for editor canvas gestures.

use super::*;

/// Builds the double click event closure for the canvas.
pub(super) fn make_double_click_handler(
    ctx: &EditorGestureContext,
) -> Closure<dyn FnMut(web_sys::MouseEvent)> {
    let container = ctx.container.clone();
    let engine = ctx.engine.clone();
    let doc = ctx.doc.clone();
    let selection = ctx.selection.clone();
    let left = ctx.left.clone();
    let ruler = ctx.ruler.clone();
    let tool_mode = ctx.tool_mode;
    let sync_ruler = make_sync_ruler(ctx);
    let ondblclick = Closure::<dyn FnMut(web_sys::MouseEvent)>::new({
        let container = container.clone();
        let engine = engine.clone();
        let doc = doc.clone();
        let ruler = ruler.clone();
        let sync_ruler = sync_ruler.clone();
        move |ev: web_sys::MouseEvent| {
            if ev.button() != 0 {
                return;
            }
            if tool_mode.get_untracked().is_ruler() {
                let mut r = ruler.borrow_mut();
                r.dedup_tail(0.5);
                r.double_click();
                drop(r);
                sync_ruler();
                return;
            }
            if tool_mode.get_untracked().is_los() {
                return;
            }
            let rect = container.get_bounding_client_rect();
            let (px, py) = (
                ev.client_x() as f64 - rect.left(),
                ev.client_y() as f64 - rect.top(),
            );
            let cam = {
                let g = engine.borrow();
                let Some(e) = g.as_ref() else { return };
                selection::frozen_camera(
                    rect.width(),
                    rect.height(),
                    e.target_x(),
                    e.target_y(),
                    e.zoom(),
                )
            };
            let hit = doc.borrow().as_ref().and_then(|c| {
                selection::pick_slot_or_vehicle(
                    &cam,
                    &map_render_slot_soa(c),
                    &engine_ops::vehicle_points(),
                    px,
                    py,
                )
            });
            match hit {
                Some(id) => editor_context::open_attributes(id),
                None => {
                    let world = cam.unproject_xy(px, py);
                    if world[0].is_finite() && world[1].is_finite() {
                        editor_context::open_asset_picker(
                            world[0],
                            world[1],
                            ev.client_x() as f64,
                            ev.client_y() as f64,
                        );
                    }
                }
            }
        }
    });
    ondblclick
}
