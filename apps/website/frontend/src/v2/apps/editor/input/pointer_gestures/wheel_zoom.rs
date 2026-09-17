//! Wheel zoom handler for editor canvas gestures.

use super::*;

/// Builds the wheel zoom event closure for the canvas.
pub(super) fn make_wheel_handler(
    ctx: &EditorGestureContext,
) -> Closure<dyn FnMut(web_sys::WheelEvent)> {
    let engine = ctx.engine.clone();
    let container = ctx.container.clone();
    let pan_px = ctx.pan_px.clone();
    let map_host = ctx.map_host.clone();
    let left = ctx.left.clone();
    let onwheel = Closure::<dyn FnMut(web_sys::WheelEvent)>::new({
        let engine = engine.clone();
        let container = container.clone();
        let pan_px = pan_px.clone();
        let map_host = map_host.clone();
        move |ev: web_sys::WheelEvent| {
            if ev
                .target()
                .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
                .is_some_and(|el| el.closest(CHROME_SEL).ok().flatten().is_some())
            {
                return;
            }
            if let Some(e) = engine.borrow_mut().as_mut() {
                ev.prevent_default();
                let rect = container.get_bounding_client_rect();
                e.zoom_at(
                    -ev.delta_y() * WHEEL_ZOOM_PER_PX,
                    ev.client_x() as f64 - rect.left(),
                    ev.client_y() as f64 - rect.top(),
                );
                if pan_px.get().is_some() {
                    pan_px.set(Some((ev.client_x() as f64, ev.client_y() as f64)));
                }
                e.on_camera_changed();
                website_map_engine::streaming::host::schedule_camera_settle(
                    map_host.clone(),
                    engine.clone(),
                );
            }
        }
    });
    onwheel
}
