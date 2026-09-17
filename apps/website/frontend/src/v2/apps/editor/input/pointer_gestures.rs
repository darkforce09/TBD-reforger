//! The Mission Creator's canvas gesture closures.
//!
//! **Role:** owns the six pointer-family closures the map surface answers — wheel zoom,
//! pointerdown / pointermove / pointerup (pan, the LMB Pending→Move/Marquee/Ruler/Rotate machine
//! and the armed place), contextmenu and dblclick.
//! **Position:** the pointer half of [`super`], beside the keyboard dispatch in
//! [`super::window_keydown`]. Both ride [`EditorGestureContext`], which carries every `!Send`
//! handle and `Copy` signal the closures capture; the page builds that context once its handles
//! exist and calls [`attach_canvas_gestures`].
//! **Signals & state:** the in-flight gesture (the frozen camera, the pending promotion, the drag
//! preview) is tab-local and lives only for the duration of the gesture. A committed change
//! reaches the document through `website_map_engine::editing`'s hosted commands, so one gesture
//! files one undo step.
//! **Invariants:** everything here touches `web_sys` over live engine and document handles, so
//! the module is wasm-only and its `pub mod` line carries the same gate. What the canvas draws and
//! what a pick resolves against come from one read of the document, never from two.
//!
//! Not here: `keydown` (the sibling [`super::window_keydown`] rides this same context),
//! `pointercancel` / `pointerleave` / `resize` (page-side, beside the boot tasks), and the view
//! template.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use website_map_engine::editing::tools::selection;

use leptos::prelude::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use website_map_engine::editing::tools::line_of_sight::capture::{
    LosMode, LosState, ViewshedState,
};
use website_map_engine::editing::tools::line_of_sight::viewshed_texture::place_viewshed;

use crate::v2::apps::editor::bridge::document_host::history as mission_history;
use crate::v2::apps::editor::bridge::host_state::armed_placement;
use crate::v2::apps::editor::bridge::host_state::editor_context;
use crate::v2::apps::editor::bridge::overlays as ov;
use crate::v2::apps::editor::bridge::tactical_graphics::{TG_PICK_PX, TG_VERTEX_PICK_PX};
use crate::v2::apps::editor::bridge::tactical_graphics_authoring;
use crate::v2::apps::editor::mission_editor::{
    armed_place, comment_drag_lane_xy, comment_lane_ids, comment_lane_xy, comment_points,
    dragged_comment_points, hover_due, hover_hit, hover_next, hover_suppressed,
    live_connection_segments, map_render_slot_soa, pick_comment, pick_connection,
    read_widget_pivot, set_map_cursor, transform, HoverPoints, HoverState, COMMENT_PICK_PX,
    CONN_PICK_PX,
};
use website_map_engine::data::store::operations::attrs;
use website_map_engine::editing::hosted_commands as engine_ops;
use website_map_engine::editing::hosted_commands::selection_transform;

/// Every handle the six gesture closures capture, bundled so the page hands them over in one
/// `attach_canvas_gestures(&ctx)` call. `Rc`/element handles clone (shared ownership with the
/// page, which keeps using the same cells for its remaining closures — keydown, pointercancel,
/// pointerleave, the boot tasks); `RwSignal`s are `Copy`.
#[derive(Clone)]
pub(crate) struct EditorGestureContext {
    /// The gesture container div (the element every closure measures + captures pointers on).
    pub(crate) container: web_sys::HtmlDivElement,
    /// The map canvas, whose CSS cursor reflects the hover claim.
    pub(crate) canvas: web_sys::HtmlCanvasElement,
    pub(crate) engine: website_map_engine::frame::EngineHandle,
    pub(crate) doc: crate::v2::apps::editor::bridge::document_host::doc_host::DocHandle,
    pub(crate) selection: selection::SelectionHandle,
    /// The in-flight left-button gesture: pending, move, marquee, ruler, or rotate.
    pub(crate) left: Rc<RefCell<Option<selection::LeftGesture>>>,
    /// Last client position while a middle-button pan is active.
    pub(crate) pan_px: Rc<Cell<Option<(f64, f64)>>>,
    pub(crate) map_host: website_map_engine::streaming::host::HostHandle,
    pub(crate) dem_grid: website_map_engine::streaming::host::DemGridHandle,
    /// Session-local ruler polyline, separate from the mission document.
    pub(crate) ruler: Rc<RefCell<website_map_engine::editing::tools::ruler::RulerChain>>,
    /// Two-click line-of-sight capture state.
    pub(crate) los: Rc<RefCell<LosState>>,
    /// Session-local viewshed observer and raster.
    pub(crate) viewshed: Rc<RefCell<ViewshedState>>,
    /// Hover throttle clock, pickable claim, and hysteresis anchor.
    pub(crate) hover_state: Rc<Cell<HoverState>>,
    /// Hover pick points cached for the current document tick.
    pub(crate) hover_points: Rc<RefCell<Option<HoverPoints>>>,
    /// World point displayed by the cursor readout.
    pub(crate) cursor: RwSignal<Option<(f64, f64, Option<f64>)>>,
    /// Active editor tool: Select, Ruler, or Line of Sight.
    pub(crate) tool_mode: RwSignal<website_map_engine::editing::tools::ruler::EditorTool>,
    /// Line-of-sight submode: ray or viewshed.
    pub(crate) los_mode: RwSignal<LosMode>,
    /// Snap grid state used by transform commits.
    pub(crate) snap: RwSignal<transform::SnapState>,
    /// Transform widget variant used by the rotation hit test.
    pub(crate) widget_variant: RwSignal<transform::WidgetVariant>,
    /// Selected map connection edge, if any.
    pub(crate) selected_connection: RwSignal<Option<String>>,
    /// The doc-change tick `editor_context::refresh_docks` bumps (keys the hover point cache).
    pub(crate) doc_tick: RwSignal<u64>,
    /// Ruler status-bar readout.
    pub(crate) ruler_status: RwSignal<Option<String>>,
    /// Ruler repaint tick.
    pub(crate) ruler_tick: RwSignal<u64>,
    /// Line-of-sight repaint tick.
    pub(crate) los_tick: RwSignal<u64>,
    /// Backspace hide-interface latch for editor chrome.
    pub(crate) chrome_hidden: RwSignal<bool>,
    /// Entity List dock collapse latch toggled by E.
    pub(crate) dock_left_collapsed: RwSignal<bool>,
    /// Asset Browser dock collapse latch toggled by R.
    pub(crate) dock_right_collapsed: RwSignal<bool>,
    /// Telemetry HUD visibility toggled by Ctrl/Cmd+Alt+D.
    pub(crate) debug_hud_shown: RwSignal<bool>,
}

/// Synchronizes the ruler status readout and repaint tick from the shared chain.
pub(super) fn make_sync_ruler(ctx: &EditorGestureContext) -> impl Fn() + Clone {
    let ruler = ctx.ruler.clone();
    let ruler_status = ctx.ruler_status;
    let ruler_tick = ctx.ruler_tick;
    move || {
        ruler_status.set(ruler.borrow().status_readout());
        ruler_tick.update(|t| *t = t.wrapping_add(1));
    }
}

/// Bumps the line-of-sight repaint tick after capture state changes.
pub(super) fn make_sync_los(ctx: &EditorGestureContext) -> impl Fn() + Copy {
    let los_tick = ctx.los_tick;
    move || {
        los_tick.update(|t| *t = t.wrapping_add(1));
    }
}

const WHEEL_ZOOM_PER_PX: f64 = 1.0 / 500.0;
/// Selects editor chrome that handles wheel scrolling instead of map zoom.
const CHROME_SEL: &str = "[data-eden-chrome]";

mod context_menu;
mod double_click;
mod pointer_down;
mod pointer_move;
mod pointer_up;
mod wheel_zoom;

/// Attaches wheel, pointer, context-menu, and double-click gestures to the canvas container.
/// Cancellation listeners share the elevation and tactical vertex drag latches with the handlers.
pub(crate) fn attach_canvas_gestures(ctx: &EditorGestureContext) {
    let container = ctx.container.clone();
    let tool_mode = ctx.tool_mode;
    let widget_variant = ctx.widget_variant;
    let z_drag = Rc::new(RefCell::new(None::<ov::ZDrag>));
    let vertex_pointer = Rc::new(Cell::new(None::<i32>));
    let left_pointer = Rc::new(Cell::new(None::<i32>));
    let cancel_z: Rc<dyn Fn(Option<i32>)> = Rc::new({
        let z_drag = z_drag.clone();
        let vertex_pointer = vertex_pointer.clone();
        let container = container.clone();
        move |pointer| {
            let taken = {
                let mut drag = z_drag.borrow_mut();
                match pointer {
                    Some(id) => ov::take_z_drag(&mut drag, id),
                    None => drag.take(),
                }
            };
            if let Some(arm) = taken {
                ov::set_z_drag_readout(None);
                if container.has_pointer_capture(arm.pointer_id) {
                    let _ = container.release_pointer_capture(arm.pointer_id);
                }
            }
            if vertex_pointer
                .get()
                .is_some_and(|id| pointer.is_none_or(|p| p == id))
            {
                if let Some(id) = vertex_pointer.take() {
                    if container.has_pointer_capture(id) {
                        let _ = container.release_pointer_capture(id);
                    }
                }
                if tactical_graphics_authoring::cancel_tactical_vertex_drag() {
                    mission_history::refresh_tactical_lane();
                }
            }
        }
    });
    let cancel_pointer = Closure::<dyn FnMut(web_sys::PointerEvent)>::new({
        let cancel_z = cancel_z.clone();
        move |ev: web_sys::PointerEvent| cancel_z(Some(ev.pointer_id()))
    });
    let cancel_blur = Closure::<dyn FnMut(web_sys::Event)>::new({
        let cancel_z = cancel_z.clone();
        move |_| cancel_z(None)
    });
    let cancel_escape = Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new({
        let cancel_z = cancel_z.clone();
        let gesture_active = Signal::derive_local({
            let z_drag = z_drag.clone();
            let vertex_pointer = vertex_pointer.clone();
            move || z_drag.borrow().is_some() || vertex_pointer.get().is_some()
        });
        move |ev: web_sys::KeyboardEvent| {
            if ev.key() == "Escape" && gesture_active.get_untracked() {
                cancel_z(None);
            }
        }
    });
    for event in ["pointercancel", "lostpointercapture"] {
        let _ = container
            .add_event_listener_with_callback(event, cancel_pointer.as_ref().unchecked_ref());
    }
    if let Some(win) = web_sys::window() {
        let _ = win.add_event_listener_with_callback("blur", cancel_blur.as_ref().unchecked_ref());
        let _ =
            win.add_event_listener_with_callback("keydown", cancel_escape.as_ref().unchecked_ref());
    }
    let cleanup = StoredValue::new_local((
        container.clone(),
        cancel_z.clone(),
        cancel_pointer,
        cancel_blur,
        cancel_escape,
    ));
    on_cleanup(move || {
        let _ = cleanup.try_with_value(|(container, cancel, pointer, blur, escape)| {
            cancel(None);
            for event in ["pointercancel", "lostpointercapture"] {
                let _ = container
                    .remove_event_listener_with_callback(event, pointer.as_ref().unchecked_ref());
            }
            if let Some(win) = web_sys::window() {
                let _ =
                    win.remove_event_listener_with_callback("blur", blur.as_ref().unchecked_ref());
                let _ = win.remove_event_listener_with_callback(
                    "keydown",
                    escape.as_ref().unchecked_ref(),
                );
            }
        });
    });
    Effect::new(move |_| {
        let _ = (tool_mode.get(), widget_variant.get());
        cancel_z(None);
    });

    let onwheel = wheel_zoom::make_wheel_handler(ctx);
    let wheel_opts = web_sys::AddEventListenerOptions::new();
    wheel_opts.set_passive(false);
    wheel_opts.set_capture(true);
    let _ = container.add_event_listener_with_callback_and_add_event_listener_options(
        "wheel",
        onwheel.as_ref().unchecked_ref(),
        &wheel_opts,
    );

    let onpointerdown =
        pointer_down::make_pointer_down_handler(ctx, &z_drag, &vertex_pointer, &left_pointer);
    let onpointermove =
        pointer_move::make_pointer_move_handler(ctx, &z_drag, &vertex_pointer, &left_pointer);
    let onpointerup = pointer_up::make_pointer_up_handler(ctx, &z_drag, &vertex_pointer);
    let oncontextmenu = context_menu::make_context_menu_handler(ctx);
    let ondblclick = double_click::make_double_click_handler(ctx);
    let _ =
        container.add_event_listener_with_callback("dblclick", ondblclick.as_ref().unchecked_ref());

    let _ = container
        .add_event_listener_with_callback("pointerdown", onpointerdown.as_ref().unchecked_ref());
    let _ = container
        .add_event_listener_with_callback("pointermove", onpointermove.as_ref().unchecked_ref());
    let _ = container
        .add_event_listener_with_callback("pointerup", onpointerup.as_ref().unchecked_ref());
    let _ = container
        .add_event_listener_with_callback("contextmenu", oncontextmenu.as_ref().unchecked_ref());

    onwheel.forget();
    onpointerdown.forget();
    onpointermove.forget();
    onpointerup.forget();
    oncontextmenu.forget();
    ondblclick.forget();
}
