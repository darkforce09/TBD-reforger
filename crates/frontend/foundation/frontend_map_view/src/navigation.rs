//! Pointer navigation for a map view that has no editing tools: drag to pan, wheel to zoom,
//! click to pick a map position.
//!
//! **Role:** turns primary-button drags into camera pans, wheel travel into cursor-anchored
//! zoom, and a press released in place into a [`MapClick`] in map metres with the ground height
//! under it.
//! **Position:** attached by [`super::mount::mount_map_view`] for map pickers. The Mission
//! Creator routes its own pointer gestures (tools, selection, placement) and does not attach
//! this.
//! **Signals & state:** the in-flight press (pointer id, press point, last point, whether it has
//! become a drag) lives in the listeners; the camera lives in the engine.
//! **Invariants:** every camera change marks the engine and schedules the map host's viewport
//! settle; a drag never also fires a click; nothing runs after the view is disposed.

use super::handles::MapViewHandles;
use super::navigation_math::{is_click, map_metres_at, wheel_zoom_delta};
use std::cell::Cell;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;

/// A click on the map: the position in map metres and the ground height there, when the
/// full-resolution heights are loaded.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MapClick {
    /// Easting in metres.
    pub x: f64,

    /// Northing in metres.
    pub y: f64,

    /// Ground height in metres, or `None` when no full-resolution raster covers the point.
    pub height_m: Option<f64>,
}

/// The press a pointer-up resolves.
#[derive(Clone, Copy)]
struct Press {
    pointer_id: i32,
    down: (f64, f64),
    last: (f64, f64),
    dragging: bool,
}

/// Mark the camera moved and let the map host refresh what the new view needs.
fn camera_moved(handles: &MapViewHandles) {
    if let Some(e) = handles.engine.borrow_mut().as_mut() {
        e.on_camera_changed();
    }
    map_streaming_host::schedule_camera_settle(handles.map_host.clone(), handles.engine.clone());
}

/// Attach drag-pan, wheel-zoom and click-to-pick listeners to `container`.
pub fn attach_navigation(
    container: &web_sys::HtmlElement,
    handles: &MapViewHandles,
    on_click: Rc<dyn Fn(MapClick)>,
) {
    let press: Rc<Cell<Option<Press>>> = Rc::new(Cell::new(None));

    let on_down = Closure::<dyn FnMut(web_sys::PointerEvent)>::new({
        let press = press.clone();
        let container = container.clone();
        let handles = handles.clone();
        move |ev: web_sys::PointerEvent| {
            if handles.is_disposed() || ev.button() != 0 {
                return;
            }
            let at = (f64::from(ev.client_x()), f64::from(ev.client_y()));
            let _ = container.set_pointer_capture(ev.pointer_id());
            press.set(Some(Press {
                pointer_id: ev.pointer_id(),
                down: at,
                last: at,
                dragging: false,
            }));
        }
    });

    let on_move = Closure::<dyn FnMut(web_sys::PointerEvent)>::new({
        let press = press.clone();
        let handles = handles.clone();
        move |ev: web_sys::PointerEvent| {
            let Some(mut p) = press.get() else {
                return;
            };
            if handles.is_disposed() || ev.pointer_id() != p.pointer_id {
                return;
            }
            let at = (f64::from(ev.client_x()), f64::from(ev.client_y()));
            if !p.dragging && is_click(p.down, at) {
                return;
            }
            if !p.dragging {
                p.dragging = true;
                map_streaming_host::set_camera_gesture(true);
            }
            if let Some(e) = handles.engine.borrow_mut().as_mut() {
                e.pan(at.0 - p.last.0, at.1 - p.last.1);
            }
            p.last = at;
            press.set(Some(p));
            camera_moved(&handles);
        }
    });

    let on_up = Closure::<dyn FnMut(web_sys::PointerEvent)>::new({
        let press = press.clone();
        let container = container.clone();
        let handles = handles.clone();
        move |ev: web_sys::PointerEvent| {
            let Some(p) = press.take() else {
                return;
            };
            if container.has_pointer_capture(ev.pointer_id()) {
                let _ = container.release_pointer_capture(ev.pointer_id());
            }
            if p.dragging {
                map_streaming_host::set_camera_gesture(false);
                camera_moved(&handles);
                return;
            }
            if handles.is_disposed() || ev.pointer_id() != p.pointer_id {
                return;
            }
            let rect = container.get_bounding_client_rect();
            let Some(view) = handles.view_state() else {
                return;
            };
            let px = f64::from(ev.client_x()) - rect.left();
            let py = f64::from(ev.client_y()) - rect.top();
            if let Some((x, y)) = map_metres_at(rect.width(), rect.height(), view, px, py) {
                on_click(MapClick {
                    x,
                    y,
                    height_m: handles.height_at(x, y),
                });
            }
        }
    });

    let on_cancel = Closure::<dyn FnMut(web_sys::PointerEvent)>::new({
        let press = press.clone();
        move |_ev: web_sys::PointerEvent| {
            if press.take().is_some_and(|p| p.dragging) {
                map_streaming_host::set_camera_gesture(false);
            }
        }
    });

    let on_wheel = Closure::<dyn FnMut(web_sys::WheelEvent)>::new({
        let container = container.clone();
        let handles = handles.clone();
        move |ev: web_sys::WheelEvent| {
            if handles.is_disposed() {
                return;
            }
            ev.prevent_default();
            let rect = container.get_bounding_client_rect();
            if let Some(e) = handles.engine.borrow_mut().as_mut() {
                e.zoom_at(
                    wheel_zoom_delta(ev.delta_y()),
                    f64::from(ev.client_x()) - rect.left(),
                    f64::from(ev.client_y()) - rect.top(),
                );
            }
            camera_moved(&handles);
        }
    });

    let target: &web_sys::EventTarget = container.as_ref();
    for (name, listener) in [
        ("pointerdown", on_down.as_ref()),
        ("pointermove", on_move.as_ref()),
        ("pointerup", on_up.as_ref()),
        ("pointercancel", on_cancel.as_ref()),
    ] {
        let _ = target.add_event_listener_with_callback(name, listener.unchecked_ref());
    }
    let wheel_options = web_sys::AddEventListenerOptions::new();
    wheel_options.set_passive(false);
    let _ = target.add_event_listener_with_callback_and_add_event_listener_options(
        "wheel",
        on_wheel.as_ref().unchecked_ref(),
        &wheel_options,
    );
    on_down.forget();
    on_move.forget();
    on_up.forget();
    on_cancel.forget();
    on_wheel.forget();
}
