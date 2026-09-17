//! Role: the browser half of the selection tool — the drag preview lanes and the read-only
//! `window.__editorSelection` smoke bridge.
//! Position: `editor/input/tools` in the frontend editor.
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

use crate::v2::apps::editor::bridge::document_host::doc_host::DocHandle;

/// Previews a selected drag in the slot, vehicle, and squad-link render lanes.
/// Every lane uses the same ids and world delta, so the preview matches the eventual move.
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
    bind_squad_link_preview(e, ids, dx, dy);
}

/// Restores authored positions in all drag preview lanes after cancellation or completion.
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
    bind_squad_link_preview(e, &[], 0.0, 0.0);
}

/// Binds vehicle previews with their authored kind, side, and heading columns.
/// Column lengths must match the preview coordinates; otherwise the plain disc lane avoids
/// associating another vehicle's symbology with a position.
fn bind_vehicle_preview_lane(e: &mut RenderEngine, xy: &[f32]) {
    let (doc_xy, aliases, tints, headings) =
        crate::v2::apps::editor::bridge::document_host::history::vehicle_lane_fields();
    if doc_xy.len() == xy.len() {
        e.vehicles_bind_symbology(xy, aliases, &tints, &headings);
    } else {
        e.vehicles_bind(xy);
    }
}

/// Rebinds squad-link hairlines to previewed member and leader positions.
fn bind_squad_link_preview(e: &mut RenderEngine, drag_ids: &[String], dx: f64, dy: f64) {
    let Some(doc_h) = crate::v2::apps::editor::bridge::document_host::history::doc_handle() else {
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

/// Finds a screen point farthest from projected slots for the deselection probe.
/// Candidates stay inside the map region uncovered by docks and the top strip.
fn farthest_empty_px(w: f64, h: f64, proj: &[(f64, f64)]) -> (f64, f64) {
    let (nx, ny) = (21usize, 13usize);
    let (mut x0, mut x1) = (
        crate::v2::apps::editor::shell::layout::dock_left_px(),
        w - crate::v2::apps::editor::shell::layout::dock_right_px(),
    );
    let (mut y0, mut y1) = (
        crate::v2::apps::editor::shell::layout::strip_top_px(),
        h - crate::v2::apps::editor::shell::layout::toolbelt_band_px(),
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

/// Builds the movement probe using a centred seed and a 40 px drag target.
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

/// Builds the marquee probe and its expected selection from the same picker as the gesture.
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
    let start = cam.unproject_xy(x0, y0);
    let expect = marquee_ids(&cam, &soa, start[0], start[1], x1, y1);

    let mut s = String::from("{\"rect\":[");
    s.push_str(&format!("{x0},{y0},{x1},{y1}],\"expect_ids\":"));
    s.push_str(&json_id_array(&expect));
    s.push_str(&format!(",\"expect_count\":{}}}", expect.len()));
    s
}

/// Registers the read-only `window.__editorSelection` smoke bridge.
/// Probe hooks may centre the camera; selection reads use the live handles retained by the page.
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
    count.forget();
    ids.forget();
    selfcheck.forget();
    probe.forget();
    marquee_selfcheck_fn.forget();
    probe_marquee.forget();
    probe_move.forget();
}
