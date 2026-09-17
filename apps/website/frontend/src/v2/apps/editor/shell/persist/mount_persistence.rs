//! Exposes completed flush state and the mission persistence bridge.
use super::*;
/* ─────────────────────────────  the last-flush timestamp ───────────────────────────── */

thread_local! {
    // the epoch-ms instant of the last COMPLETED flush (see `note_flush_completed`), and the
    // reactive signal the top strip's "draft saved Ns ago" chip subscribes to. ONE exposed signal,
    // parked here in the `context_menu::MENU` / `eden_settings::PREFS_OPEN` idiom: the strip creates
    // it (it has a reactive owner during render; this module runs in detached timers/tasks that do
    // not) and hands it over via `set_last_flush_signal`, and this module only ever `.set()`s it.
    //
    // The `Cell` behind it is the source of truth for the ack, always writable with no owner, so a
    // flush that completes before the strip has installed its signal is not lost: `last_flush_ms`
    // reads the `Cell`, and the strip seeds the signal from it on install. `None` ⇒ no flush has
    // completed this page lifetime  which is precisely "a mission that has never been edited",
    // because a never-edited doc is content-empty and the  guard refuses to write it, so no
    // flush can complete and no chip is shown. This is NOT a second dirtiness source: the flush only
    // happens because an edit armed the writer, so the ack is strictly downstream of the same edit
    // that arms the dirty dot (the  one-source rule).
    static LAST_FLUSH_MS: Cell<Option<f64>> = const { Cell::new(None) };
    static LAST_FLUSH_SIG: RefCell<Option<leptos::prelude::RwSignal<Option<f64>>>> =
        const { RefCell::new(None) };
}

/// a flush COMPLETED (called only from [`run_save`]'s success branch). Records the instant in
/// the `Cell` (the always-available ack) and, if the strip has installed its signal, pushes the new
/// value so the chip re-renders. Two writes of one value: the `Cell` is the truth a late-installed
/// signal seeds from; the signal is the reactive mirror.
///
/// The instant is `Date.now()`  wall-clock epoch ms. The chip renders a *recency* (now − last
/// flush) with both ends read from that same clock, so a monotonic source buys nothing and the wall
/// clock is the one the browser hands back cheaply.
#[cfg(target_arch = "wasm32")]
pub(super) fn note_flush_completed() {
    use leptos::prelude::Set;
    let ts = js_sys::Date::now();
    LAST_FLUSH_MS.with(|c| c.set(Some(ts)));
    LAST_FLUSH_SIG.with(|s| {
        if let Some(sig) = *s.borrow() {
            sig.set(Some(ts));
        }
    });
}

/// Native no-op — [`run_save`]'s IO is wasm-only (`save_state_as` hits IndexedDB), so on the native
/// test shell a flush never completes and there is nothing to record. Keeps [`run_save`] free of a
/// second `cfg` at the call site.
#[cfg(not(target_arch = "wasm32"))]
pub(super) fn note_flush_completed() {}

/// install the strip's last-flush signal (once, from `TopCommandStrip` setup, which has the
/// reactive owner this module lacks). Seeds it from the `Cell` so a flush that already completed this
/// page lifetime is reflected immediately, then parks it for [`note_flush_completed`] to push to.
///
/// Idempotent-by-overwrite like `eden_settings::set_prefs_signal`: a remount hands over a fresh
/// signal and the stale one is dropped. `RwSignal` is `Copy`, wasm is single-threaded.
pub fn set_last_flush_signal(sig: leptos::prelude::RwSignal<Option<f64>>) {
    use leptos::prelude::Set;
    sig.set(LAST_FLUSH_MS.with(Cell::get));
    LAST_FLUSH_SIG.with(|s| *s.borrow_mut() = Some(sig));
}

/// the epoch-ms instant of the last completed flush, or `None` if none has completed this
/// page lifetime. Reads the `Cell` (not the signal) so it is correct before the strip mounts and
/// needs no reactive owner; exposed on the `__missionPersist` bridge so the scripted acceptance can
/// assert the recency after an edit + debounce, and its reset after reload.
#[must_use]
pub fn last_flush_ms() -> Option<f64> {
    LAST_FLUSH_MS.with(Cell::get)
}

/// Install `window.__missionPersist`  the read-only Class R gate bridge (mirrors
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
        Closure::wrap(
            Box::new(move || -> JsValue { JsValue::from_bool(ready.get()) })
                as Box<dyn FnMut() -> JsValue>,
        )
    };
    let loaded_fn = {
        let loaded = loaded.clone();
        Closure::wrap(
            Box::new(move || -> JsValue { JsValue::from_bool(loaded.get()) })
                as Box<dyn FnMut() -> JsValue>,
        )
    };
    let warm_fn = {
        let id = mission_id.clone();
        Closure::wrap(Box::new(move || -> JsValue {
            match crate::v2::apps::editor::shell::session::read_warm(&id)
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
                .map(slot_fingerprint::slots_digest)
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
                crate::v2::apps::editor::shell::session::clear();
            })
            .into()
        }) as Box<dyn FnMut() -> JsValue>)
    };
    let edit_count_fn = Closure::wrap(Box::new(move || -> JsValue {
        JsValue::from_f64(f64::from(edit_persist_count()))
    }) as Box<dyn FnMut() -> JsValue>);
    // the last COMPLETED flush's epoch-ms instant, or `null` if none has completed this page
    // lifetime. Read-only, side-effect-free (reads the `LAST_FLUSH_MS` cell). The scripted F-24
    // acceptance keys the "draft saved Ns ago" chip's recency off this: after an edit + the ≤1 s
    // debounce it is a fresh timestamp; on a never-edited mission it stays `null` (the content guard
    // refuses the empty write, so no flush completes) and the chip is absent; after reload it resets.
    let last_flush_fn = Closure::wrap(Box::new(move || -> JsValue {
        last_flush_ms().map_or(JsValue::NULL, JsValue::from_f64)
    }) as Box<dyn FnMut() -> JsValue>);
    // the refusal counters, as JSON. A guard that only ever *declines* to act is invisible:
    // "the record is still good" is equally consistent with the guard firing and with no write
    // having been attempted at all. These make the refusal itself observable, so a probe can assert
    // the guard ran rather than asserting the absence of damage.
    let blocked_fn = Closure::wrap(Box::new(move || -> JsValue {
        JsValue::from_str(&format!(
            r#"{{"empty":{},"unreadable":{},"read_only":{},"merged":{}}}"#,
            BLOCKED_EMPTY.with(Cell::get),
            BLOCKED_UNREADABLE.with(Cell::get),
            BLOCKED_READ_ONLY.with(Cell::get),
            MERGED_WRITES.with(Cell::get)
        ))
    }) as Box<dyn FnMut() -> JsValue>);
    // the crux, evaluated in the REAL wasm runtime rather than argued about.
    //
    // Constructs a fresh `MissionDocCore`, encodes it, and reports the bytes alongside both
    // verdicts: what `bytes.is_empty()` would have decided and what the guard decides now. This
    // exists because the defect is a claim about a specific byte sequence  an empty document
    // encodes to `[0, 0]`, which is two bytes and therefore not empty  and a claim about bytes
    // should be checkable on the target that produces them, not only on a native probe where
    // var-int width or the yrs build could in principle differ.
    //
    // Read-only and side-effect-free: it touches neither the live document nor IndexedDB.
    let empty_encode_fn = Closure::wrap(Box::new(move || -> JsValue {
        let fresh = MissionDocCore::new();
        let bytes = fresh.encode_state();
        let list = bytes
            .iter()
            .map(u8::to_string)
            .collect::<Vec<_>>()
            .join(",");
        JsValue::from_str(&format!(
            r#"{{"bytes":[{list}],"len":{},"isEmpty":{},"hasContent":{},"oldGuardWouldWrite":{},"newGuardWrites":{}}}"#,
            bytes.len(),
            bytes.is_empty(),
            fresh.has_content(),
            !bytes.is_empty(),
            restores_to_authored_content(&bytes)
        ))
    }) as Box<dyn FnMut() -> JsValue>);
    // the content predicate, over the record on disk for this mission. Answers "is what is
    // stored actually restorable to authored content", which is the question the old byte test only
    // appeared to answer.
    let stored_has_content_fn = {
        let id = mission_id.clone();
        Closure::wrap(Box::new(move || -> JsValue {
            let id = id.clone();
            wasm_bindgen_futures::future_to_promise(async move {
                let stored = load_state(&id).await;
                Ok(JsValue::from_str(&format!(
                    r#"{{"present":{},"bytes":{},"hasContent":{}}}"#,
                    stored.is_some(),
                    stored.as_ref().map_or(0, Vec::len),
                    stored.as_deref().is_some_and(restores_to_authored_content)
                )))
            })
            .into()
        }) as Box<dyn FnMut() -> JsValue>)
    };
    // the recovery surface for records written before per-account scoping. `orphans()`
    // answers "what is stranded on this machine" as a JSON array of logical keys; `adopt_orphans()`
    // claims them for the signed-in account. Both are Promise-returning like `flush`/`clear`.
    let orphans_fn = Closure::wrap(Box::new(move || -> JsValue {
        wasm_bindgen_futures::future_to_promise(async move {
            let keys = orphan_keys().await;
            Ok(JsValue::from_str(
                &serde_json::to_string(&keys).unwrap_or_else(|_| "[]".to_string()),
            ))
        })
        .into()
    }) as Box<dyn FnMut() -> JsValue>);
    let adopt_fn = Closure::wrap(Box::new(move || -> JsValue {
        wasm_bindgen_futures::future_to_promise(async move {
            // Refused while signed out, for the reason the whole ticket exists: adoption files a
            // document under an account, and there is no account to file it under.
            let Some(owner) = current_owner() else {
                return Ok(JsValue::from_str(
                    r#"{"error":"signed out — sign in first, then adopt"}"#,
                ));
            };
            let (adopted, skipped) = adopt_orphans(&owner).await;
            Ok(JsValue::from_str(&format!(
                r#"{{"adopted":{adopted},"skipped":{skipped}}}"#
            )))
        })
        .into()
    }) as Box<dyn FnMut() -> JsValue>);

    let _ = js_sys::Reflect::set(&obj, &JsValue::from_str("ready"), ready_fn.as_ref());
    let _ = js_sys::Reflect::set(
        &obj,
        &JsValue::from_str("loaded_from_storage"),
        loaded_fn.as_ref(),
    );
    let _ = js_sys::Reflect::set(&obj, &JsValue::from_str("warm"), warm_fn.as_ref());
    let _ = js_sys::Reflect::set(&obj, &JsValue::from_str("slots_digest"), digest_fn.as_ref());
    let _ = js_sys::Reflect::set(&obj, &JsValue::from_str("flush"), flush_fn.as_ref());
    let _ = js_sys::Reflect::set(&obj, &JsValue::from_str("clear"), clear_fn.as_ref());
    let _ = js_sys::Reflect::set(
        &obj,
        &JsValue::from_str("edit_persist_count"),
        edit_count_fn.as_ref(),
    );
    let _ = js_sys::Reflect::set(
        &obj,
        &JsValue::from_str("last_flush_ms"),
        last_flush_fn.as_ref(),
    );
    let _ = js_sys::Reflect::set(&obj, &JsValue::from_str("orphans"), orphans_fn.as_ref());
    let _ = js_sys::Reflect::set(&obj, &JsValue::from_str("adopt_orphans"), adopt_fn.as_ref());
    let _ = js_sys::Reflect::set(
        &obj,
        &JsValue::from_str("blocked_writes"),
        blocked_fn.as_ref(),
    );
    let _ = js_sys::Reflect::set(
        &obj,
        &JsValue::from_str("stored_has_content"),
        stored_has_content_fn.as_ref(),
    );
    let _ = js_sys::Reflect::set(
        &obj,
        &JsValue::from_str("empty_encode_probe"),
        empty_encode_fn.as_ref(),
    );
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
    edit_count_fn.forget();
    last_flush_fn.forget();
    orphans_fn.forget();
    adopt_fn.forget();
    blocked_fn.forget();
    stored_has_content_fn.forget();
    empty_encode_fn.forget();

    // one eviction sweep per editor boot. Spawned rather than awaited so the bridge stays
    // synchronously installed for the gate, and safe against the boot restore racing it: it only
    // ever deletes keys of a *different* owner, and the restore only ever reads this one's.
    spawn_local(async move { evict_foreign_records().await });
}
