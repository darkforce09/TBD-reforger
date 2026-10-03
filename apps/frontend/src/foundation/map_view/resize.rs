//! Keeps a map view's canvas and engine sized to its container.
//!
//! **Role:** re-sizes the canvas backing store and the engine surface whenever the container's
//! box changes (a `ResizeObserver`) or the window resizes (which also covers a device pixel
//! ratio change from browser zoom).
//! **Position:** attached once per mount, after the canvas is first sized, by the Mission
//! Creator's input listeners and by [`super::mount::mount_map_view`].
//! **Signals & state:** the observer and the window listener live for the page; both read the
//! mount's `disposed` flag.
//! **Invariants:** the canvas and the engine are sized from one measurement
//! ([`super::engine_mount::size_canvas`]); after disposal the observer disconnects itself at its
//! next callback and the window listener does nothing.

use super::engine_mount::size_canvas;
use super::handles::MapViewHandles;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

/// Watch `container` and the window, re-sizing `canvas` and the engine in `handles` on change.
pub fn observe_container_resize(
    container: &web_sys::Element,
    canvas: &web_sys::HtmlCanvasElement,
    handles: &MapViewHandles,
) {
    let resize_now = {
        let container = container.clone();
        let canvas = canvas.clone();
        let handles = handles.clone();
        move || {
            let size = size_canvas(&container, &canvas);
            if let Some(e) = handles.engine.borrow_mut().as_mut() {
                let _ = e.resize(size.css_w, size.css_h, size.dpr);
            }
        }
    };

    let on_box_change = Closure::<dyn FnMut(js_sys::Array, web_sys::ResizeObserver)>::new({
        let resize_now = resize_now.clone();
        let handles = handles.clone();
        move |_entries: js_sys::Array, observer: web_sys::ResizeObserver| {
            if handles.is_disposed() {
                observer.disconnect();
                return;
            }
            resize_now();
        }
    });
    if let Ok(observer) = web_sys::ResizeObserver::new(on_box_change.as_ref().unchecked_ref()) {
        observer.observe(container);
    }
    on_box_change.forget();

    let on_window_resize = Closure::<dyn FnMut()>::new({
        let handles = handles.clone();
        move || {
            if !handles.is_disposed() {
                resize_now();
            }
        }
    });
    if let Some(win) = web_sys::window() {
        let _ = win
            .add_event_listener_with_callback("resize", on_window_resize.as_ref().unchecked_ref());
    }
    on_window_resize.forget();
}
