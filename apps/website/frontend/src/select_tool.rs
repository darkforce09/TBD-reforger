//! T-159.18 — Select / LMB tools (pick foundation) for the Leptos Mission Creator editor.
//!
//! Adds LMB **click-select** on the seeded slots, matching the React `useSelectTool` pending-left
//! model:
//!   * a pointer-down snapshots a **frozen** ortho camera (X-05 — the live `RenderEngine::unproject_xy`
//!     is deleted; a live unproject would feedback-loop as pan/zoom mutate mid-gesture), and
//!   * a sub-threshold (< 4 px) release is a **click** that picks the nearest slot via the Rust
//!     `PointIndex` over the doc SoA, then updates the selection.
//!
//! All pick math is plain Rust reusing `map-engine-core` (`camera` + `spatial`) — no `map-engine-wasm`
//! shim, one wasm module (D5). Selection is **app-side** state (a leaked `Rc<RefCell<Vec<String>>>`,
//! NOT the Y.Doc — selection never lived in the document, matching React's Zustand). It is held in the
//! editor's leaked-handle idiom (engine/doc/pan_px are all leaked `Rc`s), so the read-only
//! `window.__editorSelection` smoke bridge — a peer of `__missionDoc`/`__missionPersist` — never reads
//! reactive-owner state that a route change could dispose.
//!
//! Deferred (kept out this slice): entity drag-move commit, marquee rect, cluster drill, Attributes.

use std::cell::RefCell;
use std::rc::Rc;

use map_engine_core::camera::OrthoCamera;
use map_engine_core::doc::SlotSoa;
use map_engine_core::spatial::point_index::PointIndex;
use map_engine_render::RenderEngine;
use wasm_bindgen::prelude::*;

use crate::mission_doc::DocHandle;

/// Motion (CSS px) separating a click from a drag — the React `useSelectTool` `DRAG_THRESHOLD`.
pub const DRAG_THRESHOLD_PX: f64 = 4.0;

/// T-723 — a `LeftGesture::Pending` must not promote into Move/Marquee unless at least one
/// button is still held (`PointerEvent.buttons != 0`). A button-less promote is the phantom-drag
/// class: a Pending stranded by the armed-place pointerup (wave-106 MAJOR-2) survives disarm,
/// the next bare `pointermove` past [`DRAG_THRESHOLD_PX`] turns it into Move, and the next
/// any-button `pointerup` commits a teleport. The mission_editor pointermove path calls this
/// before capture/promote; event-SEQUENCE coverage lives in `mission_editor::armed_place` /
/// `t723_armed_place` (this module is wasm-only and invisible to native `cargo test`).
#[inline]
pub fn may_promote_pending(buttons: u16) -> bool {
    buttons != 0
}
/// `PointIndex` grid cell (world m) — SoT on [`map_engine_core::doc::MissionDocCore::GRID_CELL_M`].
const GRID_CELL_M: f64 = map_engine_core::doc::MissionDocCore::GRID_CELL_M;
/// Everon bounds (matches `mission_editor.rs`/`mission_doc.rs`), for the frozen-camera target clamp.
const TERRAIN_W: f64 = 12_800.0;
const TERRAIN_H: f64 = 12_800.0;

/// The app-side selected-slot set (NOT in the Y.Doc). Leaked like the editor's other handles so the
/// leaked bridge closures never touch disposed reactive state — see the module docs.
pub type SelectionHandle = Rc<RefCell<Vec<String>>>;

/// A leaked `Option<RenderEngine>` handle, exactly the one `mission_editor.rs` owns. `pub` since
/// T-159.21 so `mission_history` names the same alias instead of redeclaring a twin.
pub type EngineHandle = Rc<RefCell<Option<RenderEngine>>>;

/// The pending LMB gesture: the press point (CSS px, container-local) + a **frozen** ortho camera
/// copied at pointer-down. A sub-threshold release unprojects against `cam` (never the live engine).
#[derive(Clone)]
pub struct PendingLeft {
    pub start_x: f64,
    pub start_y: f64,
    pub cam: OrthoCamera,
}

/// T-159.19 — the in-flight LMB gesture, mirroring the React `useSelectTool` union
/// (`pending-left` → `move` | `marquee`). A `pointerdown` opens `Pending`; the first `pointermove`
/// past [`DRAG_THRESHOLD_PX`] promotes it to `Move` (a pick hit under the press) or `Marquee` (an
/// empty press) **only when [`may_promote_pending`] is true** (T-723 — buttons still held); a
/// `pointerup` commits on button 0. While a place is armed the host must not open this gesture at
/// all, and the armed pointerup must `take()` any stranded value (Pending/Ruler) — see
/// `mission_editor::armed_place`. Every world unproject in the gesture uses the **frozen**
/// `cam` copied at the press (M2/X-05 — the live `RenderEngine::unproject_xy` is deleted; a live
/// one would feedback-loop as pan/zoom mutate mid-gesture). `Move.dx/dy` is the last coalesced
/// world delta (fed to `engine.set_drag` for the GPU preview + `move_entities` on release).
///
/// T-642 — the RULER is the THIRD mode a left gesture can be in, and the ticket's core "how does a
/// new mode enter `LeftGesture`" answer. When the Ruler tool is active (`ruler_tool::EditorTool`),
/// an LMB `pointerdown` opens [`LeftGesture::Ruler`] INSTEAD of [`LeftGesture::Pending`] — a
/// separate arm that the pointermove/up branches match by name, so the ruler NEVER promotes to a
/// pick/marquee/move and never reaches those commits. Critically it also does NOT route through the
/// armed-placement pointerup branch (the T-723 defect zone): that branch is gated on
/// `editor_ops::has_pending()` (a palette place), which a ruler click never sets, so the ruler
/// pointerup falls straight through to its own `LG::Ruler` arm. `Ruler.cam` is the frozen press
/// camera so the rubber-band preview (in the overlay) and the eventual commit unproject alike.
pub enum LeftGesture {
    Pending(PendingLeft),
    Move {
        ids: Vec<String>,
        start_wx: f64,
        start_wy: f64,
        cam: OrthoCamera,
        dx: f64,
        dy: f64,
    },
    Marquee {
        start_x: f64,
        start_y: f64,
        start_wx: f64,
        start_wy: f64,
        cam: OrthoCamera,
    },
    /// T-642 — an in-flight ruler press: the frozen press camera + press pixel. A sub-threshold
    /// release commits ONE ruler vertex (unprojected against `cam`); the tool stays armed for the
    /// next click. Carries no pick/move/marquee payload — a ruler gesture measures, it never edits
    /// the document, so it deliberately shares nothing with the three commit arms above.
    Ruler {
        start_x: f64,
        start_y: f64,
        cam: OrthoCamera,
    },
    /// T-648 XFORM-SHIFT-001 — an in-flight **Shift-rotate**: a Shift+LMB press that landed on a
    /// selected entity. The whole live selection rotates to FACE the cursor (each entity about its
    /// own position); the release px is unprojected against the frozen `cam` to the aim point that
    /// [`crate::editor_ops::rotate_selection_to_face`] rotates toward, quantised to the active
    /// rotation ladder rung. It is a SEPARATE arm from [`LeftGesture::Move`] on purpose: a rotate
    /// commits rotation (through the existing `attrs_update_position` / `set_vehicle_position` field
    /// writes), never the atomic `move_entities_and_vehicles` translate — so the `mission_editor`
    /// move-commit pin (which requires exactly one `LG::Move` arm calling that API) is unaffected,
    /// and Shift+drag can never be mistaken for a positional move. Carries no `ids`: the commit reads
    /// the live selection at release, so a selection edited mid-gesture cannot desync a stale copy.
    Rotate {
        start_x: f64,
        start_y: f64,
        cam: OrthoCamera,
    },
}

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
fn box_nearest(idx: &PointIndex, soa: &SlotSoa, qx: f64, qy: f64, r: f64) -> Option<u32> {
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
fn d2_to(soa: &SlotSoa, h: u32, qx: f64, qy: f64) -> f64 {
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
    map_engine_core::doc::MissionDocCore::pick_slot(cam, soa, px, py)
}

/// T-425 — nearest placed vehicle id under a screen pixel, or `None`.
///
/// Vehicles are off the slot SoA (they ride their own lane — see [`bind_vehicle_preview_lane`]), so
/// the slot [`pick`] path never sees them. `points` is `(id, world_x, world_y)` from [`crate::editor_ops::vehicle_points`].
/// Delegates to [`map_engine_core::doc::MissionDocCore::pick_vehicle`] (Class-R SoT).
#[must_use]
#[allow(dead_code)] // public host helper; live path uses pick_slot_or_vehicle
pub fn pick_vehicle(
    cam: &OrthoCamera,
    points: &[(String, f64, f64)],
    px: f64,
    py: f64,
) -> Option<String> {
    map_engine_core::doc::MissionDocCore::pick_vehicle(cam, points, px, py)
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
    map_engine_core::doc::MissionDocCore::pick_slot_or_vehicle(cam, soa, vehicle_points, px, py)
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

/// T-573 — push the live drag preview for a (possibly **mixed**) selection: slot overlay lane +
/// mission-vehicle lane, from the one world delta the gesture is carrying.
///
/// The bug this cures: the caller used to strip vehicle ids out of the selection before
/// `set_drag`, and nothing then previewed the vehicles — so dragging a slot **and** a vehicle drew
/// the slot moving and the vehicle standing, while the pointerup commit
/// ([`map_engine_core::doc::MissionDocCore::move_entities_and_vehicles`], T-491/T-574) moved both.
/// The overlay described a drop it would not perform.
///
/// Both lanes are driven from the same `ids`, so they cannot disagree:
///
/// * **Slots** — `set_drag` resolves ids against the slot SoA and *skips* what it cannot find
///   ([`map_engine_core::slots_gpu::pack_drag_overlay`]), so the vehicle ids in a mixed selection
///   cost one hash miss each and the old pre-filter was never load-bearing. Handing over the whole
///   list keeps the engine's `drag_ids` equal to the gesture's, so its Start/Restart/Delta phase
///   classification tracks the real selection instead of a filtered shadow of it.
/// * **Vehicles** — the `MissionVehicles` lane is a dense pack the engine re-uploads wholesale and
///   holds no ids for, so the preview is a re-pack of the *whole* lane with the dragged rows
///   offset ([`map_engine_core::slots_gpu::pack_vehicle_drag_preview`]). No engine change, and no
///   vehicle ids inside `map-engine-render`.
///
/// `vehicle_points` is [`crate::editor_ops::vehicle_points`] — the same list the press-time pick
/// ran against, so every draggable vehicle is by construction a row in it.
pub fn push_drag_preview(
    e: &mut RenderEngine,
    ids: &[String],
    vehicle_points: &[(String, f64, f64)],
    dx: f64,
    dy: f64,
) {
    #[allow(clippy::cast_possible_truncation)]
    e.set_drag(ids.to_vec(), dx as f32, dy as f32);
    bind_vehicle_preview_lane(
        e,
        &map_engine_core::slots_gpu::pack_vehicle_drag_preview(ids, vehicle_points, dx, dy),
    );
}

/// T-573 — drop the live drag preview and put **both** lanes back on the authored positions.
///
/// The vehicle half is not bookkeeping: [`push_drag_preview`] moves real rows in the vehicle lane,
/// so a gesture that ends without a commit (pointercancel; a release whose delta is zero) must
/// re-bind it or the discs stay parked at the last previewed offset while the document says
/// otherwise — the same lie as the original bug, just frozen. The restore is the identity re-pack
/// (`pack_vehicle_drag_preview` with an empty drag set), i.e. the lane
/// `mission_history::after_doc_change` re-binds from the document after a *committed* drag.
pub fn clear_drag_preview(e: &mut RenderEngine, vehicle_points: &[(String, f64, f64)]) {
    e.set_drag(Vec::new(), 0.0, 0.0);
    bind_vehicle_preview_lane(
        e,
        &map_engine_core::slots_gpu::pack_vehicle_drag_preview(&[], vehicle_points, 0.0, 0.0),
    );
}

/// **T-808 — bind the vehicle lane at PREVIEW positions without losing its symbology.**
///
/// The defect: [`push_drag_preview`] and [`clear_drag_preview`] both called the old
/// `vehicles_bind`, whose lane is one amber disc per vehicle. So the instant a drag started, every
/// vehicle on the map — dragged or not — dropped its silhouette, its side colour and its heading,
/// and popped back to symbology only when the pointerup commit ran `after_doc_change`. The preview
/// described a map the drop would not produce, which is the same class of lie T-573 cured for
/// position.
///
/// `xy` is the PREVIEWED lane (dragged rows already offset by
/// [`map_engine_core::slots_gpu::pack_vehicle_drag_preview`]); the other three columns are the
/// document's, because a drag moves vehicles and changes nothing else about them.
///
/// **THE COLUMN-ALIGNMENT TRAP.** The four columns must describe the same rows in the same order or
/// every vehicle wears another's kind, side and heading — and a silhouette pointing confidently the
/// wrong way is believed, which makes it worse than the disc it replaces.
/// [`crate::mission_history::vehicle_lane_fields`] is the SINGLE column builder (one pass over the
/// id-sorted `editor_ops::vehicle_rows`); this reuses it rather than growing a second one, so the
/// preview is built by the same code as the committed render and cannot drift from it. The `xy`
/// handed in comes from `editor_ops::vehicle_points`, which is that same `vehicle_rows` reader
/// filtered to placed rows — the same rows in the same order. The yrs-iteration-order
/// `vehicle_xy_flat` must never appear on this path: mixing the two orders is the trap.
///
/// The length gate is the one thing that is not structural. Both snapshots are read from the live
/// document, and a drag commits nothing until pointerup, so they agree; if they ever did not, a row
/// was added or removed between the two reads and every column after it would be shifted by one.
/// Rather than zip a shift, fall back to the plain disc lane — less information, but never a
/// confident lie.
fn bind_vehicle_preview_lane(e: &mut RenderEngine, xy: &[f32]) {
    let (doc_xy, aliases, tints, headings) = crate::mission_history::vehicle_lane_fields();
    if doc_xy.len() == xy.len() {
        e.vehicles_bind_symbology(xy, aliases, &tints, &headings);
    } else {
        e.vehicles_bind(xy);
    }
}

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
    map_engine_core::doc::MissionDocCore::marquee_slot_ids(
        cam, soa, start_wx, start_wy, end_px, end_py,
    )
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
    map_engine_core::doc::MissionDocCore::marquee_vehicle_ids(
        cam, points, start_wx, start_wy, end_px, end_py,
    )
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
    map_engine_core::doc::MissionDocCore::marquee_ids_with_vehicles(
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

/// Class-S self-check for the marquee (S3 parity, peer of [`pick_selfcheck`]): `PointIndex::pick_rect`
/// must return the SAME id SET as a brute-force box scan over the same seeded SoA, for a battery of
/// world boxes (each seed ± a spread of half-extents). Set-equality (sorted handle compare), so
/// grid vs row order is not a false negative. Runs in-browser over the real seeded SoA.
#[must_use]
pub fn marquee_selfcheck(soa: &SlotSoa) -> bool {
    let n = soa.ids.len();
    if n == 0 {
        return true;
    }
    let idx = PointIndex::build(soa.xs.clone(), soa.ys.clone(), GRID_CELL_M);
    let halfs = [0.0_f64, 5.0, 64.0, 512.0];
    for i in 0..n {
        let (sx, sy) = (f64::from(soa.xs[i]), f64::from(soa.ys[i]));
        for &h in &halfs {
            let (min_x, min_y, max_x, max_y) = (sx - h, sy - h, sx + h, sy + h);
            let mut via_index = idx.pick_rect(min_x, min_y, max_x, max_y);
            let mut via_brute: Vec<u32> = (0..n as u32)
                .filter(|&j| {
                    let (x, y) = (f64::from(soa.xs[j as usize]), f64::from(soa.ys[j as usize]));
                    x >= min_x && x <= max_x && y >= min_y && y <= max_y
                })
                .collect();
            via_index.sort_unstable();
            via_brute.sort_unstable();
            if via_index != via_brute {
                return false;
            }
        }
    }
    true
}

/// Class-S self-check (S3): the `PointIndex` box-nearest used by [`pick`] must agree with a
/// brute-force box scan over the SAME points, for every seed and a spread of ± offsets as the query.
/// Compared by resulting **nearest distance** (bit-exact f64), so an exactly-equidistant tie — where
/// grid-order and row-order could pick different handles — is not a false negative; for the
/// non-degenerate random seeds the handles coincide anyway. Runs in-browser over the real seeded SoA.
#[must_use]
pub fn pick_selfcheck(soa: &SlotSoa) -> bool {
    let n = soa.ids.len();
    if n == 0 {
        return true;
    }
    let idx = PointIndex::build(soa.xs.clone(), soa.ys.clone(), GRID_CELL_M);
    let offsets = [0.0_f64, 3.0, -3.0, 40.0, -40.0];
    let r = 64.0_f64; // world box half-size for the parity probe
    for i in 0..n {
        let (sx, sy) = (f64::from(soa.xs[i]), f64::from(soa.ys[i]));
        for &ox in &offsets {
            for &oy in &offsets {
                let (qx, qy) = (sx + ox, sy + oy);
                let via_index = box_nearest(&idx, soa, qx, qy, r);
                let via_brute = box_nearest_brute(soa, qx, qy, r);
                let ok = match (via_index, via_brute) {
                    (None, None) => true,
                    (Some(a), Some(b)) => d2_to(soa, a, qx, qy) == d2_to(soa, b, qx, qy),
                    _ => false,
                };
                if !ok {
                    return false;
                }
            }
        }
    }
    true
}

/// Brute-force box-nearest oracle: a linear scan over every slot within the ±`r` box. The Class-S
/// reference for [`box_nearest`].
fn box_nearest_brute(soa: &SlotSoa, qx: f64, qy: f64, r: f64) -> Option<u32> {
    let mut best: Option<(f64, u32)> = None;
    for i in 0..soa.ids.len() {
        let (x, y) = (f64::from(soa.xs[i]), f64::from(soa.ys[i]));
        if x >= qx - r && x <= qx + r && y >= qy - r && y <= qy + r {
            let d2 = d2_to(soa, i as u32, qx, qy);
            if best.is_none_or(|(bd, _)| d2 < bd) {
                best = Some((d2, i as u32));
            }
        }
    }
    best.map(|(_, i)| i)
}

// ── smoke bridge ────────────────────────────────────────────────────────────────────────────────

/// Append `raw` as a JSON string body (quote/backslash escaped) into `s` (no surrounding quotes).
fn push_json_escaped(s: &mut String, raw: &str) {
    for ch in raw.chars() {
        match ch {
            '"' => s.push_str("\\\""),
            '\\' => s.push_str("\\\\"),
            c => s.push(c),
        }
    }
}

/// Serialize a slice of ids as a JSON array string, e.g. `["a","b"]`.
fn json_id_array(ids: &[String]) -> String {
    let mut s = String::from("[");
    for (i, id) in ids.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        s.push('"');
        push_json_escaped(&mut s, id);
        s.push('"');
    }
    s.push(']');
    s
}

/// A container-local screen px that is farthest from every projected slot — a **guaranteed-empty**
/// click target for the smoke's clear/deselect assertion (max over a candidate grid of the min
/// distance to any slot px). With a handful of slots this is comfortably clear of every glyph.
///
/// T-159.21 — the candidate grid is inset to the **chrome-free** region. The Eden chrome overlays
/// this same container and stops `pointerdown` from reaching the map handlers, so a px under the
/// top strip or a dock would be un-clickable and the deselect gate would hang on a stale selection.
///
/// T-637 — the insets come from `eden_layout`'s accessors, and that file also owns the Tailwind
/// MOUNT CLASSES `mission_editor` renders the docks with (`DOCK_LEFT_MOUNT` etc.). That is not
/// tidiness: the numbers this function insets by and the classes the browser lays the panels out
/// from are ONE contract, and if they drift the probe grid — and every real pointer unprojection
/// alongside it — is offset by the difference while everything still looks correct.
/// `eden_layout::t637_dock_geometry` parses the width back out of the mount class and checks it
/// against the accessor this function calls.
///
/// This **shrinks the search space; it does not weaken the property.** The result is still the
/// argmax over candidates of the min distance to any projected slot, i.e. still empty — and the
/// gate needs *an* empty px, not a specific one. Sufficiency is structural rather than incidental:
/// `pick` hits within `MissionDocCore::PICK_RADIUS_PX` (4), while grid candidates are tens of px apart, so one slot
/// can shadow at most one candidate; a handful of slots can never shadow all of them. Slots that
/// project outside the region still count in the min-distance and only push the winner further out.
fn farthest_empty_px(w: f64, h: f64, proj: &[(f64, f64)]) -> (f64, f64) {
    let (nx, ny) = (21usize, 13usize);
    // Degenerate viewport (chrome ≥ container) → fall back to the whole rect rather than emit a
    // NaN/inverted box.
    // T-638 — the LIVE insets (dock collapse + chrome_hidden folded in), not the expanded consts:
    // a collapsed dock frees its strip to the map, so a "guaranteed-empty" probe px may now sit where
    // the panel used to be. `crate::eden_layout::*` owns the accessors (`eden_chrome` re-exports the
    // consts by name for the non-owned readers; the dynamic seam is the accessor).
    let (mut x0, mut x1) = (
        crate::eden_layout::dock_left_px(),
        w - crate::eden_layout::dock_right_px(),
    );
    let (mut y0, mut y1) = (
        crate::eden_layout::strip_top_px(),
        h - crate::eden_layout::toolbelt_band_px(),
    );
    if x1 - x0 < 1.0 || y1 - y0 < 1.0 {
        x0 = 0.0;
        x1 = w;
        y0 = 0.0;
        y1 = h;
    }
    let (rw, rh) = (x1 - x0, y1 - y0);
    let mut best = (x0 + rw * 0.5, y0 + rh * 0.5);
    let mut best_d = -1.0_f64;
    for iy in 0..ny {
        for ix in 0..nx {
            let cx = x0 + (ix as f64 + 0.5) / nx as f64 * rw;
            let cy = y0 + (iy as f64 + 0.5) / ny as f64 * rh;
            let mut mind = f64::INFINITY;
            for &(px, py) in proj {
                let d = ((px - cx).powi(2) + (py - cy).powi(2)).sqrt();
                mind = mind.min(d);
            }
            if mind > best_d {
                best_d = mind;
                best = (cx, cy);
            }
        }
    }
    best
}

/// Compute the `probe()` payload: centre seed 0 in the engine view (a **test hook** — `set_view`,
/// zoom preserved), then return JSON `{"id","hit":[px,py],"empty":[px,py]}` where `hit` projects the
/// centred seed to screen (≈ container centre) and `empty` is a guaranteed slot-free px. This makes
/// the click smoke deterministic and independent of where the fixed seed happens to land.
fn probe_json(
    doc: &DocHandle,
    engine: &EngineHandle,
    container: &web_sys::HtmlDivElement,
) -> String {
    let null = || String::from(r#"{"id":null,"hit":null,"empty":null}"#);
    let soa = match doc.borrow().as_ref().map(|c| c.materialize()) {
        Some(s) if !s.ids.is_empty() => s,
        _ => return null(),
    };
    let (sx, sy) = (f64::from(soa.xs[0]), f64::from(soa.ys[0]));

    // Centre seed 0 and read the (possibly clamped) resulting view so `project` is exact.
    let (tx, ty, z) = {
        let mut guard = engine.borrow_mut();
        let Some(e) = guard.as_mut() else {
            return null();
        };
        e.set_view(sx, sy, e.zoom());
        (e.target_x(), e.target_y(), e.zoom())
    };

    let rect = container.get_bounding_client_rect();
    let (w, h) = (rect.width(), rect.height());
    let cam = frozen_camera(w, h, tx, ty, z);

    let hit = cam.project([sx, sy, 0.0]);
    let proj: Vec<(f64, f64)> = (0..soa.ids.len())
        .map(|i| {
            let p = cam.project([f64::from(soa.xs[i]), f64::from(soa.ys[i]), 0.0]);
            (p[0], p[1])
        })
        .collect();
    let (ex, ey) = farthest_empty_px(w, h, &proj);

    let mut s = String::from(r#"{"id":""#);
    push_json_escaped(&mut s, &soa.ids[0]);
    s.push_str(&format!(
        r#"","hit":[{},{}],"empty":[{},{}]}}"#,
        hit[0], hit[1], ex, ey
    ));
    s
}

/// Compute the `probe_move()` payload (T-159.19): centre seed 0 in the engine view (a **test hook**
/// — `set_view`, zoom preserved), read back the (possibly clamped) view so `project` is exact, then
/// return JSON `{"id","from":[px,py],"to":[px,py]}` where `from` projects the centred seed to screen
/// and `to = from + (40, 0)` (well past [`DRAG_THRESHOLD_PX`]). The smoke drags `from`→`to` and
/// asserts the slot-position digest changed + the seed is selected + an edit persist fired.
fn probe_move_json(
    doc: &DocHandle,
    engine: &EngineHandle,
    container: &web_sys::HtmlDivElement,
) -> String {
    let null = || String::from(r#"{"id":null,"from":null,"to":null}"#);
    let soa = match doc.borrow().as_ref().map(|c| c.materialize()) {
        Some(s) if !s.ids.is_empty() => s,
        _ => return null(),
    };
    let (sx, sy) = (f64::from(soa.xs[0]), f64::from(soa.ys[0]));
    let (tx, ty, z) = {
        let mut guard = engine.borrow_mut();
        let Some(e) = guard.as_mut() else {
            return null();
        };
        e.set_view(sx, sy, e.zoom());
        (e.target_x(), e.target_y(), e.zoom())
    };
    let rect = container.get_bounding_client_rect();
    let cam = frozen_camera(rect.width(), rect.height(), tx, ty, z);
    let from = cam.project([sx, sy, 0.0]);
    let (fx, fy) = (from[0], from[1]);
    let (tox, toy) = (fx + 40.0, fy);

    let mut s = String::from(r#"{"id":""#);
    push_json_escaped(&mut s, &soa.ids[0]);
    s.push_str(&format!(r#"","from":[{fx},{fy}],"to":[{tox},{toy}]}}"#));
    s
}

/// Compute the `probe_marquee()` payload (T-159.19): centre seed 0 (test hook; read-back view), then
/// return JSON `{"rect":[x0,y0,x1,y1],"expect_ids":[…],"expect_count":n}` — a 60×60 px box around the
/// seed's projection. `expect_*` is computed by the SAME [`marquee_ids`] the pointer handler runs
/// (start world = `unproject(x0,y0)` at press, end px = `(x1,y1)` at release), so the smoke's CDP drag
/// over `rect` must reproduce it exactly — an end-to-end parity check on top of Class-S
/// [`marquee_selfcheck`].
fn probe_marquee_json(
    doc: &DocHandle,
    engine: &EngineHandle,
    container: &web_sys::HtmlDivElement,
) -> String {
    let null = || String::from(r#"{"rect":null,"expect_ids":null,"expect_count":0}"#);
    let soa = match doc.borrow().as_ref().map(|c| c.materialize()) {
        Some(s) if !s.ids.is_empty() => s,
        _ => return null(),
    };
    let (sx, sy) = (f64::from(soa.xs[0]), f64::from(soa.ys[0]));
    let (tx, ty, z) = {
        let mut guard = engine.borrow_mut();
        let Some(e) = guard.as_mut() else {
            return null();
        };
        e.set_view(sx, sy, e.zoom());
        (e.target_x(), e.target_y(), e.zoom())
    };
    let rect = container.get_bounding_client_rect();
    let cam = frozen_camera(rect.width(), rect.height(), tx, ty, z);
    let p = cam.project([sx, sy, 0.0]);
    let (x0, y0, x1, y1) = (p[0] - 30.0, p[1] - 30.0, p[0] + 30.0, p[1] + 30.0);
    // Oracle: the handler freezes the cam + press corner at pointerdown, so start world =
    // unproject(x0,y0); end px = the release (x1,y1). marquee_ids over exactly those.
    let start = cam.unproject_xy(x0, y0);
    let expect = marquee_ids(&cam, &soa, start[0], start[1], x1, y1);

    let mut s = String::from("{\"rect\":[");
    s.push_str(&format!("{x0},{y0},{x1},{y1}],\"expect_ids\":"));
    s.push_str(&json_id_array(&expect));
    s.push_str(&format!(",\"expect_count\":{}}}", expect.len()));
    s
}

/// Install `window.__editorSelection` — a thin, read-only smoke bridge (S5) mirroring
/// `register_mission_doc`/`register_mission_persist` (a `js_sys::Object` of `.forget()`'d closures
/// returning `JsValue`). Fields:
///   * `count()`             → current selection length (number)
///   * `ids()`               → JSON array string of selected ids
///   * `pick_selfcheck()`    → bool (Class-S PointIndex-vs-brute parity for click-pick over the seeds)
///   * `probe()`             → JSON `{id,hit,empty}` click test hook (centres a seed; see [`probe_json`])
///   * `marquee_selfcheck()` → bool (Class-S `pick_rect`-vs-brute parity for the marquee; T-159.19)
///   * `probe_marquee()`     → JSON `{rect,expect_ids,expect_count}` (see [`probe_marquee_json`])
///   * `probe_move()`        → JSON `{id,from,to}` (see [`probe_move_json`])
///
/// Read-only w.r.t. selection; the `probe*()` hooks mutate only the camera (`set_view`) for the smoke.
/// Registered synchronously on mount (like `__missionDoc`); the closures leak with the engine.
///
/// **T-778 audited this leak and DELIBERATELY LEFT IT.** The wave-129/T-778 seam-lifecycle fix
/// (`ruler_tool::install_seam`) does not apply here, and forcing it would break the harness:
///   * this is not a thread_local seam — it is `Reflect::set` onto `window` plus `Closure::forget()`,
///     which is irreversible by construction, so there is no cell to identity-guard;
///   * the defect's PRECONDITION is absent. The dead click needs a `set` onto a DISPOSED signal
///     (a silent no-op in `reactive_graph` 0.2.14) reported as success. These closures touch no
///     reactive state at all: every handle they close over is a leaked `Rc<RefCell<…>>` chosen for
///     exactly that reason (see the module docs — "never reads reactive-owner state that a route
///     change could dispose"), and they are read-only w.r.t. selection, so they have no success to
///     misreport;
///   * the lifetime is intentional and wider than any owner: the smoke harness reads
///     `window.__editorSelection` across the whole page lifetime, so unregistering at unmount would
///     delete the bridge the S5 smoke is mid-way through using.
/// The leak is scoped to the engine's own lifetime, which is the same leak `__missionDoc` /
/// `__missionPersist` take. Re-auditing this needs new evidence, not a re-reading of the same facts.
pub fn register_editor_selection(
    selection: SelectionHandle,
    doc: DocHandle,
    engine: EngineHandle,
    container: web_sys::HtmlDivElement,
) {
    let obj = js_sys::Object::new();

    let count = {
        let selection = selection.clone();
        Closure::wrap(Box::new(move || -> JsValue {
            JsValue::from_f64(selection.borrow().len() as f64)
        }) as Box<dyn FnMut() -> JsValue>)
    };
    let ids = {
        let selection = selection.clone();
        Closure::wrap(Box::new(move || -> JsValue {
            JsValue::from_str(&json_id_array(&selection.borrow()))
        }) as Box<dyn FnMut() -> JsValue>)
    };
    let selfcheck = {
        let doc = doc.clone();
        Closure::wrap(Box::new(move || -> JsValue {
            let ok = doc
                .borrow()
                .as_ref()
                .is_some_and(|c| pick_selfcheck(&c.materialize()));
            JsValue::from_bool(ok)
        }) as Box<dyn FnMut() -> JsValue>)
    };
    let probe = {
        let doc = doc.clone();
        let engine = engine.clone();
        let container = container.clone();
        Closure::wrap(Box::new(move || -> JsValue {
            JsValue::from_str(&probe_json(&doc, &engine, &container))
        }) as Box<dyn FnMut() -> JsValue>)
    };
    let marquee_selfcheck_fn = {
        let doc = doc.clone();
        Closure::wrap(Box::new(move || -> JsValue {
            let ok = doc
                .borrow()
                .as_ref()
                .is_some_and(|c| marquee_selfcheck(&c.materialize()));
            JsValue::from_bool(ok)
        }) as Box<dyn FnMut() -> JsValue>)
    };
    let probe_marquee = {
        let doc = doc.clone();
        let engine = engine.clone();
        let container = container.clone();
        Closure::wrap(Box::new(move || -> JsValue {
            JsValue::from_str(&probe_marquee_json(&doc, &engine, &container))
        }) as Box<dyn FnMut() -> JsValue>)
    };
    let probe_move = {
        let doc = doc.clone();
        let engine = engine.clone();
        let container = container.clone();
        Closure::wrap(Box::new(move || -> JsValue {
            JsValue::from_str(&probe_move_json(&doc, &engine, &container))
        }) as Box<dyn FnMut() -> JsValue>)
    };

    let _ = js_sys::Reflect::set(&obj, &JsValue::from_str("count"), count.as_ref());
    let _ = js_sys::Reflect::set(&obj, &JsValue::from_str("ids"), ids.as_ref());
    let _ = js_sys::Reflect::set(
        &obj,
        &JsValue::from_str("pick_selfcheck"),
        selfcheck.as_ref(),
    );
    let _ = js_sys::Reflect::set(&obj, &JsValue::from_str("probe"), probe.as_ref());
    let _ = js_sys::Reflect::set(
        &obj,
        &JsValue::from_str("marquee_selfcheck"),
        marquee_selfcheck_fn.as_ref(),
    );
    let _ = js_sys::Reflect::set(
        &obj,
        &JsValue::from_str("probe_marquee"),
        probe_marquee.as_ref(),
    );
    let _ = js_sys::Reflect::set(&obj, &JsValue::from_str("probe_move"), probe_move.as_ref());
    if let Some(win) = web_sys::window() {
        let _ = js_sys::Reflect::set(&win, &JsValue::from_str("__editorSelection"), &obj);
    }
    // The harness reads these across the page lifetime; leak them (the engine + its bridges leak too).
    count.forget();
    ids.forget();
    selfcheck.forget();
    probe.forget();
    marquee_selfcheck_fn.forget();
    probe_marquee.forget();
    probe_move.forget();
}

// T-636 / T-638 / T-637 — the inset-reader tests live in `eden_layout` (the consts' owner, natively
// compiled), NOT here: this whole module is `#[cfg(target_arch = "wasm32")]` (main.rs), so a native
// `cargo test` never sees it. `farthest_empty_px` above reads the band via the T-638 accessor
// `crate::eden_layout::toolbelt_band_px()` (was `eden_chrome::TOOLBELT_BAND_PX`) — that read is one of
// the two the layout accessor-conversion test pins by name, and it must not hardcode `96.0`.
//
// T-637 equalised the docks to 240/240. This file needed no change for that, and THAT IS THE POINT:
// it reads `dock_left_px()`/`dock_right_px()`, never the numbers, so a width change reaches the
// pointer path by construction. The half that could still go wrong — the mount CLASS drifting from
// the const — is what `eden_layout::t637_dock_geometry` now pins.
