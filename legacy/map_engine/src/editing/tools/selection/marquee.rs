//! Role: rectangle selection over slots and vehicles, and the in-view census behind select-all.
//! Position: `editing/tools/selection` in the map engine.
//! Signals & state: a frozen camera snapshot and the app-side selected-id set; never the document.
//! Invariants: the world box is ordered before it is queried, so a drag in any direction selects; slot hits come first and vehicle hits follow, each in its own query's order.

use crate::camera::ortho::state::OrthoCamera;
use crate::data::store::SlotSoa;

/// Slot ids inside the marquee box, from the two frozen-cam screen corners. The press corner is
/// already unprojected to `(start_wx, start_wy)`; this unprojects the release px `(end_px, end_py)`,
/// forms the **ordered** world AABB (the drag can go any direction — `PointIndex::pick_rect` returns
/// empty on `max < min`), then maps the returned handles to `soa.ids`. Mirrors React
/// `slotSpatialIndex.pickRect(startWorld, endWorld)` (`useSelectTool.ts:293`). A singular pixel
/// matrix (NaN unproject on either corner) yields no selection.
///
/// T-491 — implementation lives on [`MissionDocCore::marquee_slot_ids`].
#[must_use]
pub fn marquee_ids(
    cam: &OrthoCamera,
    soa: &SlotSoa,
    start_wx: f64,
    start_wy: f64,
    end_px: f64,
    end_py: f64,
) -> Vec<String> {
    crate::editing::picking::marquee_slot_ids(cam, soa, start_wx, start_wy, end_px, end_py)
}

/// T-425 — vehicle ids inside the marquee world AABB (same corners as [`marquee_ids`]).
/// Delegates to [`map_engine_core::doc::MissionDocCore::marquee_vehicle_ids`] (Class-R SoT).
#[must_use]
#[allow(dead_code)] // public host helper; live path uses marquee_ids_with_vehicles
pub fn marquee_vehicle_ids(
    cam: &OrthoCamera,
    points: &[(String, f64, f64)],
    start_wx: f64,
    start_wy: f64,
    end_px: f64,
    end_py: f64,
) -> Vec<String> {
    crate::editing::picking::marquee_vehicle_ids(cam, points, start_wx, start_wy, end_px, end_py)
}

/// T-425 — marquee over slots **and** placed vehicles (vehicles appended after slots).
#[must_use]
pub fn marquee_ids_with_vehicles(
    cam: &OrthoCamera,
    soa: &SlotSoa,
    vehicle_points: &[(String, f64, f64)],
    start_wx: f64,
    start_wy: f64,
    end_px: f64,
    end_py: f64,
) -> Vec<String> {
    crate::editing::picking::marquee_ids_with_vehicles(
        cam,
        soa,
        vehicle_points,
        start_wx,
        start_wy,
        end_px,
        end_py,
    )
}

/// T-649 SEL-ALL-001 — every slot + placed vehicle **currently on screen**, for Ctrl/Cmd+A.
///
/// Eden scopes Select All to the VIEWPORT, not to the whole mission, so this is a viewport-rect
/// query and NOT a "hand back `soa.ids`" shortcut — an entity parked off-screen is not selected.
/// The rect is the whole canvas, so it is the marquee gesture with its two corners pinned to the
/// container instead of to a pointer: unproject the top-left CSS pixel `(0, 0)` against `cam` for
/// the start corner, then hand [`marquee_ids_with_vehicles`] the bottom-right corner in **pixels**
/// (that is the shape it already takes — press corner in world, release corner in px). So Ctrl+A
/// and a marquee dragged corner-to-corner over the whole canvas return the same set by
/// construction: one primitive, one `pick_rect`, no second definition of "inside the box".
///
/// The viewport size comes off the camera itself ([`OrthoCamera::size_px`] — post-`Math.round`,
/// deck's `|| 1` coercion applied), so the caller cannot pass a rect that disagrees with the
/// projection the same camera would produce. A singular pixel matrix (NaN unproject) yields the
/// empty selection, exactly like the marquee.
#[must_use]
pub fn view_ids_with_vehicles(
    cam: &OrthoCamera,
    soa: &SlotSoa,
    vehicle_points: &[(String, f64, f64)],
) -> Vec<String> {
    let [w, h] = cam.size_px();
    let tl = cam.unproject_xy(0.0, 0.0);
    if !tl[0].is_finite() || !tl[1].is_finite() {
        return Vec::new();
    }
    marquee_ids_with_vehicles(cam, soa, vehicle_points, tl[0], tl[1], w, h)
}
