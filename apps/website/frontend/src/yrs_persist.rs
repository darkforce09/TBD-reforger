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
//!      a **content** check that never clobbers a good record (T-374 — see below; this was a byte
//!      check that did not implement the promise), one write at a time per mission.
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
//!     browser closed without signing out — neither of which any sign-out handler ever sees.
//!
//! # T-338 — sign-out is finally wired to the purge this module documented
//!
//! T-221 shipped [`purge_owner`] `pub`, tested and ready, and this header said "the synchronous
//! sign-out path calls [`purge_owner`] directly". Nothing did. For two waves
//! [`evict_foreign_records`] was its only caller repo-wide, so signing out namespaced the departing
//! account's drafts away from the next person without deleting them — on a shared machine the
//! documents sat on the disk until some later editor boot happened to notice, and "signed out" did
//! not mean the work was gone.
//!
//! `auth::clear_session` now calls it, by way of `mission_hydrate::purge_local_documents`, which
//! drops the RAM half of the same records in the same breath (see that module's T-338 section — the
//! in-memory snapshot cache in front of this store was the other half of the leak).
//!
//! **The owner is captured before the session is cleared, and that ordering is the fix, not a
//! detail.** [`current_owner`] reads `localStorage["tbd-auth"]`, and sign-out clears both the signals
//! and that blob; resolve the token afterwards and it is [`ANON_OWNER`], so the purge would delete a
//! signed-out visitor's drafts and leave the departing account's exactly where they were — the
//! inverse of the intent, and silent. `clear_session` therefore reads `discord_id` out of the session
//! signal first and passes it in.
//!
//! The boot-time eviction stays exactly as it was and is still the load-bearing backstop: no
//! sign-out handler runs for a session that expires, a token that is revoked, or a browser that is
//! closed with the tab open.
//!
//! # T-374 — the "never clobber a good record" guard now tests content, not bytes
//!
//! Item 2 above claimed an "empty-blob skip (never clobber a good record)" since .17, and [`run_save`]
//! implemented it as `bytes.is_empty()`. That is a **byte** test standing in for a **content** test,
//! and the substitution does not hold: [`MissionDocCore::encode_state`] is
//! `encode_state_as_update_v1(&StateVector::default())`, which writes a var-int client count and then
//! the delete set, so **an empty document encodes to `[0, 0]` — two bytes, non-empty** — and sailed
//! past the guard onto the record. Measured, not reasoned: see the T-374 verify output. The only
//! input the byte test ever rejected was `Vec::new()`, which `get_bytes` produces solely when the doc
//! `Option` is `None`, and `is_cancelled` already catches that. The guard was decorative.
//!
//! [`MissionDocCore::has_content`] — the predicate that defines what content *means* here (faction /
//! slot / objective / vehicle / marker) — existed the whole time with exactly one call site,
//! `mission_hydrate::classify_local`, and none on any write path. [`blob_has_content`] now consults it
//! on every write, by replaying the blob into a throwaway core the same way the boot seam replays a
//! restore. That also closes two losses no length test can see: a **content-empty but byte-fat** blob
//! (a core with only `meta` seeded is ~124 bytes and content-empty), and a **corrupt or truncated**
//! blob, which the old guard wrote cheerfully over a good record even though it can never be replayed.
//!
//! Two further things this section owes the next reader:
//!   * **The decode is O(document), so the per-edit writer does not do it.** `schedule_edit_persist`
//!     holds the live `DocHandle` and passes a [`ContentProbe`] that answers in O(1); the boot seam's
//!     `save_state_debounced` has only the two closures its caller passes and takes the decode, once
//!     per boot. Sound because probe and encode are sampled with no `.await` between them.
//!   * **A content guard does NOT close T-380.** T-380 is the same loss with a different trigger: an
//!     edit during boot arms the debounce, and if the restore has not yet swapped the core, the timer
//!     persists the 8-slot fixture seed over the good record. The seed is not empty — it has 8 real
//!     slots — so `has_content()` is true for it and this guard passes it, exactly as it must. What
//!     that needs is a *document-identity* guard at the swap, mirroring the T-221 owner check one
//!     level down; see the T-374 report.
//!
//! **One behaviour change this buys, stated plainly rather than discovered later.** Emptying a
//! document *deliberately* — select-all, Delete — now also fails the guard, because at the blob level
//! a document someone emptied on purpose and a document that was never populated are the same
//! content-empty document, and `has_content()` is the predicate that draws that line. So a full
//! delete no longer propagates to the local autosave record; a reload restores the pre-delete
//! document. That is the correct side to err on and it is the side the codebase already picked:
//! `mission_hydrate::classify_local` maps a content-empty doc to `Local::Empty`, "there is no local
//! work and no choice to offer". The delete is still an undo step, the record is still cleared by a
//! Save (`clear_local_backups`) and by `clear_state`, and the failure being traded away is an
//! authored mission overwritten with nothing. If the distinction is ever wanted, it is available: a
//! never-populated document has no delete set, an emptied one does — that is a delete-set parse, not
//! a length check, and it is deliberately not in this slice.
//!
//! Same section, second defect: [`get_raw`] collapsed "no record" and "the read failed" into one
//! `None`, and [`load_state`] reports that to the boot as "no local content" — a false negative that
//! drives the cold path. Reads are now three-valued ([`RecordRead`]), a failure is reported, and
//! [`run_save`] refuses to write over a key that is present but was unreadable this page lifetime.
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
///
/// **T-338 — `pub`, because a second cache had to agree with this one.** `mission_hydrate` keeps an
/// in-memory copy of each T-191 snapshot in front of these records, and it was not keyed by account:
/// its lookup hit before the scoped IDB read, so within one page load a change of account did not
/// hide the previous account's document. There is exactly one right token for that cache to key on —
/// this one — because any other choice makes "is a backup on record?" and "can a restore read it?"
/// two different questions, and the whole hazard is a `has()` that answers for a document the reader
/// cannot legitimately have.
#[must_use]
pub fn owner_token() -> String {
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
    /// T-374 — **physical** keys whose read failed this page lifetime (see [`RecordRead::Failed`]).
    /// Consulted by [`run_save`], which refuses to write over a record that is present but was not
    /// readable. Physical, not logical: it is the exact key a write would land on.
    static UNREADABLE: RefCell<BTreeSet<String>> = const { RefCell::new(BTreeSet::new()) };
    /// F-20 — mission ids we have already noted an empty-content refusal for, so a brand-new mission
    /// (whose every debounced tick refuses until the first authored content lands) cannot flood the
    /// console with the same T-374 refusal 8× over. The `BLOCKED_EMPTY` counter above still tallies
    /// EVERY refusal (the bridge reads it); this only gates the console message. Same "note once, stay
    /// noted" shape as [`WARNED_ORPHANS`].
    static WARNED_EMPTY: RefCell<BTreeSet<String>> = const { RefCell::new(BTreeSet::new()) };
    /// T-374 — how many writes the guards refused, by reason. Exposed on the bridge
    /// (`__missionPersist.blocked_writes()`) so a probe can prove the guard **fired**, rather than
    /// inferring it from a record that merely happens to be unchanged.
    static BLOCKED_EMPTY: Cell<u32> = const { Cell::new(0) };
    static BLOCKED_UNREADABLE: Cell<u32> = const { Cell::new(0) };
}

/// Remember that a read of `physical_key` failed. See [`RecordRead::Failed`] and [`run_save`].
fn note_unreadable(physical_key: &str) {
    UNREADABLE.with(|u| {
        u.borrow_mut().insert(physical_key.to_string());
    });
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

/// T-374 — the outcome of reading one physical key, with **`Miss` and `Failed` kept apart**.
///
/// [`get_raw`] used to return `Option<Vec<u8>>` and collapse both into `None`, so a failed
/// `open_db` / transaction / `get` read out as "there is no record here". That is a **false
/// negative on the one question the boot seam asks**: [`load_state`] reported "no local content",
/// the editor took the cold path, and the seed then went down the write path at a record that may
/// well have been someone's only copy. Two different facts deserve two different answers — the
/// same argument [`load_state`]'s own doc comment already makes for orphans.
enum RecordRead {
    /// The record is there and decoded to bytes.
    Hit(Vec<u8>),
    /// The store answered, and there is nothing at this key.
    Miss,
    /// IndexedDB could not be asked, or answered with something unreadable. **Not** a miss:
    /// nothing here licenses the claim that the key is empty.
    Failed,
}

/// Read one physical key, three-valued (T-374). A stored value that is not a `Uint8Array` counts as
/// [`RecordRead::Failed`], not `Miss` — a record exists, this code just cannot read it, which is
/// exactly the state that must not masquerade as "nothing is stored".
async fn read_raw(key: &str) -> RecordRead {
    let Ok(db) = open_db().await else {
        return RecordRead::Failed;
    };
    let Ok(tx) = db.transaction(&[STORE], idb::TransactionMode::ReadOnly) else {
        return RecordRead::Failed;
    };
    let Ok(store) = tx.object_store(STORE) else {
        return RecordRead::Failed;
    };
    let Ok(req) = store.get(JsValue::from_str(key)) else {
        return RecordRead::Failed;
    };
    match req.await {
        Ok(Some(value)) => match value.dyn_into::<js_sys::Uint8Array>() {
            Ok(arr) => RecordRead::Hit(arr.to_vec()),
            Err(_) => RecordRead::Failed,
        },
        Ok(None) => RecordRead::Miss,
        Err(_) => RecordRead::Failed,
    }
}

/// Read one physical key. Absent / unreadable / not a `Uint8Array` → `None`.
///
/// The two-valued view of [`read_raw`], for the callers that genuinely have nothing different to do
/// on a failure ([`adopt_orphans`] skips either way). [`load_state`] uses [`read_raw`] directly.
async fn get_raw(key: &str) -> Option<Vec<u8>> {
    match read_raw(key).await {
        RecordRead::Hit(bytes) => Some(bytes),
        RecordRead::Miss | RecordRead::Failed => None,
    }
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
///
/// # T-374 — a failed read is not an empty store
///
/// The same argument applies one level down. A record that could not be *read* used to come back as
/// the identical `None` as a store with nothing in it, and the boot seam turns that `None` into "no
/// local content" and keeps the seed. The failure is now reported and remembered
/// ([`note_unreadable`]) so that [`run_save`] will not let the seed overwrite a record this page
/// lifetime failed to read — see that function's third guard. The return type is unchanged (`None`
/// is still the honest answer: there are no bytes to hand back), so no caller has to change; what
/// changed is that the write path now knows the difference.
pub async fn load_state(id: &str) -> Option<Vec<u8>> {
    let owner = owner_token();
    let scoped = scoped_key(&owner, id);
    match read_raw(&scoped).await {
        RecordRead::Hit(bytes) => return Some(bytes),
        RecordRead::Failed => {
            // Do NOT probe for an orphan here: the probe is a *different* key, and reporting
            // "a pre-scoping record exists" when the real story is "this account's own record is
            // unreadable" would point recovery at the wrong drawer.
            note_unreadable(&scoped);
            web_sys::console::warn_1(&JsValue::from_str(&format!(
                "[yrs-persist] T-374: IndexedDB read FAILED for {id} — this is NOT 'no local \
                 backup'. Treating local content as unknown; writes to this record are blocked \
                 while a record is present but unreadable."
            )));
            return None;
        }
        RecordRead::Miss => {}
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
/// namespaced away from the next person.
///
/// Two callers, and they cover the two ways an account stops using this machine: [`evict_foreign_records`]
/// at the next editor boot (session expiry, revoked token, browser closed), and — since T-338 —
/// `mission_hydrate::purge_local_documents` from `auth::clear_session` on a deliberate sign-out.
///
/// One prefix scan covers all three record kinds by construction: the live doc `<id>` and both T-191
/// snapshot slots (`<id>::pre-adopt` / `<id>::pre-restore`) are logical keys under the same owner
/// prefix, so nothing here has to know they exist.
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

/* ───────────── T-374 — the content test the write path was missing ───────────── */

/// Read the leading unsigned var-int of a Yjs v1 update stream: the **number of client blocks**.
///
/// `encode_state_as_update_v1` writes `varint(num_clients)`, then that many per-client struct
/// blocks, then the delete set. `num_clients == 0` therefore means the stream carries no structs at
/// all — a document with literally nothing in it, not even `meta`. Returns `None` when the leading
/// var-int is malformed (an unterminated continuation run), which is itself grounds to refuse.
///
/// This exists as an O(1) tier in front of [`blob_has_content`] so that the *reported* T-374 blob —
/// the two bytes `[0, 0]` — is rejected without decoding anything at all.
fn update_client_count(bytes: &[u8]) -> Option<u64> {
    let mut n: u64 = 0;
    let mut shift = 0u32;
    for (i, b) in bytes.iter().enumerate() {
        // A u64 var-int is at most 10 bytes; past that the stream is not a v1 update header.
        if i >= 10 {
            return None;
        }
        n |= u64::from(b & 0x7f) << shift;
        if b & 0x80 == 0 {
            return Some(n);
        }
        shift += 7;
    }
    None
}

/// Would this blob restore to a document that holds authored content?
///
/// # Why the byte test this replaces was not a test at all
///
/// The guard in [`run_save`] was `bytes.is_empty()`, with the comment "never overwrite a good record
/// with an empty/truncated blob", and the module header above promised an "empty-blob skip". A
/// **byte** test cannot keep a **content** promise. [`MissionDocCore::encode_state`] is
/// `encode_state_as_update_v1(&StateVector::default())`, which writes a var-int client count and
/// then the delete set — so an empty document encodes to `[0, 0]`: **two bytes, non-empty, and the
/// old guard waved it through onto the record**. `get_bytes` only ever yields `Vec::new()` when the
/// doc `Option` is `None`, and `is_cancelled` already catches exactly that, so the byte test was
/// dead for its stated purpose and live only as reassurance.
///
/// # Why this decodes rather than inspecting bytes
///
/// The question that matters is "would restoring these bytes produce a document with content", and
/// the sound way to answer it is to *do the restore* — the identical `MissionDocCore::new()` +
/// `apply_update` the boot seam runs (`mission_editor.rs` step 1) — and then ask
/// [`MissionDocCore::has_content`], the predicate that already encodes what "content" means
/// (faction / slot / objective / vehicle / marker) and that until now had exactly one call site
/// (`mission_hydrate::classify_local`), none of them on a write path.
///
/// Two classes of loss this closes that no byte-level test can see:
///   * **content-empty but byte-fat.** A core with only `meta` seeded encodes to ~124 bytes and
///     `has_content()` is false. Any threshold on length is a guess; this is not.
///   * **corrupt / truncated.** A blob that fails `apply_update` is unrestorable, and the old guard
///     wrote it cheerfully over a good record. A blob that cannot be replayed is not a backup.
///
/// The decode is O(document) and runs on the write path, so the hot caller avoids it: see
/// [`PendingSave::content_probe`], which lets a caller holding the live core answer in O(1).
///
/// `pub` because the same byte test is wrong in three more places that this slice does not own, and
/// none of them should have to copy this logic or reach in and re-export it: `mission_hydrate`'s
/// `snapshot_local` (banks a content-empty doc as a "backup"), `has_snapshot` (reports `true` for
/// one), and `restore_snapshot` (would restore one over the live document). This is not the
/// uncalled-`pub` the T-338 note above scolds — [`run_save`] and the `__missionPersist` bridge both
/// call it in this file today. `snapshot_local` holds the live core and should prefer
/// `has_content()` directly (O(1)); the other two only have bytes, and this is their test.
#[must_use]
pub fn blob_has_content(bytes: &[u8]) -> bool {
    if bytes.is_empty() {
        return false;
    }
    match update_client_count(bytes) {
        // No client blocks ⇒ no structs ⇒ nothing in the document. The reported `[0, 0]` blob.
        Some(0) | None => return false,
        Some(_) => {}
    }
    let probe = MissionDocCore::new();
    // INIT, for the reason every other replay in this codebase uses it: a LOCAL apply pushes an undo
    // step and yrs keeps deleted blocks alive for as long as the stack item lives. This core is
    // dropped at the end of the function and should cost one document, not two.
    probe.set_origin_init(true);
    if probe.apply_update(bytes).is_err() {
        return false;
    }
    probe.set_origin_init(false);
    probe.has_content()
}

/* ─────────────────────── debounced + serialized writer ─────────────────────── */

type GetBytes = Box<dyn Fn() -> Vec<u8>>;
type IsCancelled = Box<dyn Fn() -> bool>;
/// T-374 — an O(1) "does the doc these bytes came from hold content" answer, for callers that hold
/// the live [`DocHandle`]. See [`PendingSave::content_probe`].
type ContentProbe = Box<dyn Fn() -> bool>;

struct PendingSave {
    get_bytes: GetBytes,
    is_cancelled: IsCancelled,
    /// T-221 — the account signed in when this write was armed. A debounce is 5 s wide and a
    /// sign-out is instant, so without this the timer could fire after the handover and file one
    /// user's document under the next one's name: the ticket's defect, reintroduced through the
    /// back door. Checked in [`run_save`].
    owner: String,
    /// T-374 — the cheap content test, when the caller can supply one.
    ///
    /// `Some` means the caller holds the live [`DocHandle`] the bytes are encoded from, so
    /// [`run_save`] can call [`MissionDocCore::has_content`] on it directly — O(1) — instead of
    /// decoding the blob. **This is sound only because it is sampled in the same synchronous window
    /// as `get_bytes`, with no `.await` between**: wasm is single-threaded, so nothing can mutate
    /// the document between the encode and the probe, and the two therefore describe one state.
    ///
    /// `None` → [`blob_has_content`] decodes the blob. That is the correct fallback and not a
    /// degraded one: it tests the bytes themselves, so it also catches a corrupt blob, which a live
    /// probe by construction cannot. It costs one document decode, which is why the per-edit writer
    /// supplies a probe and the once-per-boot writer does not have to.
    content_probe: Option<ContentProbe>,
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
/// **before** reading bytes, T-221 owner check, T-374 **content** check, T-374 unreadable-record
/// check, then persist.
///
/// A changed owner **drops** the write; it does not redirect it. The bytes were composed by the
/// previous session, so writing them anywhere the new account can read is the cross-account leak
/// this ticket closes, and writing them back to the old account after a sign-out contradicts the
/// other half of it. Nothing is lost that matters: the document is still in memory, and the next
/// edit re-arms the writer under whoever is now signed in.
///
/// # T-374 — the guard order, and why the content test comes before the IO
///
/// The content test is pure CPU; the unreadable-record test costs an IndexedDB key lookup. Refusing
/// a content-empty blob first means the common rejection never touches the disk. Both are stated as
/// the same rule the T-221 owner check states: **a write that cannot be shown to be safe does not
/// happen.** In every refusal the document is still in RAM and the next edit re-arms the writer, so
/// the cost of a false refusal is bounded by one debounce window; the cost of a false *acceptance*
/// is an authored mission.
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
    // T-374 — a CONTENT test, not a byte test. An empty document encodes to `[0, 0]`: two bytes,
    // non-empty, and the old `bytes.is_empty()` guard wrote it straight over a good record. `bytes`
    // and the probe are read with no `.await` between them, so they describe one document state.
    let has_content = match &pending.content_probe {
        Some(probe) => !bytes.is_empty() && probe(),
        None => blob_has_content(&bytes),
    };
    if !has_content {
        BLOCKED_EMPTY.with(|c| c.set(c.get().saturating_add(1)));
        // F-20 — a brand-new mission refuses on every debounced tick until the operator authors the
        // first content, so warning here fired ~8× on an empty mission. Note once per id and drop it
        // to `debug`: the refusal is expected on an empty doc (not a warning-worthy event), and the
        // `BLOCKED_EMPTY` counter above already carries the real tally for the bridge/telemetry.
        let first_time = WARNED_EMPTY.with(|w| w.borrow_mut().insert(id.to_string()));
        if first_time {
            web_sys::console::debug_1(&JsValue::from_str(&format!(
                "[yrs-persist] T-374: not persisting {id} yet — {} byte(s) that restore to a \
                 document with no authored content (expected on an empty mission until the first \
                 edit lands). The record on disk is untouched; the live document is unaffected.",
                bytes.len()
            )));
        }
        return;
    }
    // T-374 — and never write over a record this page lifetime FAILED to read. `load_state`
    // returning `None` used to mean both "nothing is stored" and "the read broke"; on the second,
    // the boot keeps the seed, and letting the seed land here is how an unreadable-but-present
    // record becomes a destroyed one. Re-probe rather than latching: if the key is genuinely absent
    // there is nothing to protect, so the flag clears and the write proceeds — otherwise a single
    // transient failure would silently disable persistence for the rest of the session, trading one
    // silent loss for another.
    let key = scoped_key(&pending.owner, id);
    let still_blocked = UNREADABLE.with(|u| u.borrow().contains(&key));
    if still_blocked {
        if has_raw(&key).await {
            BLOCKED_UNREADABLE.with(|c| c.set(c.get().saturating_add(1)));
            web_sys::console::warn_1(&JsValue::from_str(&format!(
                "[yrs-persist] T-374: refused to persist {id} — a record exists at this key but \
                 this session could not read it, so overwriting it could destroy the only copy. \
                 Reload to retry the read."
            )));
            return;
        }
        UNREADABLE.with(|u| {
            u.borrow_mut().remove(&key);
        });
    }
    if let Err(e) = save_state_as(&pending.owner, id, &bytes).await {
        web_sys::console::warn_1(&JsValue::from_str(&format!(
            "[yrs-persist] save failed: {e:?}"
        )));
        return;
    }
    // T-804 — a flush COMPLETED. Recorded here, in the one branch where the bytes actually reached
    // IndexedDB, and nowhere earlier: the T-779 ack discipline is that this timestamp means "the
    // draft is on disk", not "a write was scheduled". Every refusal above (`is_cancelled`, the
    // T-221 owner check, the T-374 content and unreadable guards) returns before this line, and the
    // IO-error branch just above returns too, so the only way to reach it is a real write. That
    // guarantee is exactly what makes the strip's "draft saved Ns ago" chip honest — see
    // [`note_flush_completed`].
    note_flush_completed();
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
    // No content probe: this entry point takes only the two closures its callers already pass, so
    // `run_save` falls back to decoding the blob. Correct, and the stronger of the two tests (it
    // catches a corrupt blob) — just O(document). The boot seam arms this once per boot.
    arm_debounced(id, get_bytes, is_cancelled, None, delay_ms);
}

/// The body of [`save_state_debounced`], plus T-374's optional [`ContentProbe`]. Private: the two
/// callers are [`save_state_debounced`] (no probe) and [`schedule_edit_persist`] (probe), and a
/// third public arming surface with no caller is the mistake this module's T-338 note already
/// records once — `purge_owner` shipped `pub` and documented as wired, with nothing calling it, for
/// two waves.
fn arm_debounced(
    id: &str,
    get_bytes: GetBytes,
    is_cancelled: IsCancelled,
    content_probe: Option<ContentProbe>,
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
                content_probe,
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
/// T-374 — this path supplies a [`ContentProbe`], so the content guard on the **per-edit** writer is
/// O(1) (`has_content()` on the live core) instead of an O(document) decode of the blob. It holds the
/// `DocHandle` the bytes are encoded from, so it can; the boot seam's entry point cannot, and pays
/// the decode once per boot. Measured on native: the decode is ~19 µs at 8 slots but ~460 ms at 100k,
/// which is a visible stall to hand a writer that re-arms on every edit.
pub fn schedule_edit_persist(doc: DocHandle, id: &str) {
    EDIT_PERSIST_COUNT.with(|c| c.set(c.get().saturating_add(1)));
    let get = doc.clone();
    let probe = doc.clone();
    let cancel = doc;
    arm_debounced(
        id,
        Box::new(move || {
            get.borrow()
                .as_ref()
                .map(MissionDocCore::encode_state)
                .unwrap_or_default()
        }),
        Box::new(move || cancel.borrow().is_none()),
        Some(Box::new(move || {
            probe
                .borrow()
                .as_ref()
                .is_some_and(MissionDocCore::has_content)
        })),
        debounce_ms(),
    );
}

/// The number of edit-driven persists scheduled this page lifetime (T-159.19). Exposed on the
/// `__missionPersist` bridge so the gate can prove a move re-armed the writer.
#[must_use]
pub fn edit_persist_count() -> u32 {
    EDIT_PERSIST_COUNT.with(Cell::get)
}

/* ───────────────────────────── T-804 — the last-flush timestamp ───────────────────────────── */

thread_local! {
    // T-804 — the epoch-ms instant of the last COMPLETED flush (see `note_flush_completed`), and the
    // reactive signal the top strip's "draft saved Ns ago" chip subscribes to. ONE exposed signal,
    // parked here in the `context_menu::MENU` / `eden_settings::PREFS_OPEN` idiom: the strip creates
    // it (it has a reactive owner during render; this module runs in detached timers/tasks that do
    // not) and hands it over via `set_last_flush_signal`, and this module only ever `.set()`s it.
    //
    // The `Cell` behind it is the source of truth for the ack, always writable with no owner, so a
    // flush that completes before the strip has installed its signal is not lost: `last_flush_ms`
    // reads the `Cell`, and the strip seeds the signal from it on install. `None` ⇒ no flush has
    // completed this page lifetime — which is precisely "a mission that has never been edited",
    // because a never-edited doc is content-empty and the T-374 guard refuses to write it, so no
    // flush can complete and no chip is shown. This is NOT a second dirtiness source: the flush only
    // happens because an edit armed the writer, so the ack is strictly downstream of the same edit
    // that arms the dirty dot (the wave-129 one-source rule).
    static LAST_FLUSH_MS: Cell<Option<f64>> = const { Cell::new(None) };
    static LAST_FLUSH_SIG: RefCell<Option<leptos::prelude::RwSignal<Option<f64>>>> =
        const { RefCell::new(None) };
}

/// T-804 — a flush COMPLETED (called only from [`run_save`]'s success branch). Records the instant in
/// the `Cell` (the always-available ack) and, if the strip has installed its signal, pushes the new
/// value so the chip re-renders. Two writes of one value: the `Cell` is the truth a late-installed
/// signal seeds from; the signal is the reactive mirror.
///
/// The instant is `Date.now()` — wall-clock epoch ms. The chip renders a *recency* (now − last
/// flush) with both ends read from that same clock, so a monotonic source buys nothing and the wall
/// clock is the one the browser hands back cheaply.
#[cfg(target_arch = "wasm32")]
fn note_flush_completed() {
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
fn note_flush_completed() {}

/// T-804 — install the strip's last-flush signal (once, from `TopCommandStrip` setup, which has the
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

/// T-804 — the epoch-ms instant of the last completed flush, or `None` if none has completed this
/// page lifetime. Reads the `Cell` (not the signal) so it is correct before the strip mounts and
/// needs no reactive owner; exposed on the `__missionPersist` bridge so the scripted acceptance can
/// assert the recency after an edit + debounce, and its reset after reload.
#[must_use]
pub fn last_flush_ms() -> Option<f64> {
    LAST_FLUSH_MS.with(Cell::get)
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
    // T-804 — the last COMPLETED flush's epoch-ms instant, or `null` if none has completed this page
    // lifetime. Read-only, side-effect-free (reads the `LAST_FLUSH_MS` cell). The scripted F-24
    // acceptance keys the "draft saved Ns ago" chip's recency off this: after an edit + the ~5 s
    // debounce it is a fresh timestamp; on a never-edited mission it stays `null` (the content guard
    // refuses the empty write, so no flush completes) and the chip is absent; after reload it resets.
    let last_flush_fn = Closure::wrap(Box::new(move || -> JsValue {
        last_flush_ms().map_or(JsValue::NULL, JsValue::from_f64)
    }) as Box<dyn FnMut() -> JsValue>);
    // T-374 — the refusal counters, as JSON. A guard that only ever *declines* to act is invisible:
    // "the record is still good" is equally consistent with the guard firing and with no write
    // having been attempted at all. These make the refusal itself observable, so a probe can assert
    // the guard ran rather than asserting the absence of damage.
    let blocked_fn = Closure::wrap(Box::new(move || -> JsValue {
        JsValue::from_str(&format!(
            r#"{{"empty":{},"unreadable":{}}}"#,
            BLOCKED_EMPTY.with(Cell::get),
            BLOCKED_UNREADABLE.with(Cell::get)
        ))
    }) as Box<dyn FnMut() -> JsValue>);
    // T-374 — the crux, evaluated in the REAL wasm runtime rather than argued about.
    //
    // Constructs a fresh `MissionDocCore`, encodes it, and reports the bytes alongside both
    // verdicts: what `bytes.is_empty()` would have decided and what the guard decides now. This
    // exists because the defect is a claim about a specific byte sequence — an empty document
    // encodes to `[0, 0]`, which is two bytes and therefore not empty — and a claim about bytes
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
            blob_has_content(&bytes)
        ))
    }) as Box<dyn FnMut() -> JsValue>);
    // T-374 — the content predicate, over the record on disk for this mission. Answers "is what is
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
                    stored.as_deref().is_some_and(blob_has_content)
                )))
            })
            .into()
        }) as Box<dyn FnMut() -> JsValue>)
    };
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
