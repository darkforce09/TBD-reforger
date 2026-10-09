//! The pointer-hover hit test over the live mission document, and the canvas cursor it drives.
//!
//! **Role:** answers whether a selectable point lies under a screen pixel — a slot, a placed
//! vehicle or a map comment, each picked with the same tolerance a click uses — caches the point
//! sets per document generation, builds the drawable connection segments from the live
//! document, and writes the cursor the canvas wears.
//! **Position:** part of the bridge. The canvas gesture closures in `input::pointer_gestures` call
//! it on pointer moves; it reads the hosted document handle and the hosted commands' vehicle
//! points, and it writes only the canvas element's `cursor` style.
//! **Signals & state:** none of its own; the caller owns the
//! [`HoverPoints`](crate::bridge::hover_hit_testing::HoverPoints) cache.
//! **Invariants:** the cache is rebuilt whenever the document generation changes, so a hover never
//! answers from a stale point set; the pick tolerance is the click's.

use map_editing_tools::selection;
use mission_editing_commands::hosted_commands as engine_ops;
use mission_editing_session::lanes::comments::{
    COMMENT_PICK_PX, CommentPoint, comment_points, pick_comment,
};
use mission_editing_session::lanes::connections::{ConnSegment, connection_segments};
use mission_editing_session::selection_universe::map_render_slot_soa;

use crate::bridge::document_host::doc_host as mission_doc;
use crate::bridge::pointer_hover::hover_cursor_css;

/// Builds drawable connection segments from the live mission document.
#[must_use]
pub fn live_connection_segments(core: &mission_document::MissionDocCore) -> Vec<ConnSegment> {
    let soa = core.materialize();
    let mut positions: std::collections::HashMap<String, (f64, f64)> =
        std::collections::HashMap::with_capacity(soa.ids.len());
    for (i, id) in soa.ids.iter().enumerate() {
        positions.insert(
            id.clone(),
            (f64::from(soa.xy[i * 2]), f64::from(soa.xy[i * 2 + 1])),
        );
    }
    for (id, x, y) in engine_ops::vehicle_points() {
        positions.insert(id, (x, y));
    }
    connection_segments(&core.connection_rows_json(), &positions)
}

/// Writes the pickability cursor to the canvas element.
pub fn set_map_cursor(canvas: &web_sys::HtmlCanvasElement, pickable: bool) {
    let _ = web_sys::HtmlElement::style(canvas).set_property("cursor", hover_cursor_css(pickable));
}

/// Point sets cached for hover hit testing by document generation.
pub struct HoverPoints {
    tick: u64,
    soa: mission_crdt::soa::SlotSoa,
    vehicles: Vec<(String, f64, f64)>,
    comments: Vec<CommentPoint>,
}

/// Tests whether a rendered selectable point lies under a screen pixel.
pub(crate) fn hover_hit(
    cache: &mut Option<HoverPoints>,
    tick: u64,
    doc: &mission_doc::DocHandle,
    cam: &camera_math::ortho::state::OrthoCamera,
    px: f64,
    py: f64,
) -> bool {
    if cache.as_ref().is_none_or(|c| c.tick != tick) {
        let fresh = doc.borrow().as_ref().map(|c| HoverPoints {
            tick,
            soa: map_render_slot_soa(c),
            vehicles: engine_ops::vehicle_points(),
            comments: comment_points(&c.comments_json()),
        });
        let Some(fresh) = fresh else { return false };
        *cache = Some(fresh);
    }
    let Some(pts) = cache.as_ref() else {
        return false;
    };
    if selection::pick_slot_or_vehicle(cam, &pts.soa, &pts.vehicles, px, py).is_some() {
        return true;
    }
    let w = cam.unproject_xy(px, py);
    let w2 = cam.unproject_xy(px + COMMENT_PICK_PX, py);
    let tol = (w2[0] - w[0]).hypot(w2[1] - w[1]);
    pick_comment(&pts.comments, w[0], w[1], tol).is_some()
}
