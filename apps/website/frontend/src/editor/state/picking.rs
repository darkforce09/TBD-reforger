//! Role: join spatial queries to document identifiers.
//! Position: frontend editor input adapter.
//! Signals & state: explicit frozen camera and document projections.
//! Invariants: square slot hits, circular vehicle hits, slot-first ties and marquee order.

use map_engine_core::camera::OrthoCamera;
use map_engine_core::doc::{MissionDocCore, SlotSoa};
use map_engine_core::spatial::picking as spatial;
use map_engine_core::squad_links::SquadLinkInput;

fn world_query(cam: &OrthoCamera, px: f64, py: f64) -> ([f64; 2], f64) {
    let c = cam.unproject_xy(px, py);
    let e = cam.unproject_xy(px + MissionDocCore::PICK_RADIUS_PX, py);
    ([c[0], c[1]], (e[0] - c[0]).abs())
}

/// Resolve a square spatial slot query against the document's row identifiers.
pub fn pick_slot(cam: &OrthoCamera, soa: &SlotSoa, px: f64, py: f64) -> Option<String> {
    let ([qx, qy], r) = world_query(cam, px, py);
    let row = spatial::pick_slot_row(&soa.xs, &soa.ys, qx, qy, r, MissionDocCore::GRID_CELL_M);
    MissionDocCore::pick_slot(soa, row)
}

/// Resolve a circular spatial vehicle query against the document's row identifiers.
#[allow(dead_code)]
pub fn pick_vehicle(
    cam: &OrthoCamera,
    points: &[(String, f64, f64)],
    px: f64,
    py: f64,
) -> Option<String> {
    let ([qx, qy], r) = world_query(cam, px, py);
    let row = spatial::pick_point_row(points.iter().map(|(_, x, y)| (*x, *y)), qx, qy, r);
    MissionDocCore::pick_vehicle(points, row)
}

/// Ask the document to resolve slot and vehicle hits using its tie policy.
pub fn pick_slot_or_vehicle(
    cam: &OrthoCamera,
    soa: &SlotSoa,
    points: &[(String, f64, f64)],
    px: f64,
    py: f64,
) -> Option<String> {
    let ([qx, qy], r) = world_query(cam, px, py);
    let slot = spatial::pick_slot_row(&soa.xs, &soa.ys, qx, qy, r, MissionDocCore::GRID_CELL_M);
    let vehicle = spatial::pick_point_row(points.iter().map(|(_, x, y)| (*x, *y)), qx, qy, r);
    MissionDocCore::pick_slot_or_vehicle(soa, points, qx, qy, slot, vehicle)
}

/// Map ordered spatial slot rows to authored IDs.
pub fn marquee_slot_ids(
    cam: &OrthoCamera,
    soa: &SlotSoa,
    start_wx: f64,
    start_wy: f64,
    end_px: f64,
    end_py: f64,
) -> Vec<String> {
    let e = cam.unproject_xy(end_px, end_py);
    let rows = spatial::marquee_slot_rows(
        &soa.xs,
        &soa.ys,
        [start_wx, start_wy],
        [e[0], e[1]],
        MissionDocCore::GRID_CELL_M,
    );
    MissionDocCore::marquee_slot_ids(soa, &rows)
}

/// Map ordered spatial vehicle rows to authored IDs.
#[allow(dead_code)]
pub fn marquee_vehicle_ids(
    cam: &OrthoCamera,
    points: &[(String, f64, f64)],
    start_wx: f64,
    start_wy: f64,
    end_px: f64,
    end_py: f64,
) -> Vec<String> {
    let e = cam.unproject_xy(end_px, end_py);
    let rows = spatial::marquee_point_rows(
        points.iter().map(|(_, x, y)| (*x, *y)),
        [start_wx, start_wy],
        [e[0], e[1]],
    );
    MissionDocCore::marquee_vehicle_ids(points, &rows)
}

/// Append vehicle selection after slots while retaining each query's order.
pub fn marquee_ids_with_vehicles(
    cam: &OrthoCamera,
    soa: &SlotSoa,
    points: &[(String, f64, f64)],
    start_wx: f64,
    start_wy: f64,
    end_px: f64,
    end_py: f64,
) -> Vec<String> {
    let e = cam.unproject_xy(end_px, end_py);
    let start = [start_wx, start_wy];
    let end = [e[0], e[1]];
    let slots =
        spatial::marquee_slot_rows(&soa.xs, &soa.ys, start, end, MissionDocCore::GRID_CELL_M);
    let vehicles = spatial::marquee_point_rows(points.iter().map(|(_, x, y)| (*x, *y)), start, end);
    MissionDocCore::marquee_ids_with_vehicles(soa, points, &slots, &vehicles)
}

/// Translate authored squad membership into the graphics engine's line inputs.
pub fn squad_link_inputs(doc: &MissionDocCore) -> Vec<SquadLinkInput> {
    doc.squad_link_inputs()
        .into_iter()
        .map(|s| SquadLinkInput {
            leader_slot_id: s.leader_slot_id,
            member_slot_ids: s.member_slot_ids,
            side: s.side,
        })
        .collect()
}

#[cfg(test)]
#[path = "picking/tests/selection.rs"]
mod tests;
