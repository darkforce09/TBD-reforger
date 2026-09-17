//! Commit and cancellation paths for elevation and tactical vertex drags.

use super::*;

/// Consumes a release owned by the elevation or tactical vertex drag lane.
/// A release from another pointer is consumed without committing the active drag.
pub(super) fn consume_special_drag(
    ev: &web_sys::PointerEvent,
    z_drag: &Rc<RefCell<Option<ov::ZDrag>>>,
    vertex_pointer: &Rc<Cell<Option<i32>>>,
    container: &web_sys::HtmlDivElement,
    doc: &crate::v2::apps::editor::bridge::document_host::doc_host::DocHandle,
    snap: RwSignal<transform::SnapState>,
) -> bool {
    if z_drag
        .borrow()
        .as_ref()
        .is_some_and(|arm| arm.pointer_id != ev.pointer_id())
    {
        return true;
    }
    let z_arm = ov::take_z_drag(&mut z_drag.borrow_mut(), ev.pointer_id());
    if let Some(arm) = z_arm {
        if container.has_pointer_capture(arm.pointer_id) {
            let _ = container.release_pointer_capture(arm.pointer_id);
        }
        ov::set_z_drag_readout(None);
        let rect = container.get_bounding_client_rect();
        let py = ev.client_y() as f64 - rect.top();
        let delta = ov::z_drag_elevation_delta(
            py,
            arm.start_y,
            arm.scale,
            ov::z_drag_snap_step(snap.get_untracked(), ev.shift_key()),
        );
        let changed = doc
            .borrow_mut()
            .as_mut()
            .is_some_and(|core| arm.commit(core, delta));
        if changed {
            mission_history::after_local_edit();
        }
        return true;
    }
    if tactical_graphics_authoring::tactical_vertex_drag_active() {
        if vertex_pointer.get() != Some(ev.pointer_id()) {
            return true;
        }
        vertex_pointer.set(None);
        if container.has_pointer_capture(ev.pointer_id()) {
            let _ = container.release_pointer_capture(ev.pointer_id());
        }
        if !tactical_graphics_authoring::commit_tactical_vertex_drag() {
            mission_history::refresh_tactical_lane();
        }
        return true;
    }
    false
}
