//! T-159.17 — yrs document persistence (IndexedDB) for the Leptos Mission Creator editor.
//!
//! Parity port of the React `yrsPersist.ts` v3 persistence layer: the WHOLE `MissionDocCore`
//! `encode_state()` blob is stored as one record per mission in IndexedDB DB `tbd-mission-yrs`
//! (store `doc-state`, out-of-line key = mission id). A reload restores the local doc from that blob
//! before any server hydrate (server hydrate itself is a later slice / out of scope here).
//!
//! Ships three things (all logic in Rust — the language gate; the JS bridge only reads/triggers):
//!   1. `save_state`/`load_state`/`clear_state` — the async IDB access (via the `idb` crate).
//!   2. A **debounced + serialized-per-mission** writer (`save_state_debounced` + `flush_state`) with
//!      the React guards: `getBytes` read at write time, `isCancelled()` checked before reading,
//!      empty-blob skip (never clobber a good record), one write at a time per mission.
//!   3. `register_mission_persist` — the read-only `window.__missionPersist` smoke bridge
//!      (ready / loaded_from_storage / warm / persist_roundtrip_ok / flush / clear).
//!
//! NOT ported (T-159.17 non-goals): server hydrate/conflict GET, the mutator/autosave subscription
//! (the core has no change hook — the writer is driven by the post-seed/post-load encode), v1/v2 IDB
//! migration, Save-Version POST. The whole module is `wasm32`-gated in `main.rs`.
#![allow(clippy::cast_precision_loss)] // usize slot count → f64 for the JS bridge; tiny.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use idb::DatabaseEvent; // brings `VersionChangeEvent::database()` into scope for the upgrade handler
use leptos::task::spawn_local;
use map_engine_core::doc::MissionDocCore;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use crate::mission_doc::DocHandle;

/// IndexedDB coordinates — identical to `yrsPersist.ts` (`DB_NAME` / `STORE` / v1). Distinct from the
/// legacy v1 `tbd-mission-${id}` and v2 `tbd-mission-persist`; **no migration** (legacy drafts drop).
const DB_NAME: &str = "tbd-mission-yrs";
const STORE: &str = "doc-state";
const DB_VERSION: u32 = 1;
/// React `delay = 5000` — a burst of edits coalesces into one write (longer than v2's 2 s).
const DEBOUNCE_MS: i32 = 5000;

/* ───────────────────────────── IndexedDB access ───────────────────────────── */

/// Open the persistence DB, creating the `doc-state` store on first upgrade. Out-of-line keys
/// (`ObjectStoreParams::new()` with no `key_path`/`auto_increment`) — the key is supplied on every
/// `put`, mirroring the React `createObjectStore(STORE)`.
async fn open_db() -> Result<idb::Database, idb::Error> {
    let factory = idb::Factory::new()?;
    let mut req = factory.open(DB_NAME, Some(DB_VERSION))?;
    req.on_upgrade_needed(|event| {
        if let Ok(db) = event.database() {
            let _ = db.create_object_store(STORE, idb::ObjectStoreParams::new());
        }
    });
    req.await
}

/// Persist the whole encode blob under `id` (React `saveState` — `put(value, key)`). Stored as a
/// `Uint8Array` (structured clone), read back the same.
pub async fn save_state(id: &str, bytes: &[u8]) -> Result<(), idb::Error> {
    let db = open_db().await?;
    let tx = db.transaction(&[STORE], idb::TransactionMode::ReadWrite)?;
    let store = tx.object_store(STORE)?;
    let value = js_sys::Uint8Array::from(bytes);
    let key = JsValue::from_str(id);
    store.put(value.as_ref(), Some(&key))?.await?;
    tx.commit()?.await?;
    Ok(())
}

/// Load the blob for `id` (React `loadState` → `value ?? null`). Any error / absence → `None`.
pub async fn load_state(id: &str) -> Option<Vec<u8>> {
    let db = open_db().await.ok()?;
    let tx = db
        .transaction(&[STORE], idb::TransactionMode::ReadOnly)
        .ok()?;
    let store = tx.object_store(STORE).ok()?;
    let value: Option<JsValue> = store.get(JsValue::from_str(id)).ok()?.await.ok()?;
    let arr = value?.dyn_into::<js_sys::Uint8Array>().ok()?;
    Some(arr.to_vec())
}

/// Delete the blob for `id` (React `clearState`).
pub async fn clear_state(id: &str) -> Result<(), idb::Error> {
    let db = open_db().await?;
    let tx = db.transaction(&[STORE], idb::TransactionMode::ReadWrite)?;
    let store = tx.object_store(STORE)?;
    store.delete(JsValue::from_str(id))?.await?;
    tx.commit()?.await?;
    Ok(())
}

/* ─────────────────────── debounced + serialized writer ─────────────────────── */

type GetBytes = Box<dyn Fn() -> Vec<u8>>;
type IsCancelled = Box<dyn Fn() -> bool>;

struct PendingSave {
    get_bytes: GetBytes,
    is_cancelled: IsCancelled,
}

/// A live debounce timer: the `setTimeout` handle + the `Closure` it fires (kept alive here so it is
/// NOT leaked per-call; dropped when the timer is cleared/re-armed — never from inside its own fire).
struct TimerEntry {
    handle: i32,
    _closure: Closure<dyn FnMut()>,
}

thread_local! {
    // Module singletons, keyed by mission id — the exact React `timers`/`pending`/`chains` triple.
    // wasm is single-threaded, so a `thread_local! RefCell` is the sound analogue of the JS `Map`s.
    static TIMERS: RefCell<HashMap<String, TimerEntry>> = RefCell::new(HashMap::new());
    static PENDING: RefCell<HashMap<String, PendingSave>> = RefCell::new(HashMap::new());
    // Per-mission async lock so writes never interleave (React's promise chain). Uncontended in
    // .17 (no mutators) but required by the contract + correct once mutators land.
    static LOCKS: RefCell<HashMap<String, Rc<futures::lock::Mutex<()>>>> = RefCell::new(HashMap::new());
}

fn lock_for(id: &str) -> Rc<futures::lock::Mutex<()>> {
    LOCKS.with(|m| {
        m.borrow_mut()
            .entry(id.to_string())
            .or_insert_with(|| Rc::new(futures::lock::Mutex::new(())))
            .clone()
    })
}

/// Clear (and drop) any live timer for `id`. Called only from arm/flush — never from inside a
/// firing timer, so dropping the `Closure` here can't drop a running one.
fn clear_timer(id: &str) {
    if let Some(entry) = TIMERS.with(|t| t.borrow_mut().remove(id)) {
        if let Some(win) = web_sys::window() {
            win.clear_timeout_with_handle(entry.handle);
        }
    }
}

/// Serialized write: take the per-mission lock, then apply the React guards in order — cancel check
/// **before** reading bytes, empty-blob skip, then persist.
async fn run_save(id: &str, pending: PendingSave) {
    let lock = lock_for(id);
    let _guard = lock.lock().await;
    if (pending.is_cancelled)() {
        return;
    }
    let bytes = (pending.get_bytes)();
    if bytes.is_empty() {
        return; // never overwrite a good record with an empty/truncated blob
    }
    if let Err(e) = save_state(id, &bytes).await {
        web_sys::console::warn_1(&JsValue::from_str(&format!(
            "[yrs-persist] save failed: {e:?}"
        )));
    }
}

/// Debounced save (React `saveStateDebounced`). Stores the pending save, resets the timer (a burst
/// coalesces to one write), and on fire reads bytes at write time. `get_bytes`/`is_cancelled` are
/// evaluated inside `run_save`, so they must not hold any `RefCell` borrow across an `.await`
/// (callers pass closures that borrow transiently and return owned data).
pub fn save_state_debounced(id: &str, get_bytes: GetBytes, is_cancelled: IsCancelled, delay_ms: i32) {
    let id_owned = id.to_string();
    PENDING.with(|p| {
        p.borrow_mut().insert(
            id_owned.clone(),
            PendingSave {
                get_bytes,
                is_cancelled,
            },
        );
    });
    clear_timer(&id_owned); // reset — each call restarts the debounce window

    let Some(win) = web_sys::window() else {
        return;
    };
    let id_fire = id_owned.clone();
    let closure = Closure::<dyn FnMut()>::new(move || {
        // Fired: take the pending save and run it. We deliberately do NOT remove our own TIMERS
        // entry here (that would drop this running Closure); it is a harmless stale entry cleared on
        // the next arm/flush/clear.
        let pending = PENDING.with(|p| p.borrow_mut().remove(&id_fire));
        if let Some(pending) = pending {
            let id2 = id_fire.clone();
            spawn_local(async move { run_save(&id2, pending).await });
        }
    });
    let handle = win
        .set_timeout_with_callback_and_timeout_and_arguments_0(
            closure.as_ref().unchecked_ref(),
            delay_ms,
        )
        .unwrap_or(0);
    TIMERS.with(|t| {
        t.borrow_mut().insert(
            id_owned,
            TimerEntry {
                handle,
                _closure: closure,
            },
        );
    });
}

/// Flush any pending save now (React `flushState`): cancel the timer, then run the pending save
/// (honoring `isCancelled`) and await the serialized chain. On `visibilitychange`(hidden), `pagehide`,
/// and the smoke's explicit `flush()`.
pub async fn flush_state(id: &str) {
    clear_timer(id);
    let pending = PENDING.with(|p| p.borrow_mut().remove(id));
    if let Some(pending) = pending {
        run_save(id, pending).await;
    }
}

/// Register the flush-on-hide listeners (React T-062.1): `visibilitychange` → flush when the document
/// is hidden, and `pagehide` → flush. Both closures leak like the editor's wheel/pan handlers (the
/// doc + engine leak too; `on_cleanup` is `Send`-bound and can't hold them).
pub fn register_flush_on_hide(mission_id: String) {
    let Some(win) = web_sys::window() else {
        return;
    };

    if let Some(doc_target) = win.document() {
        let id = mission_id.clone();
        let on_vis = Closure::<dyn FnMut()>::new(move || {
            let hidden = web_sys::window()
                .and_then(|w| w.document())
                .is_some_and(|d| d.hidden());
            if hidden {
                let id = id.clone();
                spawn_local(async move { flush_state(&id).await });
            }
        });
        let _ = doc_target
            .add_event_listener_with_callback("visibilitychange", on_vis.as_ref().unchecked_ref());
        on_vis.forget();
    }

    let id = mission_id;
    let on_hide = Closure::<dyn FnMut()>::new(move || {
        let id = id.clone();
        spawn_local(async move { flush_state(&id).await });
    });
    let _ = win.add_event_listener_with_callback("pagehide", on_hide.as_ref().unchecked_ref());
    on_hide.forget();
}

/* ───────────────────────────── smoke bridge ───────────────────────────── */

/// Wrap a Rust future as a JS `Promise` WITHOUT `wasm-bindgen-futures`: the executor spawns the
/// future and resolves once it completes (only `js-sys` + `leptos::task::spawn_local`). The executor
/// is `FnMut` but runs once — `Option::take` yields the future exactly once.
fn spawn_promise<F>(fut: F) -> js_sys::Promise
where
    F: std::future::Future<Output = ()> + 'static,
{
    let mut fut = Some(fut);
    js_sys::Promise::new(&mut move |resolve: js_sys::Function, _reject: js_sys::Function| {
        if let Some(f) = fut.take() {
            spawn_local(async move {
                f.await;
                let _ = resolve.call0(&JsValue::NULL);
            });
        }
    })
}

/// A canonical, order-independent fingerprint of the materialized slots — the SEMANTIC Class R
/// oracle. Rows are keyed by slot id and sorted, floats compared bit-exactly (`f32::to_bits`), and
/// every interned `*_idx` is resolved to its string (so the arbitrary materialize row order / dict
/// first-seen order can't perturb the digest). Two docs with the same slot data ⇒ identical digest.
///
/// This is what the persist smoke compares across reload (cold vs warm), NOT the encode bytes:
/// `yrs`'s `encode_state_as_update_v1` is deterministic for the SAME doc but NOT byte-identical
/// between a doc and a fresh peer that replayed its update (only the *materialization* is equal — the
/// exact reason the core's `encode_decode_roundtrip_is_stable` test asserts materialization equality,
/// never `b.encode_state()==bytes`). A byte compare would be a false negative; this digest is sound.
fn slots_digest(core: &MissionDocCore) -> String {
    let soa = core.materialize();
    let get = |dict: &[String], idx: u32| dict.get(idx as usize).map_or("", String::as_str).to_string();
    let mut rows: Vec<String> = (0..soa.ids.len())
        .map(|i| {
            format!(
                "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
                soa.ids[i],
                soa.xs[i].to_bits(),
                soa.ys[i].to_bits(),
                soa.zs[i].to_bits(),
                soa.rotations[i].to_bits(),
                soa.stance[i],
                get(&soa.roles, soa.role_idx[i]),
                get(&soa.tags, soa.tag_idx[i]),
                get(&soa.squads, soa.squad_idx[i]),
                get(&soa.layers, soa.layer_idx[i]),
            )
        })
        .collect();
    rows.sort(); // canonical: each row is `id|…`, ids are unique → sort orders by id
    rows.join("\n")
}

/// Install `window.__missionPersist` — the read-only Class R gate bridge (mirrors
/// `register_mission_doc`: a `js_sys::Object` of `.forget()`'d closures). `ready`/`loaded` are shared
/// `Cell`s the boot task flips; the smoke waits on `ready()` (and `loaded_from_storage()` for the
/// WARM leg) before asserting.
pub fn register_mission_persist(
    doc: DocHandle,
    mission_id: String,
    ready: Rc<std::cell::Cell<bool>>,
    loaded: Rc<std::cell::Cell<bool>>,
) {
    let obj = js_sys::Object::new();

    let ready_fn = {
        let ready = ready.clone();
        Closure::wrap(Box::new(move || -> JsValue { JsValue::from_bool(ready.get()) })
            as Box<dyn FnMut() -> JsValue>)
    };
    let loaded_fn = {
        let loaded = loaded.clone();
        Closure::wrap(Box::new(move || -> JsValue { JsValue::from_bool(loaded.get()) })
            as Box<dyn FnMut() -> JsValue>)
    };
    let warm_fn = {
        let id = mission_id.clone();
        Closure::wrap(Box::new(move || -> JsValue {
            match crate::editor_session::read_warm(&id)
                .and_then(|s| serde_json::to_string(&s).ok())
            {
                Some(json) => JsValue::from_str(&json),
                None => JsValue::NULL,
            }
        }) as Box<dyn FnMut() -> JsValue>)
    };
    let digest_fn = {
        let doc = doc.clone();
        Closure::wrap(Box::new(move || -> JsValue {
            let digest = doc
                .borrow()
                .as_ref()
                .map(slots_digest)
                .unwrap_or_default();
            JsValue::from_str(&digest)
        }) as Box<dyn FnMut() -> JsValue>)
    };
    let flush_fn = {
        let id = mission_id.clone();
        Closure::wrap(Box::new(move || -> JsValue {
            let id = id.clone();
            spawn_promise(async move { flush_state(&id).await }).into()
        }) as Box<dyn FnMut() -> JsValue>)
    };
    let clear_fn = {
        let id = mission_id.clone();
        Closure::wrap(Box::new(move || -> JsValue {
            let id = id.clone();
            spawn_promise(async move {
                let _ = clear_state(&id).await;
                crate::editor_session::clear();
            })
            .into()
        }) as Box<dyn FnMut() -> JsValue>)
    };

    let _ = js_sys::Reflect::set(&obj, &JsValue::from_str("ready"), ready_fn.as_ref());
    let _ = js_sys::Reflect::set(
        &obj,
        &JsValue::from_str("loaded_from_storage"),
        loaded_fn.as_ref(),
    );
    let _ = js_sys::Reflect::set(&obj, &JsValue::from_str("warm"), warm_fn.as_ref());
    let _ = js_sys::Reflect::set(
        &obj,
        &JsValue::from_str("slots_digest"),
        digest_fn.as_ref(),
    );
    let _ = js_sys::Reflect::set(&obj, &JsValue::from_str("flush"), flush_fn.as_ref());
    let _ = js_sys::Reflect::set(&obj, &JsValue::from_str("clear"), clear_fn.as_ref());
    if let Some(win) = web_sys::window() {
        let _ = js_sys::Reflect::set(&win, &JsValue::from_str("__missionPersist"), &obj);
    }
    // The harness reads these across the page lifetime; leak them (the doc + its bridges leak too).
    ready_fn.forget();
    loaded_fn.forget();
    warm_fn.forget();
    digest_fn.forget();
    flush_fn.forget();
    clear_fn.forget();
}

/// The debounce default, exposed so the boot seam arms the initial persist with the contract delay.
#[must_use]
pub const fn debounce_ms() -> i32 {
    DEBOUNCE_MS
}
