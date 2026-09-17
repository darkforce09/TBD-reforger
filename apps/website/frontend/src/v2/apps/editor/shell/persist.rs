//! Stores owner-scoped mission drafts in IndexedDB and schedules serialized writes.
//! A write reads the current document at commit time, refuses empty or unreadable content,
//! merges a peer record before replacing it, and publishes completion only after storage succeeds.

#![allow(clippy::cast_precision_loss)] // usize slot count → f64 for the JS bridge; tiny.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeSet, HashMap};
use std::rc::Rc;

use idb::DatabaseEvent; // brings `VersionChangeEvent::database()` into scope for the upgrade handler
use leptos::task::spawn_local;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use website_map_engine::data::store::MissionDocCore;
use website_map_engine::editing::persist::record_key::{
    owner_prefix, owner_token_or_anonymous, scoped_key, split_scoped_key,
};
use website_map_engine::editing::persist::stored_blob::restores_to_authored_content;
use website_map_engine::editing::persist::{merge_policy, record_read_retry, slot_fingerprint};

use crate::v2::apps::editor::bridge::document_host::doc_host::DocHandle;
use crate::v2::apps::editor::shell::save_status::{self, IDLE_DEBOUNCE_MS, UNREADABLE_RETRY_LIMIT};
use crate::v2::apps::editor::shell::tab_lock;

/// IndexedDB coordinates  identical to `yrsPersist.ts` (`DB_NAME` / `STORE` / v1). Distinct from the
/// legacy v1 `tbd-mission-${id}` and v2 `tbd-mission-persist`; **no migration** (legacy drafts drop).
const DB_NAME: &str = "tbd-mission-yrs";
const STORE: &str = "doc-state";
const DB_VERSION: u32 = 1;
/// idle debounce is at most 1 s (was React `delay = 5000`). A burst still coalesces
/// into one write; hiding the tab flushes immediately instead of waiting out this window.
const DEBOUNCE_MS: i32 = IDLE_DEBOUNCE_MS;

mod record_store;
use record_store::*;
pub use record_store::{
    clear_state, draft_written_at, load_state, owner_token, purge_owner, save_state,
};
mod save_scheduler;
use save_scheduler::*;
pub use save_scheduler::{
    edit_persist_count, flush_state, register_flush_on_hide, save_state_debounced,
    schedule_edit_persist,
};
mod mount_persistence;
use mount_persistence::*;
pub use mount_persistence::{last_flush_ms, register_mission_persist, set_last_flush_signal};

thread_local! {
    /// Logical keys we have already complained about, so a polled `__missionBackup.has()` cannot
    /// turn one stranded record into a console flood. Warn once, stay warned.
    static WARNED_ORPHANS: RefCell<BTreeSet<String>> = const { RefCell::new(BTreeSet::new()) };
    /// **physical** keys whose read failed this page lifetime (see [`RecordRead::Failed`]).
    /// Consulted by [`run_save`], which refuses to write over a record that is present but was not
    /// readable. Physical, not logical: it is the exact key a write would land on.
    static UNREADABLE: RefCell<BTreeSet<String>> = const { RefCell::new(BTreeSet::new()) };
    /// F-20  mission ids we have already noted an empty-content refusal for, so a brand-new mission
    /// (whose every debounced tick refuses until the first authored content lands) cannot flood the
    /// console with the same  refusal 8× over. The `BLOCKED_EMPTY` counter above still tallies
    /// EVERY refusal (the bridge reads it); this only gates the console message. Same "note once, stay
    /// noted" shape as [`WARNED_ORPHANS`].
    static WARNED_EMPTY: RefCell<BTreeSet<String>> = const { RefCell::new(BTreeSet::new()) };
    /// how many writes the guards refused, by reason. Exposed on the bridge
    /// (`__missionPersist.blocked_writes()`) so a probe can prove the guard **fired**, rather than
    /// inferring it from a record that merely happens to be unchanged.
    static BLOCKED_EMPTY: Cell<u32> = const { Cell::new(0) };
    static BLOCKED_UNREADABLE: Cell<u32> = const { Cell::new(0) };
    /// how many writes were re-armed because another tab holds the writer role, and how
    /// many went out as a read-merge-write rather than a blind put. Same argument as the two
    /// counters above: a guard that only ever declines to act is invisible, and "the record is
    /// intact" is equally consistent with the guard firing and with nothing having been attempted.
    /// `__missionPersist.blocked_writes()` reports both, so the two-tab acceptance can assert that
    /// the merge **ran** instead of inferring it from a document that happens to look right.
    static BLOCKED_READ_ONLY: Cell<u32> = const { Cell::new(0) };
    static MERGED_WRITES: Cell<u32> = const { Cell::new(0) };
}
/* ─────────────────────── debounced + serialized writer ─────────────────────── */

type GetBytes = Box<dyn Fn() -> Vec<u8>>;
type IsCancelled = Box<dyn Fn() -> bool>;
/// an O(1) "does the doc these bytes came from hold content" answer, for callers that hold
/// the live [`DocHandle`]. See [`PendingSave::content_probe`].
type ContentProbe = Box<dyn Fn() -> bool>;

struct PendingSave {
    get_bytes: GetBytes,
    is_cancelled: IsCancelled,
    /// the account signed in when this write was armed. A debounce is 1 s wide and a
    /// sign-out is instant, so without this the timer could fire after the handover and file one
    /// user's document under the next one's name: the ticket's defect, reintroduced through the
    /// back door. Checked in [`run_save`].
    owner: String,
    /// the cheap content test, when the caller can supply one.
    ///
    /// `Some` means the caller holds the live [`DocHandle`] the bytes are encoded from, so
    /// [`run_save`] can call [`MissionDocCore::has_content`] on it directly  O(1) — instead of
    /// decoding the blob. **This is sound only because it is sampled in the same synchronous window
    /// as `get_bytes`, with no `.await` between**: wasm is single-threaded, so nothing can mutate
    /// the document between the encode and the probe, and the two therefore describe one state.
    ///
    /// `None` → [`restores_to_authored_content`] decodes the blob. That is the correct fallback and not a
    /// degraded one: it tests the bytes themselves, so it also catches a corrupt blob, which a live
    /// probe by construction cannot. It costs one document decode, which is why the per-edit writer
    /// supplies a probe and the once-per-boot writer does not have to.
    content_probe: Option<ContentProbe>,
}

/// A live debounce timer: the `setTimeout` handle + the `Closure` it fires (kept alive here so it is
/// NOT leaked per-call; dropped when the timer is cleared/re-armed  never from inside its own fire).
struct TimerEntry {
    handle: i32,
    _closure: Closure<dyn FnMut()>,
}

thread_local! {
    // Module singletons, keyed by mission id  the exact React `timers`/`pending`/`chains` triple.
    // wasm is single-threaded, so a `thread_local! RefCell` is the sound analogue of the JS `Map`s.
    static TIMERS: RefCell<HashMap<String, TimerEntry>> = RefCell::new(HashMap::new());
    static PENDING: RefCell<HashMap<String, PendingSave>> = RefCell::new(HashMap::new());
    // Per-mission async lock so writes never interleave (React's promise chain). Uncontended in
    // .17 (no mutators) but required by the contract + correct once mutators land.
    static LOCKS: RefCell<HashMap<String, Rc<futures::lock::Mutex<()>>>> = RefCell::new(HashMap::new());
    // how many times a mutator re-armed the debounce via `schedule_edit_persist`. Starts
    // 0 at boot (the boot persist calls `save_state_debounced` directly, NOT this), so the
    // `__missionPersist.edit_persist_count()` gate proves the FIRST edit-driven write is scheduled
    // (a late `flush()` of the boot debounce would encode the moved doc anyway  the counter, not
    // the blob, is the sound signal that the edit itself re-armed the writer).
    static EDIT_PERSIST_COUNT: Cell<u32> = const { Cell::new(0) };
    /// True from the moment [`run_save`] takes the per-mission lock until its write settles.
    ///
    /// #  what this flag does NOT do, and what it does
    ///
    /// The per-mission lock serializes overlapping flushes from visibility and pagehide. Both listeners
    /// call [`flush_state`], which `PENDING.remove`s the entry **synchronously**, so the second one
    /// finds `None` and returns  and where they do overlap, [`lock_for`] serializes them. The flag
    /// was set, reset by [`SaveFlightGuard`], and **read by nothing**; its only test asserted the
    /// identifier appeared in one of two function bodies, which it did.
    ///
    ///  gave it the job the name always implied, because  created the first caller that
    /// genuinely needs it. A peer tab's `saved` announcement asks this tab to pull the record and
    /// [`merge_stored`] it into the **live document**. Between [`run_save`]'s `get_bytes` and its
    /// `put_raw` those bytes are already sampled, so a merge landing in that window would put
    /// blocks into the document that the write about to land does not carry  the record would go
    /// backwards relative to the document, silently, which is the very failure  exists to
    /// close. [`pull_peer_record`] therefore stands down while this is true and lets the in-flight
    /// save's own merge (`decide_save` → `SaveDecision::Merge`) do the work under the lock.
    static SAVE_IN_FLIGHT: Cell<bool> = const { Cell::new(false) };
    /// the live document a merge applies into, with the mission it belongs to. Installed by
    /// [`register_tab_sync`] from the boot seam.
    ///
    /// The id travels with the handle deliberately: `run_save` is keyed by mission and a stale
    /// pending can outlive a route change, so "the doc the editor has open" is not by itself an
    /// answer to "the doc these bytes came from". Matching the id makes the merge exact rather than
    /// best-effort  the same argument `mission_hydrate::restore_snapshot` makes for its own
    /// cross-mission refusal.
    static MERGE_DOC: RefCell<Option<(String, DocHandle)>> = const { RefCell::new(None) };
}

struct SaveFlightGuard;
impl Drop for SaveFlightGuard {
    fn drop(&mut self) {
        SAVE_IN_FLIGHT.set(false);
    }
}

/// Is a save between taking the per-mission lock and completing its write? See [`SAVE_IN_FLIGHT`].
#[must_use]
pub fn save_in_flight() -> bool {
    SAVE_IN_FLIGHT.with(Cell::get)
}

fn lock_for(id: &str) -> Rc<futures::lock::Mutex<()>> {
    LOCKS.with(|m| {
        m.borrow_mut()
            .entry(id.to_string())
            .or_insert_with(|| Rc::new(futures::lock::Mutex::new(())))
            .clone()
    })
}

/* ───────────────────────  the read-merge-write ─────────────────────── */

/// Wall-clock epoch ms. Split by target for the same reason [`note_flush_completed`] is.
#[cfg(target_arch = "wasm32")]
fn now_ms() -> f64 {
    js_sys::Date::now()
}
#[cfg(not(target_arch = "wasm32"))]
fn now_ms() -> f64 {
    0.0
}

/// Apply a stored blob into the live document for `mission_id`, and report whether it landed.
///
/// The document a merge targets is this tab's to know  it is installed by [`register_tab_sync`]
/// and dies with the tab  so the lookup and the id match sit here. The apply itself is
/// [`merge_policy::apply_update_into_document`]: a CRDT union, never a field-wise diff. Two tabs
/// that both restored from one record author under different client ids, so a sibling's blocks
/// integrate rather than collide, and a replay of this tab's own earlier blob is discarded as
/// already-seen  harmless, but an O(document) decode that learns nothing, which is what the write
/// stamp exists to let [`tab_lock::decide_save`] skip.
///
/// The HUD mirrors and the render SoA are rebound afterwards for the reason the boot restore
/// rebinds them: the document changed underneath the counters. A merge is not an undo step, so
/// Ctrl+Z after a sibling's edits arrive undoes the operator's own last action, not the sync.
///
/// No `.await` while the `RefCell` is borrowed  the engine task shares this `Rc`.
fn merge_stored(mission_id: &str, stored: &[u8]) -> bool {
    let Some((owner_id, doc)) = MERGE_DOC.with(|d| d.borrow().clone()) else {
        return false;
    };
    let applied = merge_policy::apply_update_into_document(&doc, &owner_id, mission_id, stored);
    if applied {
        crate::v2::apps::editor::bridge::document_host::history::refresh_hud();
        crate::v2::apps::editor::bridge::document_host::history::rebind_engine_from_doc();
    }
    applied
}

/// Read the record at `key`, merge it into the live document, and hand back the bytes to write.
///
/// The transport is this module's half of [`merge_policy::merge_before_write`]: the record read is
/// supplied as a closure so the engine owns the ORDER  read late, merge, re-encode — without
/// naming a store. An unreadable record reads the same as an absent one here, because both mean
/// "there is nothing this write can be shown to be losing".
async fn merge_before_write(id: &str, key: &str, bytes: Vec<u8>, get_bytes: &GetBytes) -> Vec<u8> {
    merge_policy::merge_before_write(
        bytes,
        || async move {
            match read_raw(key).await {
                RecordRead::Hit(stored) => Some(stored),
                RecordRead::Miss | RecordRead::Failed => None,
            }
        },
        &|stored| merge_stored(id, stored),
        &**get_bytes,
    )
    .await
}

/// A peer tab announced it just wrote the shared record  pull it into this document.
///
/// This is what makes a read-only tab converge rather than merely wait: it cannot write, but it can
/// (and must) see what the writer wrote, or the two documents drift until one of them reloads.
///
/// **It stands down while this tab's own save is in flight** ([`save_in_flight`], ): a merge
/// between [`run_save`]'s `get_bytes` and its `put_raw` would put blocks into the document that the
/// landing write does not carry. Nothing is lost by standing down  that in-flight save reads the
/// stamp, sees a foreign writer and merges the same record under the lock.
pub fn pull_peer_record() {
    if save_in_flight() {
        return;
    }
    let Some((id, _)) = MERGE_DOC.with(|d| d.borrow().clone()) else {
        return;
    };
    spawn_local(async move {
        let lock = lock_for(&id);
        let _guard = lock.lock().await;
        let key = scoped_key(&owner_token(), &id);
        if let RecordRead::Hit(stored) = read_raw(&key).await {
            merge_stored(&id, &stored);
        }
    });
}

/// wire this editor mount into the cross-tab machinery: park the document a merge applies
/// into, install the peer-saved reaction, join the mission's channel (which claims the writer lock)
/// and install `window.__missionTabs`.
///
/// One call, from the boot seam beside [`register_flush_on_hide`], so `mission_editor.rs` gains a
/// call site and no logic.
pub fn register_tab_sync(doc: DocHandle, mission_id: String) {
    MERGE_DOC.with(|d| *d.borrow_mut() = Some((mission_id.clone(), doc)));
    tab_lock::set_peer_saved_handler(Box::new(pull_peer_record));
    tab_lock::join(&mission_id);
    tab_lock::register_bridge();
}

/// Clear (and drop) any live timer for `id`. Called only from arm/flush  never from inside a
/// firing timer, so dropping the `Closure` here can't drop a running one.
fn clear_timer(id: &str) {
    if let Some(entry) = TIMERS.with(|t| t.borrow_mut().remove(id)) {
        if let Some(win) = web_sys::window() {
            win.clear_timeout_with_handle(entry.handle);
        }
    }
}

/// The debounce default, exposed so the boot seam arms the initial persist with the contract delay.
#[must_use]
pub const fn debounce_ms() -> i32 {
    DEBOUNCE_MS
}
