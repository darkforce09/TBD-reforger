//! Role: freeze a camera, and resolve a screen point to the entity under it.
//! Position: `editing/tools/selection` in the map engine.
//! Signals & state: a frozen camera snapshot and the app-side selected-id set; never the document.
//! Invariants: a square box query decides slots and a circular one decides vehicles, because a slot glyph is a square and a vehicle glyph is a disc. Ties are the document's to break.

use crate::camera::ortho::state::OrthoCamera;
use crate::data::store::SlotSoa;
use crate::spatial::indexing::point_index::PointIndex;

use super::gesture::{TERRAIN_H, TERRAIN_W};

/// Build a frozen ortho-camera snapshot from the engine's live view + the container CSS size (S2 —
/// the "frozen viewport"): copied once at pointer-down so the whole gesture unprojects against a
/// stable camera. Mirrors the React `viewportFromViewState` adapter (`OrthoCameraJs` there;
/// `map-engine-core`'s `OrthoCamera` here — same deck-parity math, one wasm module).
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
/// This is the **box-nearest** primitive React's `slotSpatialIndex.pickNearest` uses (a square box +
/// a min-distance loop) — NOT `PointIndex::pick_nearest`, whose cutoff is a *circle*. Shared by the
/// Class-S self-check so both prove the exact same query.
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
/// then box-nearest over the doc SoA (see [`box_nearest`]); returns `soa.ids[handle]`.
///
/// T-491 — implementation lives on [`MissionDocCore::pick_slot`] (native Class-R); this wrapper
/// keeps the select_tool call sites stable.
#[must_use]
pub fn pick(cam: &OrthoCamera, soa: &SlotSoa, px: f64, py: f64) -> Option<String> {
    crate::editing::picking::pick_slot(cam, soa, px, py)
}

/// T-425 — nearest placed vehicle id under a screen pixel, or `None`.
///
/// Vehicles are off the slot SoA (they ride their own lane — see [`bind_vehicle_preview_lane`]), so
/// the slot [`pick`] path never sees them. `points` is `(id, world_x, world_y)` from [`crate::editor::state::operations::vehicle_points`].
/// Delegates to [`map_engine_core::doc::MissionDocCore::pick_vehicle`] (Class-R SoT).
#[must_use]
#[allow(dead_code)] // public host helper; live path uses pick_slot_or_vehicle
pub fn pick_vehicle(
    cam: &OrthoCamera,
    points: &[(String, f64, f64)],
    px: f64,
    py: f64,
) -> Option<String> {
    crate::editing::picking::pick_vehicle(cam, points, px, py)
}

/// T-425 — pick slot or vehicle; when both are in range, the closer world-distance wins.
#[must_use]
pub fn pick_slot_or_vehicle(
    cam: &OrthoCamera,
    soa: &SlotSoa,
    vehicle_points: &[(String, f64, f64)],
    px: f64,
    py: f64,
) -> Option<String> {
    crate::editing::picking::pick_slot_or_vehicle(cam, soa, vehicle_points, px, py)
}

/// Apply a click to the selection set, matching React `useSelectTool` onPointerUp `pending-left`:
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

// ── T-159.19: over-threshold gesture math (pure; verified in-browser via the bridge) ─────────────

/// Which slots a drag-move commits over (React `useSelectTool.ts:204`): dragging an
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
/// live pixel `(px, py)` — the drag-move offset (React `useSelectTool.ts:226` `unproject(px) −
/// startWorld`). A singular pixel matrix (NaN unproject) yields `(0.0, 0.0)` (no move).
#[must_use]
pub fn drag_delta(cam: &OrthoCamera, start_wx: f64, start_wy: f64, px: f64, py: f64) -> (f64, f64) {
    let c = cam.unproject_xy(px, py);
    if !c[0].is_finite() || !c[1].is_finite() {
        return (0.0, 0.0);
    }
    (c[0] - start_wx, c[1] - start_wy)
}
