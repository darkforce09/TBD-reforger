//! Context menu handler for editor canvas gestures.

use super::*;

/// Builds the context menu event closure for the canvas.
pub(super) fn make_context_menu_handler(
    ctx: &EditorGestureContext,
) -> Closure<dyn FnMut(web_sys::MouseEvent)> {
    let container = ctx.container.clone();
    let engine = ctx.engine.clone();
    let doc = ctx.doc.clone();
    let selection = ctx.selection.clone();
    Closure::<dyn FnMut(web_sys::MouseEvent)>::new({
        let container = container.clone();
        let engine = engine.clone();
        let doc = doc.clone();
        let selection = selection.clone();
        move |ev: web_sys::MouseEvent| {
            ev.prevent_default();
            if tactical_graphics_authoring::tactical_draw_armed() {
                tactical_graphics_authoring::complete_tactical_draw();
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
            let sel = selection.borrow().clone();
            let world = cam.unproject_xy(px, py);
            // The menu is the workspace's: its target resolution and its open run behind the
            // opener the workspace registers at page mount.
            crate::input::context_menu_opener::open_context_menu(
                crate::input::context_menu_opener::ContextMenuRequest {
                    client_x: ev.client_x() as f64,
                    client_y: ev.client_y() as f64,
                    hit,
                    selection: sel,
                    world: (world[0], world[1]),
                },
            );
        }
    })
}
