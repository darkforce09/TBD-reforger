//! Role: freeze a camera, and resolve a screen point to the entity under it.
//! Position: `selection` in `map_editing_tools`.
//! Signals & state: a frozen camera snapshot and the app-side selected-id set; never the document.
//! Invariants: a square box query decides slots and a circular one decides vehicles, because a slot glyph is a square and a vehicle glyph is a disc. Ties are the document's to break.

use camera_math::ortho::state::OrthoCamera;
use mission_crdt::soa::SlotSoa;
use spatial_indexes::point_indexes::point_index::PointIndex;

use super::gesture::{TERRAIN_H, TERRAIN_W};

/// Build a frozen orthographic camera from the engine's live view and the container CSS size (the
/// "frozen viewport"): copied once at pointer-down so the whole gesture unprojects against a
/// stable camera, with the target clamped to the Everon bounds.
#[must_use]
pub fn frozen_camera(
    width_px: f64,
    height_px: f64,
    target_x: f64,
    target_y: f64,
    zoom: f64,
) -> OrthoCamera {
    let mut cam = OrthoCamera::new(width_px, height_px, target_x, target_y, zoom);
    cam.set_bounds(0.0, 0.0, TERRAIN_W, TERRAIN_H);
    cam
}

/// Argmin `dx²+dy²` over the handles a `PointIndex` returns for the ±`r` world box around `(qx,qy)`.
/// This is the **box-nearest** primitive (a square box and a min-distance loop) — NOT
/// `PointIndex::pick_nearest`, whose cutoff is a *circle*. Shared with the self-check so both prove
/// the exact same query.
pub(super) fn box_nearest(
    idx: &PointIndex,
    soa: &SlotSoa,
    qx: f64,
    qy: f64,
    r: f64,
) -> Option<u32> {
    let mut best: Option<(f64, u32)> = None;
    for h in idx.pick_rect(qx - r, qy - r, qx + r, qy + r) {
        let dx = f64::from(soa.xs[h as usize]) - qx;
        let dy = f64::from(soa.ys[h as usize]) - qy;
        let d2 = dx * dx + dy * dy;
        if best.is_none_or(|(bd, _)| d2 < bd) {
            best = Some((d2, h));
        }
    }
    best.map(|(_, h)| h)
}

/// Squared distance from slot `h` to the world point `(qx,qy)` (bit-exact f64).
pub(super) fn d2_to(soa: &SlotSoa, h: u32, qx: f64, qy: f64) -> f64 {
    let dx = f64::from(soa.xs[h as usize]) - qx;
    let dy = f64::from(soa.ys[h as usize]) - qy;
    dx * dx + dy * dy
}

/// Nearest slot id under a screen pixel, or `None`. Unprojects `(px,py)` against the frozen `cam`,
/// then box-nearest over the doc SoA (see `box_nearest`); returns `soa.ids[handle]`.
///
/// The resolution lives on [`mission_editing_session::picking::pick_slot`], which leaves the tie
/// policy to [`mission_document::MissionDocCore::pick_slot`].
#[must_use]
pub fn pick(cam: &OrthoCamera, soa: &SlotSoa, px: f64, py: f64) -> Option<String> {
    mission_editing_session::picking::pick_slot(cam, soa, px, py)
}

/// The slot or placed vehicle under a screen pixel; when both are in range, the closer world
/// distance wins ([`mission_editing_session::picking::pick_slot_or_vehicle`]).
#[must_use]
pub fn pick_slot_or_vehicle(
    cam: &OrthoCamera,
    soa: &SlotSoa,
    vehicle_points: &[(String, f64, f64)],
    px: f64,
    py: f64,
) -> Option<String> {
    mission_editing_session::picking::pick_slot_or_vehicle(cam, soa, vehicle_points, px, py)
}

/// Apply a sub-threshold click to the selection set:
///   * hit + additive (Ctrl/Cmd) → **toggle** (remove if present, else add; empties to none)
///   * hit + plain               → **replace** with `[id]`
///   * empty + plain             → **clear**
///   * empty + additive          → **preserve** (no-op)
pub fn apply_click(cur: &mut Vec<String>, hit: Option<String>, additive: bool) {
    match (hit, additive) {
        (Some(id), true) => {
            if let Some(pos) = cur.iter().position(|x| *x == id) {
                cur.remove(pos);
            } else {
                cur.push(id);
            }
        }
        (Some(id), false) => {
            cur.clear();
            cur.push(id);
        }
        (None, false) => cur.clear(),
        (None, true) => {}
    }
}

// ── Over-threshold gesture math (pure) ───────────────────────────────────────────────────────────

/// Which slots a drag-move commits over: dragging an
/// **already-selected** slot moves the whole selection; dragging an **unselected** slot moves just
/// it (and the caller replaces the selection with `[hit]`).
#[must_use]
pub fn compute_move_ids(hit: &str, selection: &[String]) -> Vec<String> {
    if selection.iter().any(|s| s == hit) {
        selection.to_vec()
    } else {
        vec![hit.to_string()]
    }
}

/// World-meter delta from the frozen-cam unproject of the press corner `(start_wx, start_wy)` to the
/// live pixel `(px, py)` — the drag-move offset, `unproject(px) − start`. A singular pixel matrix
/// (NaN unproject) yields `(0.0, 0.0)` (no move).
#[must_use]
pub fn drag_delta(cam: &OrthoCamera, start_wx: f64, start_wy: f64, px: f64, py: f64) -> (f64, f64) {
    let c = cam.unproject_xy(px, py);
    if !c[0].is_finite() || !c[1].is_finite() {
        return (0.0, 0.0);
    }
    (c[0] - start_wx, c[1] - start_wy)
}
