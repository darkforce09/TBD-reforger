//! Attaches the Mission Creator's canvas gestures, cancellation and resize tracking.
//!
//! **Role:** wires the editor's pointer gestures and hotkeys, the pointer-leave and
//! pointer-cancel resets, and the shared map seam's resize tracking
//! ([`crate::v2::core::map_view::resize::observe_container_resize`]) to the mounted canvas.
//! **Position:** called last by the canvas mount, after the boot tasks started.
//! **Signals & state:** the in-flight pan and the hover state live for the mount; the mount's
//! `disposed` flag is set when the page unmounts.
//! **Invariants:** a cancelled pointer releases its capture and rolls back whatever gesture it
//! held; nothing resizes or draws after disposal.

use super::*;
use crate::v2::apps::editor::bridge::host_state::armed_placement;
use crate::v2::core::map_view::handles::MapViewHandles;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::atomic::Ordering;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

/// Handles and signals used by canvas input listeners.
pub(super) struct InputContext {
    pub container: web_sys::HtmlDivElement,
    pub canvas: web_sys::HtmlCanvasElement,
    pub view: MapViewHandles,
    pub doc: mission_doc::DocHandle,
    pub selection: selection::SelectionHandle,
    pub left: Rc<RefCell<Option<selection::LeftGesture>>>,
    pub ruler: Rc<RefCell<website_map_engine::editing::tools::ruler::RulerChain>>,
    pub los: Rc<RefCell<LosState>>,
    pub viewshed: Rc<RefCell<ViewshedState>>,
    pub cursor: RwSignal<Option<(f64, f64, Option<f64>)>>,
    pub tool_mode: RwSignal<website_map_engine::editing::tools::ruler::EditorTool>,
    pub los_mode: RwSignal<LosMode>,
    pub snap: RwSignal<transform::SnapState>,
    pub widget_variant: RwSignal<transform::WidgetVariant>,
    pub selected_connection: RwSignal<Option<String>>,
    pub doc_tick: RwSignal<u64>,
    pub ruler_status: RwSignal<Option<String>>,
    pub ruler_tick: RwSignal<u64>,
    pub los_tick: RwSignal<u64>,
    pub chrome_hidden: RwSignal<bool>,
    pub dock_left_collapsed: RwSignal<bool>,
    pub dock_right_collapsed: RwSignal<bool>,
    pub debug_hud_shown: RwSignal<bool>,
}

/// Installs pointer, keyboard, and resize listeners for the canvas.
pub(super) fn attach(ctx: InputContext) {
    let InputContext {
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
    } = ctx;
    let engine = view.engine.clone();
    let map_host = view.map_host.clone();
    let dem_grid = view.dem_grid.clone();
    let pan_px: Rc<Cell<Option<(f64, f64)>>> = Rc::new(Cell::new(None));

    let hover_state: Rc<Cell<HoverState>> = Rc::new(Cell::new(HoverState::default()));
    let hover_points: Rc<RefCell<Option<HoverPoints>>> = Rc::new(RefCell::new(None));
    set_map_cursor(&canvas, false);

    let gesture_ctx = crate::v2::apps::editor::input::pointer_gestures::EditorGestureContext {
        container: container.clone(),
        canvas: canvas.clone(),
        engine: engine.clone(),
        doc: doc.clone(),
        selection: selection.clone(),
        left: left.clone(),
        pan_px: pan_px.clone(),
        map_host: map_host.clone(),
        dem_grid: dem_grid.clone(),
        ruler: ruler.clone(),
        los: los.clone(),
        viewshed: viewshed.clone(),
        hover_state: hover_state.clone(),
        hover_points: hover_points.clone(),
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
    };
    crate::v2::apps::editor::input::pointer_gestures::attach_canvas_gestures(&gesture_ctx);
    crate::v2::apps::editor::input::window_keydown::attach_editor_hotkeys(&gesture_ctx);

    let onpointerleave = Closure::<dyn FnMut(web_sys::PointerEvent)>::new({
        let engine = engine.clone();
        let hover_state = hover_state.clone();
        let canvas = canvas.clone();
        move |_ev: web_sys::PointerEvent| {
            cursor.set(None);
            hover_state.set(HoverState::default());
            set_map_cursor(&canvas, false);
            if let Some(e) = engine.borrow_mut().as_mut() {
                e.clear_place_preview();
            }
        }
    });
    let onpointercancel = Closure::<dyn FnMut(web_sys::PointerEvent)>::new({
        let pan_px = pan_px.clone();
        let container = container.clone();
        let left = left.clone();
        let engine = engine.clone();
        let doc = doc.clone(); // re-binds the comment lane on a cancelled drag
        move |ev: web_sys::PointerEvent| {
            armed_placement::cancel_pending();
            engine_ops::cancel_connect();
            if pan_px.get().is_some() {
                pan_px.set(None);
                if container.has_pointer_capture(ev.pointer_id()) {
                    let _ = container.release_pointer_capture(ev.pointer_id());
                }
            }
            use selection::LeftGesture as LG;
            let taken = left.borrow_mut().take();
            match taken {
                Some(LG::Move { .. }) => {
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
                Some(LG::Marquee { .. }) => {
                    if container.has_pointer_capture(ev.pointer_id()) {
                        let _ = container.release_pointer_capture(ev.pointer_id());
                    }
                    if let Some(e) = engine.borrow_mut().as_mut() {
                        e.upload_marquee(0.0, 0.0, 0.0, 0.0, false);
                    }
                }
                Some(LG::Rotate { .. }) if container.has_pointer_capture(ev.pointer_id()) => {
                    let _ = container.release_pointer_capture(ev.pointer_id());
                }
                _ => {}
            }
        }
    });
    let _ = container.add_event_listener_with_callback(
        "pointercancel",
        onpointercancel.as_ref().unchecked_ref(),
    );
    let _ = container
        .add_event_listener_with_callback("pointerleave", onpointerleave.as_ref().unchecked_ref());

    crate::v2::core::map_view::resize::observe_container_resize(&container, &canvas, &view);

    onpointercancel.forget();
    onpointerleave.forget();
    let disposed = view.disposed.clone();
    on_cleanup(move || disposed.store(true, Ordering::Relaxed));
}
