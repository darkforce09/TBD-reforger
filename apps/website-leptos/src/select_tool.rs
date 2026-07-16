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
/// Click pick radius (CSS px) — the React `slotSpatialIndex.pickNearest` default `radiusPx` (also 4).
const PICK_RADIUS_PX: f64 = 4.0;
/// `PointIndex` grid cell (world m) — the React `slotSpatialIndex` `GRID_CELL_M`.
const GRID_CELL_M: f64 = 256.0;
/// Everon bounds (matches `mission_editor.rs`/`mission_doc.rs`), for the frozen-camera target clamp.
const TERRAIN_W: f64 = 12_800.0;
const TERRAIN_H: f64 = 12_800.0;

/// The app-side selected-slot set (NOT in the Y.Doc). Leaked like the editor's other handles so the
/// leaked bridge closures never touch disposed reactive state — see the module docs.
pub type SelectionHandle = Rc<RefCell<Vec<String>>>;

/// A leaked `Option<RenderEngine>` handle, exactly the one `mission_editor.rs` owns.
type EngineHandle = Rc<RefCell<Option<RenderEngine>>>;

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
/// empty press); a `pointerup` commits. Every world unproject in the gesture uses the **frozen**
/// `cam` copied at the press (M2/X-05 — the live `RenderEngine::unproject_xy` is deleted; a live
/// one would feedback-loop as pan/zoom mutate mid-gesture). `Move.dx/dy` is the last coalesced
/// world delta (fed to `engine.set_drag` for the GPU preview + `move_entities` on release).
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

/// World pick radius (m) for a screen radius (px) under a camera — deck/React `worldPickRadius`:
/// `|unproject(px + r, py).x − unproject(px, py).x|`. Uniform ortho scale ⇒ the x-delta is the world
/// distance a `radius_px` offset spans.
fn world_pick_radius(cam: &OrthoCamera, px: f64, py: f64, radius_px: f64) -> f64 {
    let c = cam.unproject_xy(px, py);
    let e = cam.unproject_xy(px + radius_px, py);
    (e[0] - c[0]).abs()
}

/// Argmin `dx²+dy²` over the handles a `PointIndex` returns for the ±`r` world box around `(qx,qy)`.
/// This is the **box-nearest** primitive React's `slotSpatialIndex.pickNearest` uses (a square box +
/// a min-distance loop) — NOT `PointIndex::pick_nearest`, whose cutoff is a *circle*. Shared by the
/// live click path and the Class-S self-check so both prove the exact same query.
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
#[must_use]
pub fn pick(cam: &OrthoCamera, soa: &SlotSoa, px: f64, py: f64) -> Option<String> {
    if soa.ids.is_empty() {
        return None;
    }
    let c = cam.unproject_xy(px, py);
    let (qx, qy) = (c[0], c[1]);
    if !qx.is_finite() || !qy.is_finite() {
        return None; // singular pixel matrix (deck would have warned) — no pick
    }
    let r = world_pick_radius(cam, px, py, PICK_RADIUS_PX);
    let idx = PointIndex::build(soa.xs.clone(), soa.ys.clone(), GRID_CELL_M);
    box_nearest(&idx, soa, qx, qy, r).map(|h| soa.ids[h as usize].clone())
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

/// Slot ids inside the marquee box, from the two frozen-cam screen corners. The press corner is
/// already unprojected to `(start_wx, start_wy)`; this unprojects the release px `(end_px, end_py)`,
/// forms the **ordered** world AABB (the drag can go any direction — `PointIndex::pick_rect` returns
/// empty on `max < min`), then maps the returned handles to `soa.ids`. Mirrors React
/// `slotSpatialIndex.pickRect(startWorld, endWorld)` (`useSelectTool.ts:293`). A singular pixel
/// matrix (NaN unproject on either corner) yields no selection.
#[must_use]
pub fn marquee_ids(
    cam: &OrthoCamera,
    soa: &SlotSoa,
    start_wx: f64,
    start_wy: f64,
    end_px: f64,
    end_py: f64,
) -> Vec<String> {
    if soa.ids.is_empty() {
        return Vec::new();
    }
    let e = cam.unproject_xy(end_px, end_py);
    let (ewx, ewy) = (e[0], e[1]);
    if !ewx.is_finite() || !ewy.is_finite() || !start_wx.is_finite() || !start_wy.is_finite() {
        return Vec::new();
    }
    let (min_x, max_x) = (start_wx.min(ewx), start_wx.max(ewx));
    let (min_y, max_y) = (start_wy.min(ewy), start_wy.max(ewy));
    let idx = PointIndex::build(soa.xs.clone(), soa.ys.clone(), GRID_CELL_M);
    idx.pick_rect(min_x, min_y, max_x, max_y)
        .into_iter()
        .map(|h| soa.ids[h as usize].clone())
        .collect()
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
fn farthest_empty_px(w: f64, h: f64, proj: &[(f64, f64)]) -> (f64, f64) {
    let (nx, ny) = (21usize, 13usize);
    let mut best = (w * 0.5, h * 0.5);
    let mut best_d = -1.0_f64;
    for iy in 0..ny {
        for ix in 0..nx {
            let cx = (ix as f64 + 0.5) / nx as f64 * w;
            let cy = (iy as f64 + 0.5) / ny as f64 * h;
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
fn probe_json(doc: &DocHandle, engine: &EngineHandle, container: &web_sys::HtmlDivElement) -> String {
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
