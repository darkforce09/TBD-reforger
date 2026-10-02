//! The browser half of the map picker: the fire-mission lanes on the engine and the marker drag.
//!
//! **Role:** uploads [`super::marks::LaneUploads`] into the engine's marker, connection and zone
//! lanes, and attaches the pointer listeners that grab a placed marker and drag it.
//! **Position:** called by the map picker component (`super`) once per mounted view; the lanes are
//! the ones [`map_engine::overlay::fire_mission_marks`] assigns
//! (`FIRE_MISSION_GLYPH_LANE`, `FIRE_MISSION_LINE_LANE`, `FIRE_MISSION_DISPERSION_LANE`).
//! **Signals & state:** the drag in flight lives in the listeners; the placed positions are the
//! page's draft signals; the marker atlas is uploaded once per engine.
//! **Invariants:** the drag listeners are attached before the map view's navigation, so a press
//! on a marker is stopped before it can start a pan, and a press anywhere else reaches the
//! navigation untouched; an upload with nothing to draw removes its lane; nothing runs once the
//! view is disposed.

use super::marks::{lane_uploads, OverlayScene, GLYPH_HALF_SIZE_PX};
use super::picking::{
    apply_placement, marker_near, placed_markers, Placement, MARKER_HIT_RADIUS_PX,
};
use crate::v2::core::map_view::handles::MapViewHandles;
use crate::v2::core::map_view::navigation_math::map_metres_at;
use crate::v2::pages::field_tools::mortar::inputs::battery::GunDraft;
use crate::v2::pages::field_tools::mortar::inputs::positions::PositionDraft;
use leptos::prelude::*;
use map_engine::overlay::lanes::role_id::MISSION_ZONES;
use std::cell::Cell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

/// Map metres at CSS pixel `(px, py)` of `container`, and the metres one CSS pixel spans there.
fn metres_under(
    container: &web_sys::HtmlElement,
    handles: &MapViewHandles,
    client: (f64, f64),
) -> Option<([f64; 2], f64)> {
    let view = handles.view_state()?;
    let rect = container.get_bounding_client_rect();
    let (px, py) = (client.0 - rect.left(), client.1 - rect.top());
    let (x, y) = map_metres_at(rect.width(), rect.height(), view, px, py)?;
    let (x1, _) = map_metres_at(rect.width(), rect.height(), view, px + 1.0, py)?;
    Some(([x, y], (x1 - x).abs()))
}

/// Metres one CSS pixel spans at the centre of `container`; `None` before the engine exists.
pub(super) fn metres_per_pixel(
    container: &web_sys::HtmlElement,
    handles: &MapViewHandles,
) -> Option<f64> {
    let rect = container.get_bounding_client_rect();
    let centre = (
        rect.left() + rect.width() / 2.0,
        rect.top() + rect.height() / 2.0,
    );
    metres_under(container, handles, centre).map(|(_, per_px)| per_px)
}

/// Draws `scene` into the engine's fire-mission lanes, uploading the marker atlas first when
/// `atlas_ready` is still unset.
pub(super) fn upload_overlay(
    handles: &MapViewHandles,
    container: &web_sys::HtmlElement,
    atlas_ready: &Cell<bool>,
    scene: &OverlayScene,
) {
    if handles.is_disposed() {
        return;
    }
    let Some(per_px) = metres_per_pixel(container, handles) else {
        return;
    };
    let uploads = lane_uploads(scene, GLYPH_HALF_SIZE_PX * per_px);
    let mut slot = handles.engine.borrow_mut();
    let Some(engine) = slot.as_mut() else {
        return;
    };
    if !atlas_ready.get() {
        let (rgba, width, height, uv) =
            map_engine::overlay::symbology::markers::build_marker_slot_atlas();
        match engine.ensure_slot_atlas(&rgba, width, height, &uv) {
            Ok(()) => atlas_ready.set(true),
            Err(_) => leptos::logging::error!("mortar map: the marker atlas did not upload"),
        }
    }
    engine.markers_bind(
        &uploads.glyph_xy,
        &uploads.glyph_rgba,
        uploads.glyph_icons,
        uploads.glyph_captions,
    );
    engine.connections_bind(&uploads.line_packed, uploads.line_segments);
    if uploads.fill_indices.is_empty() {
        engine.clear_vector_lane(MISSION_ZONES);
    } else {
        engine.upload_polygon_mesh(
            MISSION_ZONES,
            &uploads.fill_positions,
            &uploads.fill_colors,
            &uploads.fill_indices,
            uploads.fill_count,
            true,
        );
    }
    engine.mark_dirty();
}

/// The page signals a drag writes.
#[derive(Clone, Copy)]
pub(super) struct DragTargets {
    /// The target draft.
    pub(super) target: RwSignal<PositionDraft>,
    /// The battery drafts.
    pub(super) guns: RwSignal<Vec<GunDraft>>,
    /// The placement the next click writes; a grabbed marker becomes it.
    pub(super) placing: RwSignal<Placement>,
}

/// Attaches the marker grab-and-drag listeners to `container`; call before the view's navigation
/// is attached. `after_zoom` runs on the animation frame after every wheel turn.
pub(super) fn attach_marker_drag(
    container: &web_sys::HtmlElement,
    handles: &MapViewHandles,
    targets: DragTargets,
    after_zoom: Rc<dyn Fn()>,
) {
    let grabbed: Rc<Cell<Option<(i32, Placement)>>> = Rc::new(Cell::new(None));
    let move_to = {
        let container = container.clone();
        let handles = handles.clone();
        move |ev: &web_sys::PointerEvent, placing: Placement| {
            let client = (f64::from(ev.client_x()), f64::from(ev.client_y()));
            if let Some(([x, y], _)) = metres_under(&container, &handles, client) {
                targets.guns.update(|guns| {
                    targets.target.update(|target| {
                        apply_placement(target, guns, placing, x, y);
                    });
                });
            }
        }
    };
    let on_down = Closure::<dyn FnMut(web_sys::PointerEvent)>::new({
        let (grabbed, container, handles) = (grabbed.clone(), container.clone(), handles.clone());
        move |ev: web_sys::PointerEvent| {
            if handles.is_disposed() || ev.button() != 0 {
                return;
            }
            let client = (f64::from(ev.client_x()), f64::from(ev.client_y()));
            let Some((at, per_px)) = metres_under(&container, &handles, client) else {
                return;
            };
            let markers = targets
                .guns
                .with_untracked(|guns| targets.target.with_untracked(|t| placed_markers(t, guns)));
            if let Some(placing) = marker_near(&markers, at, MARKER_HIT_RADIUS_PX * per_px) {
                ev.stop_immediate_propagation();
                ev.prevent_default();
                let _ = container.set_pointer_capture(ev.pointer_id());
                grabbed.set(Some((ev.pointer_id(), placing)));
                targets.placing.set(placing);
            }
        }
    });
    let on_move = Closure::<dyn FnMut(web_sys::PointerEvent)>::new({
        let (grabbed, move_to) = (grabbed.clone(), move_to.clone());
        move |ev: web_sys::PointerEvent| {
            if let Some((pointer_id, placing)) = grabbed.get() {
                if pointer_id == ev.pointer_id() {
                    ev.stop_immediate_propagation();
                    move_to(&ev, placing);
                }
            }
        }
    });
    let on_up = Closure::<dyn FnMut(web_sys::PointerEvent)>::new({
        let (grabbed, container) = (grabbed.clone(), container.clone());
        move |ev: web_sys::PointerEvent| {
            if let Some((pointer_id, placing)) = grabbed.get() {
                if pointer_id == ev.pointer_id() {
                    ev.stop_immediate_propagation();
                    grabbed.set(None);
                    move_to(&ev, placing);
                    if container.has_pointer_capture(pointer_id) {
                        let _ = container.release_pointer_capture(pointer_id);
                    }
                }
            }
        }
    });
    let on_wheel =
        Closure::<dyn FnMut(web_sys::WheelEvent)>::new(move |_ev: web_sys::WheelEvent| {
            let after_zoom = after_zoom.clone();
            request_animation_frame(move || after_zoom());
        });
    let target: &web_sys::EventTarget = container.as_ref();
    for (name, listener) in [
        ("pointerdown", on_down.as_ref()),
        ("pointermove", on_move.as_ref()),
        ("pointerup", on_up.as_ref()),
        ("pointercancel", on_up.as_ref()),
        ("wheel", on_wheel.as_ref()),
    ] {
        let _ = target.add_event_listener_with_callback(name, listener.unchecked_ref());
    }
    on_down.forget();
    on_move.forget();
    on_up.forget();
    on_wheel.forget();
}
