//! Role: the browser half of the selection tool — the drag preview lanes and the read-only
//! `window.__editorSelection` smoke bridge.
//! Position: `editor/tools` in the frontend editor.
//! Signals & state: the leaked selection / engine / document handles the editor owns.
//! Invariants: the gesture model, the pick, the marquee and their brute-force oracles belong to
//! `website_map_engine::editing::tools::selection`. What remains here is what needs a `window`, a
//! container element, or the host's live document: the preview lanes that keep the tether
//! hairlines and vehicle symbology attached to provisional positions, and the probe closures the
//! headless gates read.

use std::collections::HashMap;
use website_map_engine::editing::tools::selection;

use wasm_bindgen::prelude::*;
use website_map_engine::frame::engine::RenderEngine;
use website_map_engine::overlay::lanes::role_id;
use website_map_engine::overlay::symbology::links::squad_links::pack_squad_link_drag_preview;

use selection::gesture::{EngineHandle, SelectionHandle};
use selection::marquee::{marquee_ids, marquee_ids_with_vehicles, view_ids_with_vehicles};
use selection::pick::{frozen_camera, pick};
use selection::self_check::{marquee_selfcheck, pick_selfcheck};

use crate::editor::state::doc_host::DocHandle;

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
/// `vehicle_points` is [`website_map_engine::editing::hosted_commands::vehicle_points`] — the same list the press-time pick
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
        &website_map_engine::overlay::symbology::instances::drag::pack_vehicle_drag_preview(
            ids,
            vehicle_points,
            dx,
            dy,
        ),
    );
    // T-801 — tether/squad lines track the same world delta as the sprite preview. Commit still
    // rebuilds from the document on `after_doc_change`; this is preview-only.
    bind_squad_link_preview(e, ids, dx, dy);
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
        &website_map_engine::overlay::symbology::instances::drag::pack_vehicle_drag_preview(
            &[],
            vehicle_points,
            0.0,
            0.0,
        ),
    );
    // T-801 — identity re-pack puts tether endpoints back on authored xy (cancel / zero-delta).
    bind_squad_link_preview(e, &[], 0.0, 0.0);
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
/// [`crate::editor::state::history::vehicle_lane_fields`] is the SINGLE column builder (one pass over the
/// id-sorted `editor_ops::vehicle_rows`); this reuses it rather than growing a second one, so the
/// preview is built by the same code as the committed render and cannot drift from it. The `xy`
/// handed in comes from `engine_ops::vehicle_points`, which is that same `vehicle_rows` reader
/// filtered to placed rows — the same rows in the same order. The yrs-iteration-order
/// `vehicle_xy_flat` must never appear on this path: mixing the two orders is the trap.
///
/// The length gate is the one thing that is not structural. Both snapshots are read from the live
/// document, and a drag commits nothing until pointerup, so they agree; if they ever did not, a row
/// was added or removed between the two reads and every column after it would be shifted by one.
/// Rather than zip a shift, fall back to the plain disc lane — less information, but never a
/// confident lie.
fn bind_vehicle_preview_lane(e: &mut RenderEngine, xy: &[f32]) {
    let (doc_xy, aliases, tints, headings) = crate::editor::state::history::vehicle_lane_fields();
    if doc_xy.len() == xy.len() {
        e.vehicles_bind_symbology(xy, aliases, &tints, &headings);
    } else {
        e.vehicles_bind(xy);
    }
}

/// T-801 — re-upload squad tether hairlines with previewed endpoints for the live drag.
///
/// Composes with the slot `set_drag` + vehicle re-pack above: sprites already move on the GPU
/// preview; this keeps leader→member lines attached to those provisional positions. Only squads
/// that touch a dragged id are offset inside [`pack_squad_link_drag_preview`]; the lane upload is
/// still wholesale (hairline API replaces the role). `doc_handle` is the same live `Rc` the commit
/// path reads — no signature change for callers, matching `vehicle_lane_fields`.
fn bind_squad_link_preview(e: &mut RenderEngine, drag_ids: &[String], dx: f64, dy: f64) {
    let Some(doc_h) = crate::editor::state::history::doc_handle() else {
        return;
    };
    let guard = doc_h.borrow();
    let Some(doc) = guard.as_ref() else {
        return;
    };
    let soa = doc.materialize();
    let mut xy_by_slot: HashMap<String, (f32, f32)> = HashMap::with_capacity(soa.ids.len());
    for (i, id) in soa.ids.iter().enumerate() {
        xy_by_slot.insert(id.clone(), (soa.xy[i * 2], soa.xy[i * 2 + 1]));
    }
    let inputs = website_map_engine::editing::picking::squad_link_inputs(doc);
    #[allow(clippy::cast_possible_truncation)]
    let verts = pack_squad_link_drag_preview(&inputs, &xy_by_slot, drag_ids, dx as f32, dy as f32);
    #[allow(clippy::cast_possible_truncation)]
    let segment_count = (verts.len() / 12) as u32;
    e.upload_hairline_segments(role_id::SQUAD_LINKS, &verts, segment_count, true);
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
    // the panel used to be. `crate::editor::layout::*` owns the accessors (`eden_chrome` re-exports the
    // consts by name for the non-owned readers; the dynamic seam is the accessor).
    let (mut x0, mut x1) = (
        crate::editor::layout::dock_left_px(),
        w - crate::editor::layout::dock_right_px(),
    );
    let (mut y0, mut y1) = (
        crate::editor::layout::strip_top_px(),
        h - crate::editor::layout::toolbelt_band_px(),
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
// `crate::editor::layout::toolbelt_band_px()` (was `eden_chrome::TOOLBELT_BAND_PX`) — that read is one of
// the two the layout accessor-conversion test pins by name, and it must not hardcode `96.0`.
//
// T-637 equalised the docks to 240/240. This file needed no change for that, and THAT IS THE POINT:
// it reads `dock_left_px()`/`dock_right_px()`, never the numbers, so a width change reaches the
