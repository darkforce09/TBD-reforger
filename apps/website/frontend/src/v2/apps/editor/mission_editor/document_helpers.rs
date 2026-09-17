//! Document helpers.
use super::*;

#[cfg(target_arch = "wasm32")]
#[must_use]
/// Builds drawable connection segments from the live mission document.
pub(crate) fn live_connection_segments(
    core: &website_map_engine::data::store::MissionDocCore,
) -> Vec<ConnSegment> {
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

#[cfg(target_arch = "wasm32")]
/// Writes the pickability cursor to the canvas element.
pub(crate) fn set_map_cursor(canvas: &web_sys::HtmlCanvasElement, pickable: bool) {
    let _ = web_sys::HtmlElement::style(canvas).set_property("cursor", hover_cursor_css(pickable));
}

#[cfg(target_arch = "wasm32")]
/// Point sets cached for hover hit testing by document generation.
pub(crate) struct HoverPoints {
    tick: u64,
    soa: website_map_engine::data::store::SlotSoa,
    vehicles: Vec<(String, f64, f64)>,
    comments: Vec<CommentPoint>,
}

#[cfg(target_arch = "wasm32")]
/// Tests whether a rendered selectable point lies under a screen pixel.
pub(crate) fn hover_hit(
    cache: &mut Option<HoverPoints>,
    tick: u64,
    doc: &mission_doc::DocHandle,
    cam: &website_map_engine::camera::ortho::state::OrthoCamera,
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

#[cfg(target_arch = "wasm32")]
/// Maps a document subject to its route and world position.
pub(crate) type SubjectResolver = std::rc::Rc<dyn Fn(&str) -> Option<(RouteTarget, f64, f64)>>;

#[cfg(target_arch = "wasm32")]
/// Runs an Arrange command when enough entities are selected.
pub(crate) fn arrange_chord(kind: top_strip::ArrangeKind) -> bool {
    if website_map_engine::editing::host::selection_len() < top_strip::ARRANGE_MIN_SELECTION {
        return false;
    }
    top_strip::run_arrange(kind);
    true
}
