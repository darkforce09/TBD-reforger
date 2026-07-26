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
//!      (ready / loaded_from_storage / warm / slots_digest / flush / clear / edit_persist_count).
//!
//! T-159.19 adds `schedule_edit_persist` — the first **edit-driven** re-arm of the debounced writer,
//! called explicitly from the editor's `move_entities` commit (there is still no automatic core
//! change-hook/subscription; the mutator calls it). NOT ported: server hydrate/conflict GET, v1/v2
//! IDB migration, Save-Version POST. The whole module is `wasm32`-gated in `main.rs`.
//!
//! # T-221 — the records are per-account
//!
//! Every record in this store used to be keyed by mission id alone, so IndexedDB was a shared
//! drawer: on a machine two people use, B opening the mission A was last editing restored A's
//! unsaved draft, and a Save then posted A's work under B's account. The *physical* key is now
//! `u{len}:{owner}|{logical}` (see [`scoped_key`]) while every caller keeps passing the same logical
//! key it always did — `<id>` for the live doc, and T-191's `<id>::pre-adopt` / `<id>::pre-restore`
//! for the snapshot pair. The scoping therefore covers all three record kinds by construction, and
//! `mission_hydrate::clear_local_backups` still deletes exactly what it wrote.
//!
//! Three consequences worth naming, because each is a decision and not a detail:
//!   * **Pre-existing records are neither silently dropped nor silently adopted.** A record written
//!     before this change carries no owner, so nothing can attest whose it is — and handing it to
//!     whoever opens the mission next is the very bug. It stays on disk, is reported the moment it
//!     is passed over ([`note_orphan`]), is listed by `__missionPersist.orphans()`, and can be
//!     claimed on purpose with `__missionPersist.adopt_orphans()`. The one thing it must never do
//!     is read as "no backup", because that is the state in which someone reaches for one.
//!   * **A queued write cannot outlive the account that armed it.** The debounce is 5 s; sign-out is
//!     instant. [`PendingSave`] records the owner at arm time and [`run_save`] drops the write when
//!     it no longer matches, so A's bytes can never be filed under B.
//!   * **Records belonging to other accounts are evicted at editor boot**
//!     ([`evict_foreign_records`]). That is the backstop which also covers session expiry and a
//!     browser closed without signing out — neither of which any sign-out handler ever sees. The
//!     synchronous sign-out path calls [`purge_owner`] directly.
#![allow(clippy::cast_precision_loss)] // usize slot count → f64 for the JS bridge; tiny.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeSet, HashMap};
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

/* ─────────────────────── T-221 — per-account record scoping ─────────────────────── */

/// The owner token used when nobody is signed in.
///
/// Signed-out editing is a real state, not a defect: `/missions/:id/edit` carries no auth guard and
/// the whole editor smoke suite drives it logged out. Those records get their own namespace rather
/// than the bare legacy key, so after this change the store has exactly one key shape.
const ANON_OWNER: &str = "anon";

/// The account a record belongs to — the Discord id out of `localStorage["tbd-auth"]`.
///
/// **Why `discord_id`.** It is the identity the *server* keys on, and the bug is about server
/// identity: "a Save then posts it under B's account" is a statement about account ownership, so the
/// local record has to be partitioned by the same thing the Save is attributed to. It is also
/// immutable (a snowflake — usernames and avatars are not) and it survives a reload without the
/// reactive `AuthStore`, which matters because this module runs inside detached async tasks and
/// timer callbacks that hold no Leptos context. It is deliberately **not** a credential: the access
/// token rotates and the refresh token is single-use, so keying on either would re-namespace the
/// same user mid-session and hide their own draft from them.
///
/// Read as a loose `serde_json::Value` rather than through `auth::from_persist_json`. That parse is
/// strict over the whole `User` struct, so a single added or renamed backend field would make it
/// return `None`, silently demote a signed-in user to [`ANON_OWNER`], and lose them their work.
/// Exactly one string is needed here and exactly that string is allowed to fail.
fn current_owner() -> Option<String> {
    let storage = web_sys::window()?.local_storage().ok()??;
    let raw = storage.get_item(crate::auth::AUTH_PERSIST_KEY).ok()??;
    let blob: serde_json::Value = serde_json::from_str(&raw).ok()?;
    let id = blob
        .get("state")?
        .get("user")?
        .get("discord_id")?
        .as_str()?
        .trim();
    (!id.is_empty()).then(|| id.to_string())
}

/// [`current_owner`] with the signed-out fallback applied — the namespace a read or write uses now.
fn owner_token() -> String {
    current_owner().unwrap_or_else(|| ANON_OWNER.to_string())
}

/// The physical key prefix owning `owner`. Every key under it belongs to that account and to no
/// other, which is what makes [`purge_owner`] a prefix scan rather than a guess.
///
/// The length prefix is not decoration: it makes the mapping `(owner, logical) → key` **injective**
/// for arbitrary owner bytes. A plain `{owner}|{logical}` join collides the moment an id contains
/// the separator, and a `discord_id` is whatever the backend sends, not a shape this module gets to
/// assume. With the length in front, the owner segment is read by count and can hold anything.
fn owner_prefix(owner: &str) -> String {
    format!("u{}:{owner}|", owner.len())
}

/// The IndexedDB key for one caller-facing (logical) key under one account.
///
/// Callers never see this. `mission_editor` still asks for `<id>`, `mission_hydrate` still asks for
/// `<id>::pre-adopt` / `<id>::pre-restore`, and the account is applied here — which is precisely why
/// all three T-191 record kinds are scoped by this one change and none of them needed to move.
fn scoped_key(owner: &str, logical: &str) -> String {
    format!("{}{logical}", owner_prefix(owner))
}

/// Parse a physical key back into `(owner, logical)`, or `None` when it carries no owner at all —
/// i.e. when it is a record written before this change. That `None` is the *only* orphan test in
/// this module, so it has to be exact rather than a prefix guess: read `u`, the decimal length, `:`,
/// exactly that many bytes of owner, then `|`. `str::get` returns `None` on a non-boundary index, so
/// a multi-byte owner can never be sliced apart.
fn split_scoped(key: &str) -> Option<(&str, &str)> {
    let rest = key.strip_prefix('u')?;
    let (len_digits, rest) = rest.split_once(':')?;
    if len_digits.is_empty() || !len_digits.bytes().all(|b| b.is_ascii_digit()) {
        return None; // `+3` / `` / `0x2` are not the canonical form this module writes
    }
    let len: usize = len_digits.parse().ok()?;
    let owner = rest.get(..len)?;
    let logical = rest.get(len..)?.strip_prefix('|')?;
    Some((owner, logical))
}

thread_local! {
    /// Logical keys we have already complained about, so a polled `__missionBackup.has()` cannot
    /// turn one stranded record into a console flood. Warn once, stay warned.
    static WARNED_ORPHANS: RefCell<BTreeSet<String>> = const { RefCell::new(BTreeSet::new()) };
}

/// Report a pre-scoping record that a read just declined to return.
///
/// The alternative designs are both worse. Returning it re-creates the exact defect this ticket
/// closes — the record predates ownership, so "it is probably yours" is a guess made on behalf of
/// someone else's unsaved work. Deleting it destroys the one copy of a document that may exist
/// nowhere else. So it is kept, and it is made *loud*: the failure mode this warning exists to
/// prevent is a silent `None` reading as "no backup" to a user who is reaching for a backup.
fn note_orphan(logical: &str) {
    let first_time = WARNED_ORPHANS.with(|w| w.borrow_mut().insert(logical.to_string()));
    if !first_time {
        return;
    }
    web_sys::console::warn_1(&JsValue::from_str(&format!(
        "[yrs-persist] T-221: a record for \"{logical}\" predates per-account scoping, so it \
         carries no owner and was NOT restored — it may belong to another account on this machine. \
         It has NOT been deleted. List what is stranded with window.__missionPersist.orphans(); \
         claim it for the signed-in account with window.__missionPersist.adopt_orphans()."
    )));
}

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

/* ── raw access, by PHYSICAL key. Everything above these four applies the account scoping. ── */

/// `put(value, key)` at one physical key. Each helper runs its own transaction and never awaits
/// between issuing requests, so no request can be filed against a transaction the event loop has
/// already auto-committed.
async fn put_raw(key: &str, bytes: &[u8]) -> Result<(), idb::Error> {
    let db = open_db().await?;
    let tx = db.transaction(&[STORE], idb::TransactionMode::ReadWrite)?;
    let store = tx.object_store(STORE)?;
    let value = js_sys::Uint8Array::from(bytes);
    store
        .put(value.as_ref(), Some(&JsValue::from_str(key)))?
        .await?;
    tx.commit()?.await?;
    Ok(())
}

/// Read one physical key. Absent / unreadable / not a `Uint8Array` → `None`.
async fn get_raw(key: &str) -> Option<Vec<u8>> {
    let db = open_db().await.ok()?;
    let tx = db
        .transaction(&[STORE], idb::TransactionMode::ReadOnly)
        .ok()?;
    let store = tx.object_store(STORE).ok()?;
    let value: Option<JsValue> = store.get(JsValue::from_str(key)).ok()?.await.ok()?;
    let arr = value?.dyn_into::<js_sys::Uint8Array>().ok()?;
    Some(arr.to_vec())
}

/// Does a record exist at one physical key? `getKey` returns the key alone, so the orphan probe on
/// the [`load_state`] miss path costs a key lookup instead of a whole-document read.
async fn has_raw(key: &str) -> bool {
    let Ok(db) = open_db().await else {
        return false;
    };
    let Ok(tx) = db.transaction(&[STORE], idb::TransactionMode::ReadOnly) else {
        return false;
    };
    let Ok(store) = tx.object_store(STORE) else {
        return false;
    };
    match store.get_key(JsValue::from_str(key)) {
        Ok(req) => matches!(req.await, Ok(Some(_))),
        Err(_) => false,
    }
}

/// Delete one physical key.
async fn delete_raw(key: &str) -> Result<(), idb::Error> {
    let db = open_db().await?;
    let tx = db.transaction(&[STORE], idb::TransactionMode::ReadWrite)?;
    let store = tx.object_store(STORE)?;
    store.delete(JsValue::from_str(key))?.await?;
    tx.commit()?.await?;
    Ok(())
}

/// Every physical key in the store. The store holds a handful of records per account, so a full
/// key scan is the honest way to answer "what is on this machine" — and it is the only way to see
/// records whose owner is no longer known to this session.
async fn all_keys() -> Vec<String> {
    let Ok(db) = open_db().await else {
        return Vec::new();
    };
    let Ok(tx) = db.transaction(&[STORE], idb::TransactionMode::ReadOnly) else {
        return Vec::new();
    };
    let Ok(store) = tx.object_store(STORE) else {
        return Vec::new();
    };
    let Ok(req) = store.get_all_keys(None, None) else {
        return Vec::new();
    };
    req.await
        .unwrap_or_default()
        .iter()
        .filter_map(JsValue::as_string)
        .collect()
}

/* ── the account-scoped API. `id` is always a LOGICAL key; the owner is applied here. ── */

/// Persist the whole encode blob for `id` under an explicit account (React `saveState`). Stored as a
/// `Uint8Array` (structured clone), read back the same.
///
/// The owner is a parameter rather than a lookup so that a deferred write commits to the account it
/// was *armed* under or to nothing at all — see [`run_save`]. Resolving it here instead would leave
/// a window in which the sign-out landed between the check and the `put`.
async fn save_state_as(owner: &str, id: &str, bytes: &[u8]) -> Result<(), idb::Error> {
    put_raw(&scoped_key(owner, id), bytes).await
}

/// Persist the whole encode blob for `id` under the account signed in right now.
pub async fn save_state(id: &str, bytes: &[u8]) -> Result<(), idb::Error> {
    save_state_as(&owner_token(), id, bytes).await
}

/// Load the blob for `id` (React `loadState` → `value ?? null`). Any error / absence → `None`.
///
/// On a miss it probes for the pre-scoping record at the bare logical key. That record is **not**
/// returned — it carries no owner, so returning it is the cross-account restore this ticket exists
/// to stop — but its existence is reported ([`note_orphan`]) rather than collapsed into the same
/// silent `None` that a genuinely empty store produces. "There is nothing" and "there is something
/// I will not hand you" are different answers and a caller reaching for a backup deserves the second
/// one out loud.
pub async fn load_state(id: &str) -> Option<Vec<u8>> {
    let owner = owner_token();
    if let Some(bytes) = get_raw(&scoped_key(&owner, id)).await {
        return Some(bytes);
    }
    if has_raw(id).await {
        note_orphan(id);
    }
    None
}

/// Delete the blob for `id` (React `clearState`).
///
/// Scoped-key only, deliberately. This is reached from `mission_hydrate::clear_local_backups` on a
/// successful Save — an act that speaks for one account — and a pre-scoping record at the bare key
/// may be another account's only copy. Those are dropped by an explicit
/// `__missionPersist.adopt_orphans()`, never as a side effect of somebody else's Save.
pub async fn clear_state(id: &str) -> Result<(), idb::Error> {
    delete_raw(&scoped_key(&owner_token(), id)).await
}

/// Delete every record owned by `owner`, returning how many went.
///
/// Public because sign-out has to be able to call it: clearing the session leaves the drafts behind,
/// and on a shared machine "signed out" has to mean the work is gone from the disk too, not merely
/// namespaced away from the next person. Used here by [`evict_foreign_records`].
pub async fn purge_owner(owner: &str) -> usize {
    let prefix = owner_prefix(owner);
    let doomed: Vec<String> = all_keys()
        .await
        .into_iter()
        .filter(|k| k.starts_with(&prefix))
        .collect();
    let mut gone = 0;
    for key in doomed {
        if delete_raw(&key).await.is_ok() {
            gone += 1;
        }
    }
    gone
}

/// Drop the records of every account that is not the one signed in now.
///
/// Run once per editor boot. A sign-out hook alone cannot carry the guarantee — a session that
/// expires, a revoked token, or a browser closed with the tab open all leave records behind that no
/// handler ever runs for — so the rule is enforced where it can actually be checked: at the next
/// boot, the only account with records on this machine is the account using it.
///
/// **No-op while signed out**, and that is load-bearing in two directions. A signed-out visitor must
/// not be able to delete a signed-in user's drafts, and the editor smokes run logged out against
/// the same store. Unowned pre-scoping records are also left alone: this evicts records it can
/// attribute to someone else, and an orphan is by definition one it cannot attribute at all.
async fn evict_foreign_records() {
    let Some(me) = current_owner() else {
        return;
    };
    let strangers: BTreeSet<String> = all_keys()
        .await
        .iter()
        .filter_map(|k| split_scoped(k).map(|(owner, _)| owner.to_string()))
        .filter(|owner| *owner != me)
        .collect();
    if strangers.is_empty() {
        return;
    }
    let mut gone = 0;
    for owner in strangers {
        gone += purge_owner(&owner).await;
    }
    web_sys::console::log_1(&JsValue::from_str(&format!(
        "[yrs-persist] T-221: evicted {gone} local record(s) belonging to another account"
    )));
}

/// The logical keys of every pre-scoping record still on this machine — what
/// `__missionPersist.orphans()` reports.
async fn orphan_keys() -> Vec<String> {
    all_keys()
        .await
        .into_iter()
        .filter(|k| split_scoped(k).is_none())
        .collect()
}

/// Claim every pre-scoping record for the signed-in account. Returns `(adopted, skipped)`.
///
/// Deliberately manual. Nothing in the data can prove who wrote these records, so the act of
/// claiming them is a human saying "this machine is mine" — the one authority that actually exists
/// here. A logical key that already has a record under this account is **skipped**, never
/// overwritten: adoption is for recovering a stranded document, not for letting an older one
/// clobber the work someone is doing now. Adopted records are removed from the unowned key so the
/// claim happens exactly once.
async fn adopt_orphans(owner: &str) -> (usize, usize) {
    let (mut adopted, mut skipped) = (0, 0);
    for logical in orphan_keys().await {
        let scoped = scoped_key(owner, &logical);
        if has_raw(&scoped).await {
            skipped += 1;
            continue;
        }
        let Some(bytes) = get_raw(&logical).await else {
            skipped += 1;
            continue;
        };
        if put_raw(&scoped, &bytes).await.is_ok() {
            let _ = delete_raw(&logical).await;
            WARNED_ORPHANS.with(|w| {
                w.borrow_mut().remove(&logical);
            });
            adopted += 1;
        } else {
            skipped += 1;
        }
    }
    (adopted, skipped)
}

/* ─────────────────────── debounced + serialized writer ─────────────────────── */

type GetBytes = Box<dyn Fn() -> Vec<u8>>;
type IsCancelled = Box<dyn Fn() -> bool>;

struct PendingSave {
    get_bytes: GetBytes,
    is_cancelled: IsCancelled,
    /// T-221 — the account signed in when this write was armed. A debounce is 5 s wide and a
    /// sign-out is instant, so without this the timer could fire after the handover and file one
    /// user's document under the next one's name: the ticket's defect, reintroduced through the
    /// back door. Checked in [`run_save`].
    owner: String,
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
    // T-159.19 — how many times a mutator re-armed the debounce via `schedule_edit_persist`. Starts
    // 0 at boot (the boot persist calls `save_state_debounced` directly, NOT this), so the
    // `__missionPersist.edit_persist_count()` gate proves the FIRST edit-driven write is scheduled
    // (a late `flush()` of the boot debounce would encode the moved doc anyway — the counter, not
    // the blob, is the sound signal that the edit itself re-armed the writer).
    static EDIT_PERSIST_COUNT: Cell<u32> = const { Cell::new(0) };
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

/// Serialized write: take the per-mission lock, then apply the guards in order — cancel check
/// **before** reading bytes, T-221 owner check, empty-blob skip, then persist.
///
/// A changed owner **drops** the write; it does not redirect it. The bytes were composed by the
/// previous session, so writing them anywhere the new account can read is the cross-account leak
/// this ticket closes, and writing them back to the old account after a sign-out contradicts the
/// other half of it. Nothing is lost that matters: the document is still in memory, and the next
/// edit re-arms the writer under whoever is now signed in.
async fn run_save(id: &str, pending: PendingSave) {
    let lock = lock_for(id);
    let _guard = lock.lock().await;
    if (pending.is_cancelled)() {
        return;
    }
    if owner_token() != pending.owner {
        return;
    }
    let bytes = (pending.get_bytes)();
    if bytes.is_empty() {
        return; // never overwrite a good record with an empty/truncated blob
    }
    if let Err(e) = save_state_as(&pending.owner, id, &bytes).await {
        web_sys::console::warn_1(&JsValue::from_str(&format!(
            "[yrs-persist] save failed: {e:?}"
        )));
    }
}

/// Debounced save (React `saveStateDebounced`). Stores the pending save, resets the timer (a burst
/// coalesces to one write), and on fire reads bytes at write time. `get_bytes`/`is_cancelled` are
/// evaluated inside `run_save`, so they must not hold any `RefCell` borrow across an `.await`
/// (callers pass closures that borrow transiently and return owned data).
pub fn save_state_debounced(
    id: &str,
    get_bytes: GetBytes,
    is_cancelled: IsCancelled,
    delay_ms: i32,
) {
    let id_owned = id.to_string();
    PENDING.with(|p| {
        p.borrow_mut().insert(
            id_owned.clone(),
            PendingSave {
                get_bytes,
                is_cancelled,
                owner: owner_token(),
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
    js_sys::Promise::new(
        &mut move |resolve: js_sys::Function, _reject: js_sys::Function| {
            if let Some(f) = fut.take() {
                spawn_local(async move {
                    f.await;
                    let _ = resolve.call0(&JsValue::NULL);
                });
            }
        },
    )
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
    let get = |dict: &[String], idx: u32| {
        dict.get(idx as usize)
            .map_or("", String::as_str)
            .to_string()
    };
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

/// T-159.19 — schedule an **edit-driven** persist after a mutator (the first real doc change; the
/// S8 hook .17/.18 deferred). Re-arms the SAME debounced + serialized writer the boot seam uses
/// (`mission_editor.rs` initial persist): `get_bytes` reads `encode_state()` at write time, the
/// write is cancelled once the doc `Option` clears (route leave). A burst of edits within
/// [`debounce_ms`] coalesces into one IDB write. Bumps [`EDIT_PERSIST_COUNT`] for the gate.
pub fn schedule_edit_persist(doc: DocHandle, id: &str) {
    EDIT_PERSIST_COUNT.with(|c| c.set(c.get().saturating_add(1)));
    let get = doc.clone();
    let cancel = doc;
    save_state_debounced(
        id,
        Box::new(move || {
            get.borrow()
                .as_ref()
                .map(MissionDocCore::encode_state)
                .unwrap_or_default()
        }),
        Box::new(move || cancel.borrow().is_none()),
        debounce_ms(),
    );
}

/// The number of edit-driven persists scheduled this page lifetime (T-159.19). Exposed on the
/// `__missionPersist` bridge so the gate can prove a move re-armed the writer.
#[must_use]
pub fn edit_persist_count() -> u32 {
    EDIT_PERSIST_COUNT.with(Cell::get)
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
            match crate::editor_session::read_warm(&id).and_then(|s| serde_json::to_string(&s).ok())
            {
                Some(json) => JsValue::from_str(&json),
                None => JsValue::NULL,
            }
        }) as Box<dyn FnMut() -> JsValue>)
    };
    let digest_fn = {
        let doc = doc.clone();
        Closure::wrap(Box::new(move || -> JsValue {
            let digest = doc.borrow().as_ref().map(slots_digest).unwrap_or_default();
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
    let edit_count_fn = Closure::wrap(Box::new(move || -> JsValue {
        JsValue::from_f64(f64::from(edit_persist_count()))
    }) as Box<dyn FnMut() -> JsValue>);
    // T-221 — the recovery surface for records written before per-account scoping. `orphans()`
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
    let _ = js_sys::Reflect::set(&obj, &JsValue::from_str("orphans"), orphans_fn.as_ref());
    let _ = js_sys::Reflect::set(&obj, &JsValue::from_str("adopt_orphans"), adopt_fn.as_ref());
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
    orphans_fn.forget();
    adopt_fn.forget();

    // T-221 — one eviction sweep per editor boot. Spawned rather than awaited so the bridge stays
    // synchronously installed for the gate, and safe against the boot restore racing it: it only
    // ever deletes keys of a *different* owner, and the restore only ever reads this one's.
    spawn_local(async move { evict_foreign_records().await });
}

/// The debounce default, exposed so the boot seam arms the initial persist with the contract delay.
#[must_use]
pub const fn debounce_ms() -> i32 {
    DEBOUNCE_MS
}
