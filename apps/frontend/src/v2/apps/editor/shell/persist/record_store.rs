//! Reads and writes owner-scoped IndexedDB mission records.
use super::*;
/* ───────────────────────  per-account record scoping ─────────────────────── */

/// The account a record belongs to  the Discord id out of `localStorage["tbd-auth"]`.
///
/// **Why `discord_id`.** It is the identity the *server* keys on, and the bug is about server
/// identity: "a Save then posts it under B's account" is a statement about account ownership, so the
/// local record has to be partitioned by the same thing the Save is attributed to. It is also
/// immutable (a snowflake  usernames and avatars are not) and it survives a reload without the
/// reactive `AuthStore`, which matters because this module runs inside detached async tasks and
/// timer callbacks that hold no Leptos context. It is deliberately **not** a credential: the access
/// token rotates and the refresh token is single-use, so keying on either would re-namespace the
/// same user mid-session and hide their own draft from them.
///
/// Read as a loose `serde_json::Value` rather than through `auth::from_persist_json`. That parse is
/// strict over the whole `User` struct, so a single added or renamed backend field would make it
/// return `None`, silently demote a signed-in user to the anonymous namespace, and lose them their
/// work. Exactly one string is needed here and exactly that string is allowed to fail.
pub(super) fn current_owner() -> Option<String> {
    let storage = web_sys::window()?.local_storage().ok()??;
    let raw = storage
        .get_item(crate::v2::core::auth::AUTH_PERSIST_KEY)
        .ok()??;
    let blob: serde_json::Value = serde_json::from_str(&raw).ok()?;
    let id = blob
        .get("state")?
        .get("user")?
        .get("discord_id")?
        .as_str()?
        .trim();
    (!id.is_empty()).then(|| id.to_string())
}

/// [`current_owner`] with the signed-out fallback applied  the namespace a read or write uses now.
///
/// ** `pub`, because a second cache had to agree with this one.** `mission_hydrate` keeps an
/// in-memory copy of each  snapshot in front of these records, and it was not keyed by account:
/// its lookup hit before the scoped IDB read, so within one page load a change of account did not
/// hide the previous account's document. There is exactly one right token for that cache to key on —
/// this one  because any other choice makes "is a backup on record?" and "can a restore read it?"
/// two different questions, and the whole hazard is a `has()` that answers for a document the reader
/// cannot legitimately have.
///
/// The session is this module's to read; the namespace a signed-out author lands in belongs to the
/// engine's key space, which is why the fallback is applied there and not here.
#[must_use]
pub fn owner_token() -> String {
    owner_token_or_anonymous(current_owner().as_deref())
}

/// Retry a failed read of `physical_key` three times with backoff, then stay locked out. See
/// [`RecordRead::Failed`] and [`run_save`].
///
/// A failed `get` enters a bounded retry path, so a transient IndexedDB error does not
/// disable autosave for the rest of the page lifetime. Each failed attempt reports
/// [`save_status::SaveStatus::Unreadable`]; after [`UNREADABLE_RETRY_LIMIT`] the chip offers Retry.
///
/// #  the latch goes on FIRST, and the retries run behind it
///
///  also moved the `UNREADABLE` insert to *after* the loop, and that inverted the guard it
/// feeds. The backoffs are 80 + 160 + 320 ms, so for ~560 ms after a read failed the set was still
/// empty  and [`run_save`]'s fourth guard consults exactly that set. A debounce firing inside the
/// window therefore passed a guard whose whole purpose is "never write over a record this session
/// could not read", and overwrote a record already known unreadable. The window is not exotic: the
/// idle debounce is 1 s and the boot read is what fails, so a single edit during boot lands inside
/// it.
///
/// Latching first costs nothing the retry does not give back  every arm below that *learns* the
/// key is safe (`Hit`, and `Miss`, which means there is no record to protect) clears the latch in
/// the same breath, and `retry_unreadable_keys` re-enters here from the chip. The trade is
/// deliberate and one-sided: a false latch delays autosave by one debounce window; a false clear
/// destroys the only copy.
pub(super) async fn note_unreadable(physical_key: &str) -> Option<Vec<u8>> {
    UNREADABLE.with(|u| {
        u.borrow_mut().insert(physical_key.to_string());
    });
    for attempt in 1u8..=UNREADABLE_RETRY_LIMIT {
        sleep_ms(record_read_retry::backoff_before_attempt_ms(attempt)).await;
        match read_raw(physical_key).await {
            RecordRead::Hit(bytes) => {
                UNREADABLE.with(|u| {
                    u.borrow_mut().remove(physical_key);
                });
                save_status::report_saved();
                return Some(bytes);
            }
            RecordRead::Miss => {
                UNREADABLE.with(|u| {
                    u.borrow_mut().remove(physical_key);
                });
                return None;
            }
            RecordRead::Failed => {
                save_status::report_unreadable(attempt);
            }
        }
    }
    save_status::report_unreadable(UNREADABLE_RETRY_LIMIT);
    None
}

/// wasm `setTimeout` as a future  the backoff for [`note_unreadable`].
pub(super) async fn sleep_ms(ms: i32) {
    let promise = js_sys::Promise::new(&mut |resolve, _reject| {
        let Some(window) = web_sys::window() else {
            let _ = resolve.call0(&JsValue::UNDEFINED);
            return;
        };
        let done = resolve.clone();
        let cb = Closure::once_into_js(move || {
            let _ = done.call0(&JsValue::UNDEFINED);
        });
        let _ = window
            .set_timeout_with_callback_and_timeout_and_arguments_0(cb.as_ref().unchecked_ref(), ms);
    });
    let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
}

/// Report a pre-scoping record that a read just declined to return.
///
/// The alternative designs are both worse. Returning it re-creates the exact defect this ticket
/// closes  the record predates ownership, so "it is probably yours" is a guess made on behalf of
/// someone else's unsaved work. Deleting it destroys the one copy of a document that may exist
/// nowhere else. So it is kept, and it is made *loud*: the failure mode this warning exists to
/// prevent is a silent `None` reading as "no backup" to a user who is reaching for a backup.
pub(super) fn note_orphan(logical: &str) {
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
/// (`ObjectStoreParams::new()` with no `key_path`/`auto_increment`)  the key is supplied on every
/// `put`, mirroring the React `createObjectStore(STORE)`.
pub(super) async fn open_db() -> Result<idb::Database, idb::Error> {
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
pub(super) async fn put_raw(key: &str, bytes: &[u8]) -> Result<(), idb::Error> {
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

/// the outcome of reading one physical key, with **`Miss` and `Failed` kept apart**.
///
/// [`RecordRead`] distinguishes a missing key from a failed `open_db`, transaction, or `get`.
/// Treating a read failure as an absent record would be a **false
/// negative on the one question the boot seam asks**: [`load_state`] reported "no local content",
/// the editor took the cold path, and the seed then went down the write path at a record that may
/// well have been someone's only copy. Two different facts deserve two different answers  the
/// same argument [`load_state`]'s own doc comment already makes for orphans.
pub(super) enum RecordRead {
    /// The record is there and decoded to bytes.
    Hit(Vec<u8>),
    /// The store answered, and there is nothing at this key.
    Miss,
    /// IndexedDB could not be asked, or answered with something unreadable. **Not** a miss:
    /// nothing here licenses the claim that the key is empty.
    Failed,
}

/// Read one physical key, three-valued (). A stored value that is not a `Uint8Array` counts as
/// [`RecordRead::Failed`], not `Miss`  a record exists, this code just cannot read it, which is
/// exactly the state that must not masquerade as "nothing is stored".
pub(super) async fn read_raw(key: &str) -> RecordRead {
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
pub(super) async fn get_raw(key: &str) -> Option<Vec<u8>> {
    match read_raw(key).await {
        RecordRead::Hit(bytes) => Some(bytes),
        RecordRead::Miss | RecordRead::Failed => None,
    }
}

/// Does a record exist at one physical key? `getKey` returns the key alone, so the orphan probe on
/// the [`load_state`] miss path costs a key lookup instead of a whole-document read.
pub(super) async fn has_raw(key: &str) -> bool {
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
pub(super) async fn delete_raw(key: &str) -> Result<(), idb::Error> {
    let db = open_db().await?;
    let tx = db.transaction(&[STORE], idb::TransactionMode::ReadWrite)?;
    let store = tx.object_store(STORE)?;
    store.delete(JsValue::from_str(key))?.await?;
    tx.commit()?.await?;
    Ok(())
}

/// Every physical key in the store. The store holds a handful of records per account, so a full
/// key scan is the honest way to answer "what is on this machine"  and it is the only way to see
/// records whose owner is no longer known to this session.
pub(super) async fn all_keys() -> Vec<String> {
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
/// was *armed* under or to nothing at all  see [`run_save`]. Resolving it here instead would leave
/// a window in which the sign-out landed between the check and the `put`.
pub(super) async fn save_state_as(owner: &str, id: &str, bytes: &[u8]) -> Result<(), idb::Error> {
    put_raw(&scoped_key(owner, id), bytes).await
}

/// Persist the whole encode blob for `id` under the account signed in right now.
pub async fn save_state(id: &str, bytes: &[u8]) -> Result<(), idb::Error> {
    save_state_as(&owner_token(), id, bytes).await
}

/// Load the blob for `id` (React `loadState` → `value ?? null`). Any error / absence → `None`.
///
/// On a miss it probes for the pre-scoping record at the bare logical key. That record is **not**
/// returned  it carries no owner, so returning it is the cross-account restore this ticket exists
/// to stop  but its existence is reported ([`note_orphan`]) rather than collapsed into the same
/// silent `None` that a genuinely empty store produces. "There is nothing" and "there is something
/// I will not hand you" are different answers and a caller reaching for a backup deserves the second
/// one out loud.
///
/// #  a failed read is not an empty store
///
/// The read path keeps storage errors distinct from an absent record. The boot seam treats `None` as "no
/// local content" and keeps the seed. The failure is now reported and remembered
/// ([`note_unreadable`]) so that [`run_save`] will not let the seed overwrite a record this page
/// lifetime failed to read  see that function's third guard. The return type is unchanged (`None`
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
            return note_unreadable(&scoped).await;
        }
        RecordRead::Miss => {}
    }
    if has_raw(id).await {
        note_orphan(id);
    }
    None
}

/// when the local draft for `id` was last written, epoch ms, or `None` when this browser
/// holds no stamp for it (no draft yet, or one written before ).
///
/// Lives here rather than in `tab_lock` because the *key* is this module's business: `scoped_key`
/// is private and account scoping () is the one thing a caller must not have to reproduce.
/// `tab_lock` owns the stamp's shape; this owns which record it describes.
#[must_use]
pub fn draft_written_at(id: &str) -> Option<f64> {
    tab_lock::read_stamp(&scoped_key(&owner_token(), id)).map(|s| s.at)
}

/// Delete the blob for `id` (React `clearState`).
///
/// Scoped-key only, deliberately. This is reached from `mission_hydrate::clear_local_backups` on a
/// successful Save  an act that speaks for one account — and a pre-scoping record at the bare key
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
/// at the next editor boot (session expiry, revoked token, browser closed), and  since  —
/// `mission_hydrate::purge_local_documents` from `auth::clear_session` on a deliberate sign-out.
///
/// One prefix scan covers all three record kinds by construction: the live doc `<id>` and both
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
/// Run once per editor boot. A sign-out hook alone cannot carry the guarantee  a session that
/// expires, a revoked token, or a browser closed with the tab open all leave records behind that no
/// handler ever runs for  so the rule is enforced where it can actually be checked: at the next
/// boot, the only account with records on this machine is the account using it.
///
/// **No-op while signed out**, and that is load-bearing in two directions. A signed-out visitor must
/// not be able to delete a signed-in user's drafts, and the editor smokes run logged out against
/// the same store. Unowned pre-scoping records are also left alone: this evicts records it can
/// attribute to someone else, and an orphan is by definition one it cannot attribute at all.
pub(super) async fn evict_foreign_records() {
    let Some(me) = current_owner() else {
        return;
    };
    let strangers: BTreeSet<String> = all_keys()
        .await
        .iter()
        .filter_map(|k| split_scoped_key(k).map(|(owner, _)| owner.to_string()))
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

/// The logical keys of every pre-scoping record still on this machine  what
/// `__missionPersist.orphans()` reports.
pub(super) async fn orphan_keys() -> Vec<String> {
    all_keys()
        .await
        .into_iter()
        .filter(|k| split_scoped_key(k).is_none())
        .collect()
}

/// Claim every pre-scoping record for the signed-in account. Returns `(adopted, skipped)`.
///
/// Deliberately manual. Nothing in the data can prove who wrote these records, so the act of
/// claiming them is a human saying "this machine is mine"  the one authority that actually exists
/// here. A logical key that already has a record under this account is **skipped**, never
/// overwritten: adoption is for recovering a stranded document, not for letting an older one
/// clobber the work someone is doing now. Adopted records are removed from the unowned key so the
/// claim happens exactly once.
pub(super) async fn adopt_orphans(owner: &str) -> (usize, usize) {
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
