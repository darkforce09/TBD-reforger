//! Debounces and serializes mission document writes.
use super::*;
/// Serialized write: take the per-mission lock, then apply the guards in order  cancel check
/// **before** reading bytes,  owner check,  writer role,  **content** check,
/// unreadable-record check,  read-merge, then persist.
///
/// A changed owner **drops** the write; it does not redirect it. The bytes were composed by the
/// previous session, so writing them anywhere the new account can read is the cross-account leak
/// this ticket closes, and writing them back to the old account after a sign-out contradicts the
/// other half of it. Nothing is lost that matters: the document is still in memory, and the next
/// edit re-arms the writer under whoever is now signed in.
///
/// #  the guard order, and why the content test comes before the IO
///
/// The content test is pure CPU; the unreadable-record test costs an IndexedDB key lookup. Refusing
/// a content-empty blob first means the common rejection never touches the disk. Both are stated as
/// the same rule the  owner check states: **a write that cannot be shown to be safe does not
/// happen.** In every refusal the document is still in RAM and the next edit re-arms the writer, so
/// the cost of a false refusal is bounded by one debounce window; the cost of a false *acceptance*
/// is an authored mission.
///
/// #  the two cross-tab steps, and why they sit where they sit
///
/// The **role check is before the encode.** A read-only tab's debounce re-arms every idle window
/// for as long as the operator keeps typing, and `encode_state` is O(document) — 460 ms at 100k
/// slots, measured for the  probe. Paying that for a write that cannot land would make the
/// second tab the slow one, which is precisely the tab whose work we are trying not to punish. It
/// **re-arms rather than drops**: the read-only tab inherits the writer role the instant the other
/// tab closes, and its whole session's work has to survive to that moment.
///
/// The **stamp read is late**, just before the write, rather than beside the role check. It answers
/// "is the record on disk still the one I last wrote", and every millisecond between the answer and
/// the `put` is a millisecond in which a peer could make it wrong. Reading it last shrinks that
/// window to the merge's own `read_raw`. It cannot be closed entirely from one tab  IndexedDB has
/// no cross-tab transaction here  and it does not need to be: a peer whose bytes this write passes
/// over still holds them in its own live document, its `saved` announcement makes this tab pull,
/// and its next save reads *this* record and writes the union. The system converges; what
/// removes is the silent, permanent loss.
pub(super) async fn run_save(id: &str, pending: PendingSave) {
    let lock = lock_for(id);
    let _guard = lock.lock().await;
    SAVE_IN_FLIGHT.set(true);
    let _flight = SaveFlightGuard;
    if (pending.is_cancelled)() {
        return;
    }
    if owner_token() != pending.owner {
        return;
    }
    // is this tab the writer? Before the encode; see the section above.
    if !tab_lock::may_write() {
        BLOCKED_READ_ONLY.with(|c| c.set(c.get().saturating_add(1)));
        install_pending(id, pending, debounce_ms());
        return;
    }
    let bytes = (pending.get_bytes)();
    // a CONTENT test, not a byte test. An empty document encodes to `[0, 0]`: two bytes,
    // non-empty, and the old `bytes.is_empty()` guard wrote it straight over a good record. `bytes`
    // and the probe are read with no `.await` between them, so they describe one document state.
    let has_content = match &pending.content_probe {
        Some(probe) => !bytes.is_empty() && probe(),
        None => restores_to_authored_content(&bytes),
    };
    if !has_content {
        BLOCKED_EMPTY.with(|c| c.set(c.get().saturating_add(1)));
        // F-20  a brand-new mission refuses on every debounced tick until the operator authors the
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
    // and never write over a record this page lifetime FAILED to read. `load_state`
    // distinguishes an absent record from an unreadable record; on the second,
    // the boot keeps the seed, and letting the seed land here is how an unreadable-but-present
    // record becomes a destroyed one. Re-probe rather than latching: if the key is genuinely absent
    // there is nothing to protect, so the flag clears and the write proceeds  otherwise a single
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
                 Retry from the save-status chip, or reload."
            )));
            save_status::report_unreadable(UNREADABLE_RETRY_LIMIT);
            return;
        }
        UNREADABLE.with(|u| {
            u.borrow_mut().remove(&key);
        });
    }
    // READ, MERGE, then write. Everything above this line interrogates the *incoming* blob;
    // this is the first line that asks what is already on disk. `Merge` is the answer whenever the
    // record was written by another tab or by nobody this browser can attribute; `WriteThrough`
    // only when the stamp says these very bytes' document already contains it. See
    // [`tab_lock::decide_save`] and [`merge_before_write`].
    let bytes = match tab_lock::decide_save(
        tab_lock::role(),
        tab_lock::read_stamp(&key).as_ref(),
        &tab_lock::tab_id(),
    ) {
        tab_lock::SaveDecision::Defer => {
            // The role flipped during the encode  a peer took the writer lock. Same treatment as
            // the early check: re-arm, never drop.
            BLOCKED_READ_ONLY.with(|c| c.set(c.get().saturating_add(1)));
            install_pending(id, pending, debounce_ms());
            return;
        }
        tab_lock::SaveDecision::WriteThrough => bytes,
        tab_lock::SaveDecision::Merge => {
            MERGED_WRITES.with(|c| c.set(c.get().saturating_add(1)));
            merge_before_write(id, &key, bytes, &pending.get_bytes).await
        }
    };
    save_status::report_saving();
    if let Err(e) = save_state_as(&pending.owner, id, &bytes).await {
        let reason = save_status::format_save_error(&format!("{e} {e:?}"));
        web_sys::console::warn_1(&JsValue::from_str(&format!(
            "[yrs-persist] save failed: {reason}"
        )));
        save_status::report_failed(reason);
        return;
    }
    save_status::report_saved();
    // stamp the record with this tab and this instant, then tell the other tabs it is
    // there. Both AFTER the write settled, for the / ack reason stated below: the stamp
    // is what lets the next save skip a pointless O(document) merge and what dates the conflict
    // modal's local option, and a stamp for bytes that never landed would be a lie in both roles.
    let at = now_ms();
    tab_lock::write_stamp(&key, at);
    tab_lock::announce_saved(at);
    // a flush COMPLETED. Recorded here, in the one branch where the bytes actually reached
    // IndexedDB, and nowhere earlier: the  ack discipline is that this timestamp means "the
    // draft is on disk", not "a write was scheduled". Every refusal above (`is_cancelled`, the
    //  owner check, the  content and unreadable guards) returns before this line, and the
    // IO-error branch just above returns too, so the only way to reach it is a real write. That
    // guarantee is exactly what makes the strip's "draft saved Ns ago" chip honest  see
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
    // catches a corrupt blob)  just O(document). The boot seam arms this once per boot.
    arm_debounced(id, get_bytes, is_cancelled, None, delay_ms);
}

/// The body of [`save_state_debounced`], plus 's optional [`ContentProbe`]. Private: the two
/// callers are [`save_state_debounced`] (no probe) and [`schedule_edit_persist`] (probe), and a
/// third public arming surface with no caller is the mistake this module's  note already
/// records through the same owner-scoped writer.
pub(super) fn arm_debounced(
    id: &str,
    get_bytes: GetBytes,
    is_cancelled: IsCancelled,
    content_probe: Option<ContentProbe>,
    delay_ms: i32,
) {
    install_pending(
        id,
        PendingSave {
            get_bytes,
            is_cancelled,
            owner: owner_token(),
            content_probe,
        },
        delay_ms,
    );
}

/// Park one [`PendingSave`] under `id` and (re)start its debounce window.
///
/// Reinstalls a pending save that [`run_save`] has declined
/// to write straight back where it came from. That re-arm keeps the **original** `owner` rather
/// than resolving a fresh one, which is the  contract restated: a deferred write commits to
/// the account it was *armed* under or to nothing at all, and a read-only tab waiting for the
/// writer role must not quietly re-file its pending under whoever signs in next.
pub(super) fn install_pending(id: &str, pending: PendingSave, delay_ms: i32) {
    let id_owned = id.to_string();
    PENDING.with(|p| {
        p.borrow_mut().insert(id_owned.clone(), pending);
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

/// Register the flush-on-hide listeners (React ): `visibilitychange` → flush when the document
/// is hidden, and `pagehide` → flush. Both closures leak like the editor's wheel/pan handlers (the
/// doc + engine leak too; `on_cleanup` is `Send`-bound and can't hold them).
pub fn register_flush_on_hide(mission_id: String) {
    save_status::bind_runtime();
    save_status::set_retry_handler(|| {
        spawn_local(async move { retry_unreadable_keys().await });
    });

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

    // pagehide stays fire-and-forget (`spawn_local`, never awaited in the handler). A concurrent
    // hidden flush cannot double-save the same pending: `flush_state` takes it out of `PENDING`
    // synchronously, so the second call finds nothing, and where the two writes do overlap
    // `lock_for(id)` serializes them. `SAVE_IN_FLIGHT` guards peer pulls.
    //
    // announce departure on the same event, so the surviving tabs re-elect at once instead
    // of waiting for the Web Lock release to propagate. Synchronous and best-effort by design:
    // `postMessage` on a channel that is about to die either goes out or does not, and the lock
    // release is the guarantee behind it either way.
    let id = mission_id;
    let on_hide = Closure::<dyn FnMut()>::new(move || {
        tab_lock::leave();
        let id = id.clone();
        spawn_local(async move { flush_state(&id).await });
    });
    let _ = win.add_event_listener_with_callback("pagehide", on_hide.as_ref().unchecked_ref());
    on_hide.forget();
}

/// Retries keys whose stored records were unreadable during this page lifetime.
pub(super) async fn retry_unreadable_keys() {
    let keys: Vec<String> = UNREADABLE.with(|u| u.borrow().iter().cloned().collect());
    if keys.is_empty() {
        return;
    }
    save_status::report_saving();
    let mut still_locked = false;
    for key in keys {
        if note_unreadable(&key).await.is_none() && UNREADABLE.with(|u| u.borrow().contains(&key)) {
            still_locked = true;
        }
    }
    if !still_locked {
        save_status::report_saved();
    }
}

/* ───────────────────────────── smoke bridge ───────────────────────────── */

/// Wrap a Rust future as a JS `Promise` WITHOUT `wasm-bindgen-futures`: the executor spawns the
/// future and resolves once it completes (only `js-sys` + `leptos::task::spawn_local`). The executor
/// is `FnMut` but runs once  `Option::take` yields the future exactly once.
pub(super) fn spawn_promise<F>(fut: F) -> js_sys::Promise
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

/// schedule an **edit-driven** persist after a mutator (the first real doc change; the
/// S8 hook .17/.18 deferred). Re-arms the SAME debounced + serialized writer the boot seam uses
/// (`mission_editor.rs` initial persist): `get_bytes` reads `encode_state()` at write time, the
/// write is cancelled once the doc `Option` clears (route leave). A burst of edits within
/// [`debounce_ms`] coalesces into one IDB write. Bumps [`EDIT_PERSIST_COUNT`] for the gate.
/// this path supplies a [`ContentProbe`], so the content guard on the **per-edit** writer is
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

/// The number of edit-driven persists scheduled this page lifetime (). Exposed on the
/// `__missionPersist` bridge so the gate can prove a move re-armed the writer.
#[must_use]
pub fn edit_persist_count() -> u32 {
    EDIT_PERSIST_COUNT.with(Cell::get)
}
