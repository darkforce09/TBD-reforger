//! T-159.16 — MissionDoc host for the Leptos Mission Creator editor.
//!
//! The editor owns the `RenderEngine` directly since T-159.15; this slice gives it the mission
//! document too, as a **plain Rust `MissionDocCore`** (map-engine-core `doc` feature) living in the
//! SAME wasm module — no `map-engine-wasm` JS shim (spec D2), and no wasm-bindgen `.free()`, so the
//! React StrictMode double-free hazard cannot occur here (Rust `Drop` runs at most once).
//!
//! Scope is **lifecycle + a thin `window.__missionDoc` smoke bridge only** — NOT a `state/ydoc.ts`
//! mutator port (that lands in later slices). All document logic stays in Rust (the language gate);
//! the JS bridge only reads. The doc is held in the same `Rc<RefCell<Option<_>>>` idiom the engine
//! uses and leaks on route-leave exactly as the engine does (`on_cleanup` is `Send`-bound and cannot
//! hold the `!Send` `Rc`) — consistent, and free-of-double-free by plain-Rust ownership.
#![allow(clippy::cast_precision_loss)] // usize/u32 slot counters → f64 for the JS bridge; values are tiny

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use map_engine_core::doc::MissionDocCore;
use wasm_bindgen::prelude::*;

/// The hosted document. `Rc` is `!Send`, so — like the engine handle in `mission_editor.rs` — the
/// `Send`-bound `on_cleanup` cannot drop it; it leaks on route-leave (documented, consistent). There
/// is no manual free, so no double-free.
pub type DocHandle = Rc<RefCell<Option<MissionDocCore>>>;

/// Deterministic seed size. The `seed_random` generator writes exactly this many slots into the
/// `slots` map (no faction/squad/layer needed) in one transaction; with the core's fixed `CLIENT_ID`
/// this makes `encode_state()` byte-stable — the golden recorded in the verify log (spec D3).
const SEED_N: u32 = 8;
/// Fixed LCG seed — any change re-rolls the golden encode bytes.
const SEED: u64 = 0x0071_5916;
/// Everon bounds (matches `mission_editor.rs`), so seeded slot positions land in-world.
const TERRAIN_W: f64 = 12_800.0;
const TERRAIN_H: f64 = 12_800.0;

/// Build + seed a fresh document. The seed runs under the `INIT` origin (`set_origin_init`), so it is
/// never an undo step — mirroring how boot/hydrate/seed run in `state/ydoc.ts`.
#[must_use]
pub fn new_seeded_doc() -> DocHandle {
    let core = MissionDocCore::new();
    core.set_origin_init(true);
    core.seed_random(SEED_N, TERRAIN_W, TERRAIN_H, SEED);
    core.set_origin_init(false);
    Rc::new(RefCell::new(Some(core)))
}

/// Class R (encode round-trip) self-check, in Rust — the live equivalent of the core unit test
/// `encode_decode_roundtrip_is_stable`, run against the seeded hosted doc. It asserts exactly the two
/// properties that test proves:
///   1. **Re-encode stability** — encoding the *same* document twice is byte-identical (deterministic
///      v1 encode + fixed client id).
///   2. **Round-trip result-set equality (Class S)** — a fresh peer that replays the update
///      materializes the same slot set (matched by id; SoA row order is arbitrary) with equal x/rot.
fn roundtrip_ok(core: &MissionDocCore) -> bool {
    let bytes = core.encode_state();
    // (1) Re-encoding the same doc is byte-identical.
    if core.encode_state() != bytes {
        return false;
    }
    // (2) Replay into a fresh peer and compare materialized slots by id.
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

/// Lowercase hex of the encode stream, for the smoke gate to compare across two reads + log a prefix.
fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

/// Install `window.__missionDoc` — a thin, read-only smoke bridge (spec D6) mirroring
/// `register_self_checks`/`register_editor_cam` in `mission_editor.rs` (a `js_sys::Object` of
/// `.forget()`'d closures). Enough for the headless Class R gate; NOT the mutator surface. Each
/// closure returns `JsValue` (the proven `register_editor_cam` shape). Registered synchronously on
/// mount so the gate does not depend on the wgpu engine having come up.
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

    // T-169 smoke hook — bulk-add N slots so the virtual-outliner gate can exceed the window
    // threshold. `FnMut(f64)`, not the read-only `FnMut() -> JsValue` shape of the others.
    let seed_slots = Closure::wrap(Box::new(move |n: f64| {
        crate::editor_ops::debug_seed_slots(n.max(0.0) as u32);
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
    // The harness reads these across the page lifetime; leak them (the engine + its bridges leak too).
    slot_count.forget();
    seed_slots.forget();
    encode_hex.forget();
    change_version.forget();
    roundtrip.forget();
}
