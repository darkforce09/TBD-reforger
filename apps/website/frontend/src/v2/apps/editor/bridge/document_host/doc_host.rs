//! Mission document ownership and seed setup.
#![allow(clippy::cast_precision_loss)] // usize/u32 slot counters → f64 for the JS bridge; values are tiny

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::v2::apps::editor::ui::outliner::outliner;
use wasm_bindgen::prelude::*;
use website_map_engine::data::store::MissionDocCore;

/// Shared ownership of the optional active mission document.
pub type DocHandle = Rc<RefCell<Option<MissionDocCore>>>;

const SEED_N: u32 = 8;
const SEED: u64 = 0x0071_5916;
const TERRAIN_W: f64 = 12_800.0;
const TERRAIN_H: f64 = 12_800.0;

/// Creates a new mission document with deterministic seed slots.
#[must_use]
pub fn new_seeded_doc() -> DocHandle {
    let core = MissionDocCore::new();
    core.set_origin_init(true);
    core.seed_random(SEED_N, TERRAIN_W, TERRAIN_H, SEED);
    core.set_origin_init(false);
    Rc::new(RefCell::new(Some(core)))
}

fn roundtrip_ok(core: &MissionDocCore) -> bool {
    let bytes = core.encode_state();
    if core.encode_state() != bytes {
        return false;
    }
    let fresh = MissionDocCore::new();
    if fresh.apply_update(&bytes).is_err() {
        return false;
    }
    let a = core.materialize();
    let b = fresh.materialize();
    if a.ids.len() != b.ids.len() {
        return false;
    }
    for (i, id) in a.ids.iter().enumerate() {
        let Some(j) = b.ids.iter().position(|x| x == id) else {
            return false;
        };
        if a.xs[i] != b.xs[j] || a.rotations[i] != b.rotations[j] {
            return false;
        }
    }
    true
}

fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

/// Registers the active mission document and version counter.
pub fn register_mission_doc(doc: DocHandle, ver: Rc<Cell<u32>>) {
    let obj = js_sys::Object::new();

    let slot_count = {
        let doc = doc.clone();
        Closure::wrap(Box::new(move || -> JsValue {
            let n = match doc.borrow().as_ref() {
                Some(c) => c.slot_count() as f64,
                None => 0.0,
            };
            JsValue::from_f64(n)
        }) as Box<dyn FnMut() -> JsValue>)
    };
    let encode_hex = {
        let doc = doc.clone();
        Closure::wrap(Box::new(move || -> JsValue {
            let h = match doc.borrow().as_ref() {
                Some(c) => hex(&c.encode_state()),
                None => String::new(),
            };
            JsValue::from_str(&h)
        }) as Box<dyn FnMut() -> JsValue>)
    };
    let change_version = {
        let ver = ver.clone();
        Closure::wrap(
            Box::new(move || -> JsValue { JsValue::from_f64(f64::from(ver.get())) })
                as Box<dyn FnMut() -> JsValue>,
        )
    };
    let roundtrip = {
        let doc = doc.clone();
        Closure::wrap(Box::new(move || -> JsValue {
            let ok = match doc.borrow().as_ref() {
                Some(c) => roundtrip_ok(c),
                None => false,
            };
            JsValue::from_bool(ok)
        }) as Box<dyn FnMut() -> JsValue>)
    };

    let seed_slots = Closure::wrap(Box::new(move |n: f64| {
        debug_seed_slots(n.max(0.0) as u32);
    }) as Box<dyn FnMut(f64)>);

    let _ = js_sys::Reflect::set(&obj, &JsValue::from_str("slot_count"), slot_count.as_ref());
    let _ = js_sys::Reflect::set(&obj, &JsValue::from_str("seed_slots"), seed_slots.as_ref());
    let _ = js_sys::Reflect::set(&obj, &JsValue::from_str("encode_hex"), encode_hex.as_ref());
    let _ = js_sys::Reflect::set(
        &obj,
        &JsValue::from_str("change_version"),
        change_version.as_ref(),
    );
    let _ = js_sys::Reflect::set(&obj, &JsValue::from_str("roundtrip_ok"), roundtrip.as_ref());
    if let Some(win) = web_sys::window() {
        let _ = js_sys::Reflect::set(&win, &JsValue::from_str("__missionDoc"), &obj);
    }
    slot_count.forget();
    seed_slots.forget();
    encode_hex.forget();
    change_version.forget();
    roundtrip.forget();
}

fn debug_seed_slots(n: u32) {
    use crate::v2::apps::editor::bridge::host_state::editor_context::EDITOR_CONTEXT;
    use outliner::ensure_active_layer;

    EDITOR_CONTEXT.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return;
        };
        let layer_id = ensure_active_layer(core);
        website_map_engine::data::store::operations::entity::seed_debug_slots(
            core,
            &ctx.next_id,
            &layer_id,
            n,
        );
    });
    crate::v2::apps::editor::bridge::document_host::history::after_local_edit();
}
