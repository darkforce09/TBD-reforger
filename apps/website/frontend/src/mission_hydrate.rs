//! T-159.26 — server hydrate / conflict / dirty (the useMissionEditor `onSynced` + `resolveConflict`
//! port). The **data-safety** slice: before this the editor opened every real mission on the fixed
//! 8-slot seed, so a Save would overwrite the server version with seed data. Now a real (UUID)
//! mission's `current_version.json_payload` is fetched and hydrated into the doc (replacing the
//! seed), with a Keep-local / Load-server prompt when local IDB content genuinely diverges.
//!
//! **T-191 — "Load server version" is no longer a one-way door.** The adopt is a whole-document
//! replacement (`hydrate` clears nine root maps) and it used to run under the INIT origin, which the
//! `UndoManager` does not track (`store.rs` `tracked_origins` holds LOCAL only) — so Ctrl+Z could not
//! reach it. About five seconds later the debounced editor persist rewrote the mission's local
//! IndexedDB record with the adopted state, so the work was gone from memory *and* from the local
//! backup. Two independent recoveries now exist, and the conflict adopt takes both:
//!
//!   1. **Undo** — the conflict adopt runs under LOCAL, so `hydrate`'s single transaction becomes
//!      exactly one undo step (`capture_timeout_millis = 0`, `store.rs`). Ctrl/Cmd+Z puts the
//!      document back in-session. Covers the six undo-scoped roots (`slots` / `squads` / `factions` /
//!      `editorLayers` / `meta` / `vehicles`) — everything the editor can author.
//!   2. **Pre-adopt snapshot** — the whole `encode_state()` blob is captured *before* the hydrate and
//!      written to IndexedDB under its own key (`<id>::pre-adopt`), so the post-adopt persist cannot
//!      overwrite it and it outlives a reload (which drops the undo stack). Covers **all** roots,
//!      including the four the undo manager does not scope (`loadouts` / `items` / `objectives` /
//!      `markers`). Restored by [`restore_local_backup`] / `window.__missionBackup.restore()`.
//!
//! Boot hydrates (cold doc, still on the fixture seed) stay under INIT and take no snapshot: there is
//! no local work to lose, and a step there would make the user's first Ctrl+Z resurrect the 8 seed
//! slots.
//!
//! **Known gap, owned elsewhere:** the conflict modal itself (`mission_editor.rs`) still offers two
//! buttons with no diff and no change count, so the user still chooses blind — this slice only makes
//! the wrong choice survivable.
//!
//! **Gate safety:** the whole path is skipped for a non-UUID id (the gate route is
//! `/missions/smoke/edit`), so the 12 editor smokes — which all run on `smoke` — are untouched.
#![cfg(target_arch = "wasm32")]

use std::cell::RefCell;

use leptos::prelude::*;
use leptos::task::spawn_local;
use map_engine_core::doc::MissionDocCore;
use wasm_bindgen::prelude::*;

use crate::auth::AuthStore;
use crate::dto::MissionDetail;
use crate::mission_doc::DocHandle;

/// React `UUID_RE` — an id that can exist on the API. `smoke`/`draft` fail this and stay local.
fn is_uuid(id: &str) -> bool {
    let b = id.as_bytes();
    b.len() == 36
        && b.iter().enumerate().all(|(i, &c)| match i {
            8 | 13 | 18 | 23 => c == b'-',
            _ => c.is_ascii_hexdigit(),
        })
}

/// The lazily-minted default layer id (shared with `editor_ops`) — `hydrate` needs one for slots
/// whose layer was pruned.
const DEFAULT_LAYER_ID: &str = "layer-1";

/// Fetch `GET /missions/:id` and reconcile it with the just-loaded local doc:
///  * new mission (empty server payload) → apply the row terrain only;
///  * empty local (no IDB content) → hydrate the server payload, mark adopted, refresh;
///  * local content that derives from this exact server semver → trust local silently;
///  * genuinely divergent local content → set `conflict` so the UI can prompt.
///
/// `loaded_from_idb` is the persist layer's flag. On any non-404 failure it leaves the doc as-is
/// (local-only) — the caller shows no blocking error (the editor is usable on the local copy).
pub async fn hydrate_from_server(
    doc: DocHandle,
    id: String,
    auth: AuthStore,
    loaded_from_idb: bool,
    current_semver: RwSignal<Option<String>>,
    conflict: RwSignal<Option<crate::mission_editor::ConflictInfo>>,
) {
    // T-191 — the recovery bridge is registered on every editor boot, not only when a conflict
    // fires: after a reload the in-memory snapshot is gone and the IDB record is the only copy, and
    // that reload is exactly when someone reaches for it.
    register_mission_backup(id.clone());
    if !is_uuid(&id) {
        return;
    }
    let path = format!("/missions/{id}");
    let detail = match crate::client::api_get::<MissionDetail>(auth, &path).await {
        Ok(d) => d,
        Err((404, _)) => return, // ad-hoc/local-only id — stay local, silently
        Err(_) => {
            crate::toast::use_toasts()
                .error("Could not load the saved version — editing your local copy.");
            return;
        }
    };

    let row = RowMeta::from(&detail);
    let version = detail.current_version.as_ref();
    let semver = version.map(|v| v.semver.clone());
    current_semver.set(semver.clone());

    // The editor superset lives in `current_version.json_payload`; empty `{}` = a fresh mission.
    let payload = version.map(|v| &v.json_payload);
    let is_empty = payload
        .map(|p| p.as_object().is_none_or(serde_json::Map::is_empty))
        .unwrap_or(true);

    if is_empty {
        // A fresh real mission (no saved version). React's editor opens empty; the Leptos editor
        // seeds 8 fixture slots, so on the FIRST open (no IDB content) clear the seed to match —
        // a Save must not round-trip fixture data. A warm/IDB reopen keeps the user's local work.
        if !loaded_from_idb {
            adopt_payload(&doc, "{}", &row, Adopt::Init);
            crate::editor_session::mark_adopted(&id, semver.as_deref());
            crate::mission_history::set_dirty(false);
        } else {
            apply_row(&doc, &row);
        }
        return;
    }
    let payload_json = serde_json::to_string(payload.unwrap()).unwrap_or_default();

    if loaded_from_idb {
        // New-tab / warm cold boot: if local derives from this exact server version, the delta is
        // the user's own unsaved edits — trust local. Otherwise prompt.
        if let (Some(adopted), Some(sv)) = (crate::editor_session::read_adopted(&id), &semver) {
            if &adopted == sv {
                return;
            }
        }
        conflict.set(Some(crate::mission_editor::ConflictInfo {
            payload_json,
            semver,
        }));
    } else {
        // Empty local → adopt the server payload (replaces the seed). Cold doc: INIT, no snapshot.
        adopt_payload(&doc, &payload_json, &row, Adopt::Init);
        crate::editor_session::mark_adopted(&id, semver.as_deref());
        crate::mission_history::set_dirty(false);
    }
}

/// The "Load server" conflict resolution (React `resolveConflict('server')`): hydrate the offered
/// payload, adopt it, and mark clean. Clears the conflict signal.
///
/// T-191: this is the only adopt that runs over *live local work*, so it is the only one that takes
/// a [`snapshot_local_before_adopt`] and the only one that runs [`Adopt::Undoable`]. Both happen
/// before the conflict signal is cleared, so a failure to encode cannot leave the dialog gone AND the
/// work unrecoverable.
pub fn resolve_conflict_server(
    id: String,
    conflict: RwSignal<Option<crate::mission_editor::ConflictInfo>>,
) {
    if let (Some(c), Some(doc)) = (
        conflict.get_untracked(),
        crate::mission_history::doc_handle(),
    ) {
        // Capture the WHOLE local document before `hydrate` clears it. Synchronous encode (so the
        // bytes are pre-mutation by construction), deferred IDB write, own record key.
        let saved = snapshot_local_before_adopt(&doc, &id);
        // The payload carries its own map.terrain; the compile drops the title, so leave the
        // existing title untouched (row meta isn't refetched here).
        adopt_payload(&doc, &c.payload_json, &RowMeta::default(), Adopt::Undoable);
        crate::editor_session::mark_adopted(&id, c.semver.as_deref());
        crate::mission_history::set_dirty(false);
        // Tell the user the door swings both ways — the modal can't (it is gone by the next line),
        // and an undo nobody knows about is not a recovery.
        notify(&match saved {
            Some(n) => format!(
                "Loaded the server version. Your local copy ({n} objects) was backed up — press Ctrl/Cmd+Z to put it back."
            ),
            None => "Loaded the server version. Press Ctrl/Cmd+Z to undo.".to_string(),
        });
    }
    conflict.set(None);
}

/// The "Keep local" resolution (React `resolveConflict('local')`): local knowingly diverges, so
/// drop the adopted marker and mark dirty. Clears the conflict signal.
pub fn resolve_conflict_local(
    id: String,
    conflict: RwSignal<Option<crate::mission_editor::ConflictInfo>>,
) {
    crate::editor_session::mark_adopted(&id, None);
    crate::mission_history::set_dirty(true);
    conflict.set(None);
}

/// Mission-row fields from `GET /missions/:id` (title/terrain/time/weather) — the `apply_row_meta`
/// input.
#[derive(Default)]
struct RowMeta {
    title: String,
    terrain: String,
    time_of_day: String,
    weather: String,
}
impl RowMeta {
    fn from(d: &MissionDetail) -> Self {
        Self {
            title: d.title.clone(),
            terrain: d.terrain.clone(),
            time_of_day: d.time_of_day.clone(),
            weather: d.weather.clone(),
        }
    }
    fn is_empty(&self) -> bool {
        self.title.is_empty() && self.terrain.is_empty()
    }
}

/// T-191 — whether an adopt is reachable by Ctrl+Z. The choice is purely the transaction origin
/// `hydrate` runs under; the `UndoManager` tracks LOCAL and ignores INIT (`store.rs` `tracked_origins`).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Adopt {
    /// Boot hydrate over a cold document (the 8-slot fixture seed, or an empty local). INIT: there is
    /// nothing authored to lose, and an undo step here would make the user's first Ctrl+Z resurrect
    /// the seed the hydrate just removed.
    Init,
    /// Conflict resolution over live local work. LOCAL, so the single `hydrate` transaction becomes
    /// exactly one undo step and Ctrl+Z brings the slots / squads / factions / editorLayers / meta /
    /// vehicles back — the six roots `store.rs` puts in the undo manager's scope, which is every root
    /// the editor can author into.
    ///
    /// **Partial by construction:** `hydrate` also clears `loadouts` / `items` / `objectives` /
    /// `markers`, which are *not* in scope, so after an undo those four still hold the server
    /// payload's rows. Nothing in the editor writes them (they are hydrate-only today), but that is
    /// the reason the pre-adopt snapshot exists as well — it is whole-document and covers all nine.
    ///
    /// Cost, accepted deliberately: yrs keeps the deleted blocks alive for as long as the stack item
    /// exists (`undo.rs` marks in-scope deletions `keep(true)`), so the pre-adopt document stays
    /// resident — roughly one extra copy of the doc until the stack is trimmed or the page reloads.
    /// The alternative was destroying it, which is the defect.
    Undoable,
}

/// Hydrate a compiled payload into the doc, then rebind the engine glyphs + persist via the shared
/// tail. `mode` decides whether the replacement is an undo step (see [`Adopt`]); `after_local_edit`
/// then rebinds/persists (and marks dirty — the caller clears it).
fn adopt_payload(doc: &DocHandle, payload_json: &str, row: &RowMeta, mode: Adopt) {
    // An `Undoable` adopt must stay exactly ONE step: `hydrate` and `apply_row_meta` are separate
    // transactions and `capture_timeout_millis = 0` gives each its own stack item, so a non-empty row
    // here would need two Ctrl+Z presses to fully revert. The one undoable caller passes an empty row
    // (the conflict payload carries its own terrain and the row meta is not refetched), so the branch
    // below cannot run under `Undoable` — asserted rather than assumed.
    debug_assert!(mode == Adopt::Init || row.is_empty());
    {
        let guard = doc.borrow();
        let Some(core) = guard.as_ref() else {
            return;
        };
        core.set_origin_init(mode == Adopt::Init);
        core.hydrate(payload_json, DEFAULT_LAYER_ID);
        if !row.is_empty() {
            core.apply_row_meta(
                &row.title,
                &row.terrain,
                opt(&row.time_of_day),
                opt(&row.weather),
            );
        }
        core.set_origin_init(false);
    }
    // Rebind glyphs + HUD + schedule the persist (the drag-commit / undo tail). It sets dirty=true;
    // the caller corrects to false after marking adopted.
    crate::mission_history::after_local_edit();
}

/// Apply the row meta to a doc with no server payload (fresh mission) under INIT.
fn apply_row(doc: &DocHandle, row: &RowMeta) {
    if row.is_empty() {
        return;
    }
    let guard = doc.borrow();
    if let Some(core) = guard.as_ref() {
        core.set_origin_init(true);
        core.apply_row_meta(
            &row.title,
            &row.terrain,
            opt(&row.time_of_day),
            opt(&row.weather),
        );
        core.set_origin_init(false);
    }
}

fn opt(s: &str) -> Option<String> {
    (!s.is_empty()).then(|| s.to_string())
}

/* ─────────────────── T-191 — pre-adopt local backup + restore ─────────────────── */

/// IndexedDB record key for the pre-adopt snapshot. Same DB/store as the live doc
/// (`tbd-mission-yrs` / `doc-state`, out-of-line keys) but a **suffixed** key, which is the whole
/// point: the debounced editor persist re-arms on the adopt and rewrites the plain mission id a few
/// seconds later, and that write must not be able to reach this record. A mission id is a UUID
/// (`is_uuid`), so the suffix can never collide with a real one.
fn backup_key(mission_id: &str) -> String {
    format!("{mission_id}::pre-adopt")
}

/// The in-memory half of the snapshot — an instant, IDB-independent restore for the session that
/// took the adopt. Keyed by mission id so navigating to another mission can't restore the wrong doc.
struct LocalBackup {
    mission_id: String,
    bytes: Vec<u8>,
}

thread_local! {
    static LOCAL_BACKUP: RefCell<Option<LocalBackup>> = const { RefCell::new(None) };
}

/// Capture the whole local document **before** a destructive adopt.
///
/// `encode_state()` is the same v1 update stream the persist layer stores and the boot seam replays
/// (`mission_editor` step 1), so a snapshot is restorable by exactly the path the editor already
/// proves on every warm reload — no new serialization format, no new trust.
///
/// The encode is synchronous and runs before any mutation, so the bytes are pre-adopt by
/// construction; only the IDB write is deferred. Returns the slot count captured, or `None` when
/// there was nothing to write (an empty blob would only replace a good record with a bad one — the
/// `yrs_persist::run_save` rule).
fn snapshot_local_before_adopt(doc: &DocHandle, mission_id: &str) -> Option<usize> {
    let (bytes, slots) = {
        let guard = doc.borrow();
        let core = guard.as_ref()?;
        (core.encode_state(), core.slot_count())
    };
    if bytes.is_empty() {
        return None;
    }
    LOCAL_BACKUP.with(|b| {
        *b.borrow_mut() = Some(LocalBackup {
            mission_id: mission_id.to_string(),
            bytes: bytes.clone(),
        });
    });
    let key = backup_key(mission_id);
    spawn_local(async move {
        if let Err(e) = crate::yrs_persist::save_state(&key, &bytes).await {
            // Non-fatal: the in-memory copy and the undo step are both still standing.
            web_sys::console::warn_1(&JsValue::from_str(&format!(
                "[t191] pre-adopt backup save failed: {e:?}"
            )));
        }
    });
    Some(slots)
}

/// Is a pre-adopt snapshot on record for `mission_id`? Checks the in-session copy first, then IDB
/// (the copy that outlives a reload).
async fn has_local_backup(mission_id: &str) -> bool {
    let in_memory = LOCAL_BACKUP.with(|b| {
        b.borrow()
            .as_ref()
            .is_some_and(|s| s.mission_id == mission_id)
    });
    if in_memory {
        return true;
    }
    crate::yrs_persist::load_state(&backup_key(mission_id))
        .await
        .is_some_and(|b| !b.is_empty())
}

/// Restore the pre-adopt snapshot over the live document — the "I did not mean that" lever. Prefers
/// the in-session copy, falls back to the IDB record. `true` when the document was replaced.
///
/// The snapshot is **not** consumed: after a restore the server version is still one refetch away,
/// while the local work exists nowhere else, so the safer record to keep is this one.
pub async fn restore_local_backup(mission_id: String) -> bool {
    let cached = LOCAL_BACKUP.with(|b| {
        b.borrow()
            .as_ref()
            .filter(|s| s.mission_id == mission_id)
            .map(|s| s.bytes.clone())
    });
    let bytes = match cached {
        Some(b) => b,
        None => crate::yrs_persist::load_state(&backup_key(&mission_id))
            .await
            .unwrap_or_default(),
    };
    if bytes.is_empty() {
        return false;
    }
    let Some(doc) = crate::mission_history::doc_handle() else {
        return false;
    };
    // Rebuild as a FRESH core and swap, exactly like the boot IDB restore. Applying the update over
    // the adopted doc would MERGE the two states — yrs is a CRDT, and replaying an old update can
    // never delete the rows the adopt inserted — which is the one thing a restore must not do.
    let fresh = MissionDocCore::new();
    fresh.set_origin_init(true);
    let ok = fresh.apply_update(&bytes).is_ok();
    fresh.set_origin_init(false);
    if !ok {
        return false;
    }
    *doc.borrow_mut() = Some(fresh);
    // The local doc no longer derives from the server semver we adopted, so drop the marker or the
    // next cold boot would silently trust local against the wrong version.
    crate::editor_session::mark_adopted(&mission_id, None);
    // Wholesale document swap: rebind glyphs/HUD/docks (`after_local_edit` would be wrong — it
    // rebinds from a doc it assumes was edited in place), then mark dirty and re-arm the persist so
    // the restored document becomes the local record rather than the adopted one.
    crate::mission_history::rebind_engine_from_doc();
    crate::mission_history::set_dirty(true);
    crate::yrs_persist::schedule_edit_persist(doc, &mission_id);
    notify("Restored your local copy. The server version is unchanged — reopen the mission to load it again.");
    true
}

/// Toast without `expect_context`. [`restore_local_backup`] can be driven from a JS bridge closure,
/// which has no reactive Owner, and `use_toasts()` would panic there — a panic in the middle of a
/// recovery being the worst possible time for one.
fn notify(msg: &str) {
    if let Some(toasts) = use_context::<crate::toast::Toasts>() {
        toasts.message(msg);
    }
}

/// Install `window.__missionBackup` — the recovery surface for the pre-adopt snapshot, and the peer
/// of `__missionDoc` / `__missionPersist` / `__editorHistory` (a `js_sys::Object` of `.forget()`'d
/// closures). Two Promise-returning verbs:
///   * `has()`     → bool — is a pre-adopt snapshot on record for this mission?
///   * `restore()` → bool — swap it back over the live document.
///
/// Unlike its read-only peers this one mutates, on purpose: a backup nobody can restore is not a
/// backup. It is also the only surface this slice can offer — the conflict modal lives in
/// `mission_editor.rs`, which another slice owns this wave, so the in-product "Undo this" button is
/// a follow-up that can call [`restore_local_backup`] directly.
fn register_mission_backup(mission_id: String) {
    let obj = js_sys::Object::new();

    let has_fn = {
        let id = mission_id.clone();
        Closure::wrap(Box::new(move || -> JsValue {
            let id = id.clone();
            wasm_bindgen_futures::future_to_promise(async move {
                Ok(JsValue::from_bool(has_local_backup(&id).await))
            })
            .into()
        }) as Box<dyn FnMut() -> JsValue>)
    };
    let restore_fn = Closure::wrap(Box::new(move || -> JsValue {
        let id = mission_id.clone();
        wasm_bindgen_futures::future_to_promise(async move {
            Ok(JsValue::from_bool(restore_local_backup(id).await))
        })
        .into()
    }) as Box<dyn FnMut() -> JsValue>);

    let _ = js_sys::Reflect::set(&obj, &JsValue::from_str("has"), has_fn.as_ref());
    let _ = js_sys::Reflect::set(&obj, &JsValue::from_str("restore"), restore_fn.as_ref());
    if let Some(win) = web_sys::window() {
        let _ = js_sys::Reflect::set(&win, &JsValue::from_str("__missionBackup"), &obj);
    }
    // Read across the page lifetime; leak like every other editor bridge.
    has_fn.forget();
    restore_fn.forget();
}
