//! Stores and restores the paired local mission snapshots.
use super::*;
/// The in-memory half of a snapshot  an instant, IDB-independent restore for the session that took
/// the swap. Keyed by **owner**, mission id and kind: by owner so a change of account can't read the
/// previous account's document (), by mission so navigating to another mission can't restore the
/// wrong doc, by kind so the two slots of the pair can't shadow each other.
struct LocalBackup {
    /// the account signed in when this snapshot was captured, resolved by the same
    /// `yrs_persist::owner_token()` that namespaces the IndexedDB record it mirrors (`discord_id`,
    /// or `anon` while signed out).
    ///
    /// It is stored rather than re-derived because the token is a function of *now*: a snapshot
    /// taken by A must stay attributed to A across a sign-out, or the record would silently follow
    /// the page to whoever signs in next  which is the whole defect, one layer up from the IDB
    /// records  scoped.
    owner: String,
    mission_id: String,
    kind: SnapshotSlot,
    bytes: Vec<u8>,
}

/// The editor mount this module's recovery surface is bound to: the mission id, plus the very
/// `DocHandle` `mission_editor::on_load` built for it (the same `Rc` it hands
/// `mission_history::set_ctx`). Re-registered on every boot by [`register_mission_backup`]  see
/// [`live_editor_is`] for what the pair proves and why the id alone is not enough.
struct LiveEditor {
    mission_id: String,
    doc: DocHandle,
}

thread_local! {
    static LOCAL_BACKUPS: RefCell<Vec<LocalBackup>> = const { RefCell::new(Vec::new()) };
    static LIVE_EDITOR: RefCell<Option<LiveEditor>> = const { RefCell::new(None) };
}

/// Write (replacing) the in-memory copy of one slot, under the account signed in **now** ().
///
/// The replace is scoped to that account too, so writing a snapshot cannot evict another account's:
/// the namespaces are independent, exactly as `yrs_persist`'s `(owner, logical) → key` mapping makes
/// them independent on disk. That matters beyond tidiness  an unscoped replace would make the mere
/// *presence* of A's recovery record depend on B's activity, and the one thing this pair must never
/// do is disappear because somebody else touched the machine.
fn remember(mission_id: &str, kind: SnapshotSlot, bytes: Vec<u8>) {
    let owner = crate::v2::apps::editor::shell::persist::owner_token();
    LOCAL_BACKUPS.with(|b| {
        let mut slots = b.borrow_mut();
        slots.retain(|s| s.owner != owner || s.mission_id != mission_id || s.kind != kind);
        slots.push(LocalBackup {
            owner,
            mission_id: mission_id.to_string(),
            kind,
            bytes,
        });
    });
}

/// Read the in-memory copy of one slot, if the account signed in **now** is the one that took it.
///
/// the owner test is the fix. Without it this lookup answered for whoever last used the page
/// rather than whoever is using it, and because [`has_snapshot`] consults it before the (already
/// scoped) IDB read, it shadowed the scoping  put on the records themselves.
fn recall(mission_id: &str, kind: SnapshotSlot) -> Option<Vec<u8>> {
    let owner = crate::v2::apps::editor::shell::persist::owner_token();
    LOCAL_BACKUPS.with(|b| {
        b.borrow()
            .iter()
            .find(|s| s.owner == owner && s.mission_id == mission_id && s.kind == kind)
            .map(|s| s.bytes.clone())
    })
}

/// Drop one slot, in memory and on disk. In-memory first and synchronously, so [`has_snapshot`]
/// tells the truth on the very next line; the IDB delete is deferred and best-effort (a failed
/// delete leaves a stale record  the pre-existing behaviour, not a new failure mode).
///
/// Both halves are scoped to the signed-in account (), and that is the conservative choice
/// rather than the convenient one. Every caller is an act of one account  a Save, or a new conflict
/// cycle  and `clear_state` was already owner-scoped on the IDB side, so an unscoped in-memory
/// delete would have let one account's Save destroy another's last in-session copy.  drew the
/// same line for the same reason (`yrs_persist::clear_state`: records are "dropped by an explicit
/// `adopt_orphans()`, never as a side effect of somebody else's Save"). The one operation allowed to
/// cross the namespace boundary is [`purge_local_documents`], and there the account is deleting its
/// own.
pub(super) fn forget_snapshot(mission_id: &str, kind: SnapshotSlot) {
    let owner = crate::v2::apps::editor::shell::persist::owner_token();
    LOCAL_BACKUPS.with(|b| {
        b.borrow_mut()
            .retain(|s| s.owner != owner || s.mission_id != mission_id || s.kind != kind);
    });
    let key = snapshot_key(mission_id, kind.suffix());
    spawn_local(async move {
        if let Err(e) = crate::v2::apps::editor::shell::persist::clear_state(&key).await {
            web_sys::console::warn_1(&JsValue::from_str(&format!(
                "[t191] backup clear failed for {key}: {e:?}"
            )));
        }
    });
}

/// Drop every in-memory snapshot belonging to `owner`, returning how many went. The RAM half of
/// [`purge_local_documents`].
fn forget_owner(owner: &str) -> usize {
    LOCAL_BACKUPS.with(|b| {
        let mut slots = b.borrow_mut();
        let before = slots.len();
        slots.retain(|s| s.owner != owner);
        before - slots.len()
    })
}

/// Expire **every** snapshot on record for `mission_id`  the expiry these records never had.
///
/// Called from `mission_commands::save_now` on a 201, which is the one moment the snapshots stop
/// being anybody's last copy: the document in front of the user is now an immutable server version,
/// and a server version is one refetch away. Clearing the pair bounds stored whole-document copies,
/// and `window.__missionBackup.has()` kept answering `true` for a document from weeks ago that a
/// restore would then swap over good current work.
///
/// **The accepted cost:** if the user adopted the server version, kept working on it, and saved,
/// their pre-conflict local work is only in `pre-adopt` and this drops it. That is the deliberate
/// reading of a Save  an explicit act that names one document as the one — and the alternative
/// (never expire) is both the unbounded-growth defect and a live hazard, because the older the
/// record gets the more likely restoring it is the destructive move.
pub fn clear_local_backups(mission_id: &str) {
    forget_snapshot(mission_id, SnapshotSlot::PreAdopt);
    forget_snapshot(mission_id, SnapshotSlot::PreRestore);
}

/// the sign-out purge: destroy every local document belonging to `owner`, in RAM and on disk.
///
/// Called from `auth::clear_session` with the `discord_id` read out of the session signal **before**
/// the signals are cleared. That ordering is load-bearing: `yrs_persist`'s token resolves from
/// `localStorage["tbd-auth"]`, which sign-out also clears, so a token resolved after the fact is
/// `anon`  the purge would then delete a signed-out visitor's drafts and leave the departing
/// account's exactly where they were.
///
/// Both halves are needed and neither is redundant. The RAM half is this module's snapshot cache,
/// which lives *in front of* the IDB records and is consulted first ([`has_snapshot`]); the disk half
/// is every record under the owner's key prefix  the live doc plus both snapshot slots, all three by
/// construction, because `yrs_persist::purge_owner` scans a prefix and the  suffixes are logical
/// keys under it. Purging only the disk would leave the in-memory hit shadowing the deletion for the
/// rest of the page load; purging only RAM would leave the documents on the disk of a shared machine.
///
/// This is the **only** operation in this module that deletes across the account boundary, and it is
/// the departing account deleting its own. Unowned pre-scoping records are untouched: they match no
/// owner prefix, so the  orphan contract  never returned, never destroyed, recoverable by an
/// explicit `__missionPersist.adopt_orphans()`  is intact.
///
/// The IDB sweep is spawned rather than awaited because sign-out is synchronous and must not block on
/// an IndexedDB round-trip. Nothing races it into a leak: the RAM drop above is synchronous, every
/// surviving read is account-scoped, a debounced write armed by the departing account is dropped by
/// `yrs_persist::run_save`'s owner check, and the next editor boot's `evict_foreign_records` sweeps
/// anything a failed delete left behind.
pub fn purge_local_documents(owner: &str) {
    let dropped = forget_owner(owner);
    let owner = owner.to_string();
    spawn_local(async move {
        let gone = crate::v2::apps::editor::shell::persist::purge_owner(&owner).await;
        web_sys::console::log_1(&JsValue::from_str(&format!(
            "[t338] sign-out purge: dropped {dropped} in-memory snapshot(s) and deleted {gone} local record(s)"
        )));
    });
}

/// Store one snapshot of the live document in both tiers  the in-session copy and the record that
/// outlives a reload  and report how many slots went into it.
///
/// The capture itself belongs to the engine: WHEN the encode happens relative to the replacement is
/// the property that makes a snapshot a snapshot, and it is the engine that holds it, running the
/// encode synchronously inside the call before the caller has touched anything. This function is
/// the transport half  where the bytes go, and that the record write is deferred so an adopt is
/// never waiting on a store round-trip.
///
/// A failed record write is non-fatal: the in-session copy, and for a pre-adopt the undo step,
/// still stand.
pub(super) fn snapshot_local(
    doc: &DocHandle,
    mission_id: &str,
    kind: SnapshotSlot,
) -> Option<usize> {
    capture_document_snapshot(doc, &|bytes| {
        remember(mission_id, kind, bytes.clone());
        let key = snapshot_key(mission_id, kind.suffix());
        spawn_local(async move {
            if let Err(e) = crate::v2::apps::editor::shell::persist::save_state(&key, &bytes).await
            {
                web_sys::console::warn_1(&JsValue::from_str(&format!(
                    "[backup] save failed for {key}: {e:?}"
                )));
            }
        });
    })
}

/// Is a snapshot of `kind` on record for `mission_id` **for the account signed in now**? Checks the
/// in-session copy first, then IDB (the copy that outlives a reload).
///
/// both tiers are account-scoped, so the order is a cache optimisation and nothing more. It
/// [`recall`] checks the current account before returning a cached document.
pub(super) async fn has_snapshot(mission_id: &str, kind: SnapshotSlot) -> bool {
    if recall(mission_id, kind).is_some() {
        return true;
    }
    crate::v2::apps::editor::shell::persist::load_state(&snapshot_key(mission_id, kind.suffix()))
        .await
        .is_some_and(|b| !b.is_empty())
}

/// Bind the recovery surface to the editor mount that is booting. Called from
/// [`register_mission_backup`], i.e. once per editor boot, with the `DocHandle` `on_load` created
/// for this mission  so the pair is the live editor by construction, never a stale capture.
pub(super) fn set_live_editor(mission_id: &str, doc: &DocHandle) {
    LIVE_EDITOR.with(|e| {
        *e.borrow_mut() = Some(LiveEditor {
            mission_id: mission_id.to_string(),
            doc: doc.clone(),
        });
    });
}

/// Is `mission_id` the mission the live editor is showing  and is the document
/// `mission_history::doc_handle()` resolves the one that mission booted with?
///
/// Both halves are load-bearing. [`restore_snapshot`] is not handed a document: it is handed an id,
/// and it asks `HISTORY_CTX` for somewhere to put the bytes. `HISTORY_CTX` follows the LIVE editor
/// and is never cleared, so the guard prevents a stale closure for mission A from writing into
/// mission B's document and triggering its persist.
/// under B's key. Silent, total data loss on a mission the user never even conflicted on.
///
///   * The **id** check catches the stale caller (a `.forget()`'d closure from a previous mount, or
///     a recovery control retained across mounts).
///   * The **`Rc::ptr_eq`** check catches the window the id alone cannot see: between
///     `mission_history::set_ctx(B)` (synchronous in `on_load`) and this module's re-registration
///     for B (an IDB round-trip later, inside the boot task), the id still reads `A` while the ctx
///     doc is already B's  exactly the case that must be refused. A fresh `DocHandle` per mount
///     makes pointer identity the exact test; the in-place `*doc.borrow_mut() = …` swaps that the
///     IDB restore and this module perform do not disturb it.
pub(super) fn live_editor_is(mission_id: &str) -> bool {
    let Some(ctx_doc) = crate::v2::apps::editor::bridge::document_host::history::doc_handle()
    else {
        return false;
    };
    LIVE_EDITOR.with(|e| {
        e.borrow()
            .as_ref()
            .is_some_and(|live| live.mission_id == mission_id && Rc::ptr_eq(&live.doc, &ctx_doc))
    })
}

/// Restore the pre-adopt snapshot over the live document  the "I did not mean that" lever for the
/// conflict adopt. Prefers the in-session copy, falls back to the IDB record. `true` when the
/// document was replaced.
///
/// The snapshot is **not** consumed: after a restore the server version is still one refetch away,
/// while the local work exists nowhere else, so the safer record to keep is this one. What the
/// restore displaces is written to `<id>::pre-restore` first  see [`restore_snapshot`].
pub async fn restore_local_backup(mission_id: String) -> bool {
    restore_snapshot(mission_id, SnapshotSlot::PreAdopt).await
}

/// Undo a [`restore_local_backup`]: put back the (server) document that restore displaced.
///
/// The inverse verb, and the reason a restore is now as reversible as the adopt it recovers from.
/// It is a true inverse, not a rollback  it snapshots the document *it* displaces into
/// `<id>::pre-adopt` on the way through, so a user who restores, edits for an hour and then changes
/// their mind again does not lose the hour.
pub async fn undo_local_restore(mission_id: String) -> bool {
    restore_snapshot(mission_id, SnapshotSlot::PreRestore).await
}

/// The shared body of both restore verbs: refuse unless this is the live editor's own mission, swap
/// the requested snapshot in as a fresh core, and bank whatever that swap displaced in the
/// counterpart slot.
async fn restore_snapshot(mission_id: String, want: SnapshotSlot) -> bool {
    // The mismatch says so, loudly, on both channels  a silent `false` here is indistinguishable
    // from "no backup on record", and the whole defect was that this path failed quietly.
    if !live_editor_is(&mission_id) {
        let msg = format!(
            "Did not restore: the {} backup belongs to mission {mission_id}, which is not the mission that is open. Open that mission and try again.",
            want.label()
        );
        web_sys::console::error_1(&JsValue::from_str(&format!("[t191] {msg}")));
        notify(&msg);
        return false;
    }
    let bytes = match recall(&mission_id, want) {
        Some(b) => b,
        None => crate::v2::apps::editor::shell::persist::load_state(&snapshot_key(
            &mission_id,
            want.suffix(),
        ))
        .await
        .unwrap_or_default(),
    };
    if bytes.is_empty() {
        return false;
    }
    let Some(doc) = crate::v2::apps::editor::bridge::document_host::history::doc_handle() else {
        return false;
    };
    // Rebuild as a FRESH core and swap, exactly like the boot IDB restore. Applying the update over
    // the live doc would MERGE the two states  yrs is a CRDT, and replaying an old update can
    // never delete the rows the live state inserted  which is the one thing a restore must not do.
    let fresh = MissionDocCore::new();
    fresh.set_origin_init(true);
    let ok = fresh.apply_update(&bytes).is_ok();
    fresh.set_origin_init(false);
    if !ok {
        return false;
    }
    //  fix  bank what this swap is about to destroy BEFORE destroying it. The first pass went
    // straight from here to the swap below, which drops the previous core (and with it the adopt's
    // undo step: a fresh core's stack is empty, so `can_undo()` is false the moment a restore lands)
    // and then lets `schedule_edit_persist` overwrite the plain `<id>` record  the last remaining
    // copy. A restore the user did not mean left them nothing.
    //
    // Placed after the `apply_update` check rather than literally first so a corrupt blob costs
    // nothing; the encode is still pre-swap by construction, because `fresh` is a separate core and
    // nothing has touched `doc` yet.
    let displaced = snapshot_local(&doc, &mission_id, want.counterpart());
    *doc.borrow_mut() = Some(fresh);
    // The restored local document does not derive from the adopted server semver.
    // clearing the `tbd-editor-adopted:<id>` marker here so the next cold boot would not silently
    // trust local against the wrong version  but  replaced that test with `classify_local`,
    // which compares the two *documents* and cannot be misled by a stale semver, so there is nothing
    // left to clear. `set_dirty(true)` below carries the whole of the signal now.
    // Wholesale document swap: rebind glyphs/HUD/docks (`after_local_edit` would be wrong  it
    // rebinds from a doc it assumes was edited in place), then mark dirty and re-arm the persist so
    // the restored document becomes the local record rather than the displaced one.
    crate::v2::apps::editor::bridge::document_host::history::rebind_engine_from_doc();
    crate::v2::apps::editor::bridge::document_host::history::set_dirty(true);
    crate::v2::apps::editor::shell::persist::schedule_edit_persist(doc, &mission_id);
    // Name the way back, for the same reason the adopt names Ctrl/Cmd+Z: a recovery nobody knows
    // about is not a recovery.
    let banked = match displaced {
        Some(n) => format!(" ({n} objects)"),
        None => String::new(),
    };
    notify(&match want {
        SnapshotSlot::PreAdopt => format!(
            "Restored your local copy. The server version it replaced{banked} was backed up — run window.__missionBackup.undoRestore() to put it back."
        ),
        SnapshotSlot::PreRestore => format!(
            "Put the server version back. The local copy it replaced{banked} was backed up — run window.__missionBackup.restore() to return to it."
        ),
    });
    true
}
