//! Role: reconcile the mission the server holds with the draft this browser holds, and offer the
//! way back from every replacement that reconciliation performs.
//! Position: `editor/state` in the frontend.
//! Signals & state: the measured document fetch, the conflict and semver signals, the toasts, the
//! in-session snapshot cache, and the identity of the editor mount the recovery surface is bound to.
//! Invariants: every DECISION this file used to make now lives in `map-engine`'s
//! `editing::persist`, where it is answerable with no browser; what stays here is the transport and
//! the session — the authed GET, the record store, the signals, and the state that dies with the
//! tab.
//!
//! # The reconciliation, in order
//!
//! A boot fetches `GET /missions/:id` and asks one question of the two documents: **would adopting
//! the server payload change the local draft?** Empty local adopts silently, an identical local
//! trusts itself and is provably clean, and a genuine divergence raises the prompt. Nothing here
//! consults a version number: a marker is evidence about a *number* standing in for evidence about
//! *content*, and it is wrong in both directions — a missing one asks the operator to choose
//! between two identical documents, and a matching one vouches for local work it has never seen.
//! [`purge_legacy_markers`] runs once per boot to erase what earlier builds left behind, before any
//! branch and before the fetch, so a network failure cannot strand residue in a browser.
//!
//! # Two independent ways back from an adopt
//!
//!   1. **Undo.** A conflict adopt runs as a local edit, so its single transaction is exactly one
//!      undo step and one keypress puts the document back in-session. It covers every root the
//!      editor can author into, and not the four roots the undo drive does not scope.
//!   2. **The snapshot pair.** The whole document is encoded *before* the adopt and written under
//!      its own suffixed key, so the debounced draft write cannot reach it and it outlives a reload
//!      — which is exactly when somebody reaches for it. It covers all nine roots.
//!
//! The pair is a pair and not a stack: [`restore_snapshot`] banks whatever it displaces in the
//! counterpart slot, so [`restore_local_backup`] and [`undo_local_restore`] are exact inverses and
//! neither record is consumed by reading it. [`clear_local_backups`] is the only expiry, hung on a
//! successful save — the one moment the document in front of the operator is an immutable server
//! version and the snapshots stop being anybody's last copy.
//!
//! # Account scoping, on both tiers
//!
//! [`LOCAL_BACKUPS`] sits in front of the stored records and is consulted first, so it carries the
//! owner that captured it and every read tests that owner. Scoped rather than merely cleared on
//! sign-out, because a session that *expires* re-namespaces the records while running no handler at
//! all: an unscoped cache would still hand one account's document to whoever the page belongs to
//! next. [`purge_local_documents`] is the sign-out half — it drops the departing account's
//! snapshots here and deletes every record under its key prefix — and it is the only operation in
//! this file that crosses the account boundary, where it is an account deleting its own. It takes
//! the account id as an argument because the session signals are cleared before it runs; resolved
//! afterwards the token would be the anonymous one, and the purge would delete a signed-out
//! visitor's drafts while leaving the departing account's untouched.
//!
//! A record carrying no owner at all matches no prefix and is therefore neither returned nor
//! destroyed. The explicit orphan adoption surface is the only thing that moves one.
//!
//! [`purge_legacy_markers`]: crate::editor::state::session::purge_legacy_markers
#![cfg(target_arch = "wasm32")]

use std::cell::RefCell;
use std::rc::Rc;

use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::prelude::*;
use website_map_engine::data::store::MissionDocCore;
use website_map_engine::editing::persist::local_versus_server::{
    classify_local_draft, server_slot_count, LocalDraftVerdict,
};
use website_map_engine::editing::persist::mission_id::is_uuid;
use website_map_engine::editing::persist::record_key::snapshot_key;
use website_map_engine::editing::persist::server_adoption::{
    adopt_payload, apply_row_meta_only, Adopt, RowMeta,
};
use website_map_engine::editing::persist::snapshot_slot::{
    capture_document_snapshot, SnapshotSlot,
};

use crate::editor::state::doc_host::DocHandle;
use crate::editor::state::history::after_local_edit;
use crate::editor::state::tab_lock;
use crate::v2::core::api::dto::MissionDetail;
use crate::v2::core::auth::AuthStore;

/// T-628 — `GET /api/v1/missions/:id`, measured, with [`crate::v2::core::api::client::api_get`] behind it.
///
/// The mission document is the boot bar's first segment and the API sends a `content-length` for
/// it, so it is determinate for the same reason the DEM is: budget from the header, progress from
/// the body's `ReadableStream`. What it is *not* is a second copy of the auth contract. The token is
/// injected and the body is read here; **anything that is not a 2xx — including the 401 that means
/// the access token expired — falls through to `api_get`**, which owns the single-flight refresh and
/// the one retry (`client.rs`). So this adds a fast path and cannot add a second place that spends a
/// refresh token; the worst case is one wasted GET before the real one.
///
/// A response with no `content-length` reports no budget and the segment simply stays at 0 until it
/// is finished — the bar under-claims rather than invents.
async fn get_mission_measured(
    auth: AuthStore,
    path: &str,
    report: &dyn Fn(website_map_engine::streaming::bridge::progress::BootEvent),
) -> Result<MissionDetail, crate::v2::core::api::client::ApiErr> {
    use wasm_bindgen::JsCast;
    use website_map_engine::streaming::bridge::progress::BootEvent;
    use website_map_engine::streaming::bridge::progress::BootSeg;
    use website_map_engine::streaming::bridge::progress::STREAM_REPORT_BYTES;

    let measured = async {
        let token = auth.access_token.get_untracked()?;
        let resp = gloo_net::http::RequestBuilder::new(&format!("/api/v1{path}"))
            .method(gloo_net::http::Method::GET)
            .credentials(web_sys::RequestCredentials::Include)
            .header("Authorization", &format!("Bearer {token}"))
            .build()
            .ok()?
            .send()
            .await
            .ok()?;
        if !(200..300).contains(&resp.status()) {
            return None;
        }
        let budget = resp
            .headers()
            .get("content-length")
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(0);
        report(BootEvent::Budget(BootSeg::Mission, budget));
        let body = resp.body()?;
        let reader: web_sys::ReadableStreamDefaultReader = body.get_reader().unchecked_into();
        let mut out: Vec<u8> = Vec::with_capacity(usize::try_from(budget).unwrap_or(0));
        let mut unreported: u64 = 0;
        loop {
            let chunk = wasm_bindgen_futures::JsFuture::from(reader.read())
                .await
                .ok()?;
            let done = js_sys::Reflect::get(&chunk, &"done".into())
                .ok()
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            if done {
                break;
            }
            let arr: js_sys::Uint8Array = js_sys::Reflect::get(&chunk, &"value".into())
                .ok()?
                .unchecked_into();
            let at = out.len();
            out.resize(at + arr.length() as usize, 0);
            arr.copy_to(&mut out[at..]);
            unreported += u64::from(arr.length());
            if unreported >= STREAM_REPORT_BYTES {
                report(BootEvent::Done(BootSeg::Mission, unreported));
                unreported = 0;
            }
        }
        if unreported > 0 {
            report(BootEvent::Done(BootSeg::Mission, unreported));
        }
        serde_json::from_slice::<MissionDetail>(&out).ok()
    }
    .await;
    match measured {
        Some(d) => Ok(d),
        None => crate::v2::core::api::client::api_get::<MissionDetail>(auth, path).await,
    }
}

/// Fetch `GET /missions/:id` and reconcile it with the just-loaded local doc:
///  * new mission (empty server payload) → apply the row terrain only;
///  * no local content — no IDB record, or one that decodes to an empty document → hydrate the
///    server payload, mark adopted, refresh;
///  * local content a hydrate of the server payload would reproduce exactly → trust local silently;
///  * local content that genuinely differs → set `conflict` so the UI can prompt.
///
/// `loaded_from_idb` is the persist layer's flag. On any non-404 failure it leaves the doc as-is
/// (local-only) — the caller shows no blocking error (the editor is usable on the local copy).
///
/// T-628 — `report` carries the mission segment of the boot bar. The document GET is the one part
/// of this function with a number on it, and it is not a small one: the editor's own payload runs
/// from ~700 B on a fresh mission to the ~142 MB T-060 measured at 367k slots, so a boot bar that
/// treated it as a rounding error would sit full while the operator waited on it. See
/// [`get_mission_measured`].
pub async fn hydrate_from_server(
    doc: DocHandle,
    id: String,
    auth: AuthStore,
    loaded_from_idb: bool,
    current_semver: RwSignal<Option<String>>,
    conflict: RwSignal<Option<crate::editor::mission_editor::ConflictInfo>>,
    report: website_map_engine::streaming::bridge::progress::ProgressFn,
) {
    // The recovery surface is bound on every editor boot, not only when a conflict fires: after a
    // reload the in-session snapshot is gone and the stored record is the only copy, and that
    // reload is exactly when someone reaches for it. It also re-binds the live-editor identity
    // every boot — `doc` is the very handle the mount created — which is what makes the
    // cross-mission refusal in `restore_snapshot` exact rather than best-effort.
    register_mission_backup(id.clone(), &doc);
    // The residue purge, and three properties of THIS position, all three of them the point:
    //   * **before `is_uuid`** — a local-only id returns two lines down, and the residue is one
    //     global key set per browser rather than anything to do with which mission was opened.
    //   * **before the fetch** — so a network failure or an expired session cannot skip it, which
    //     is the open that no other path reaches.
    //   * **before every branch** — so no branch added later can miss it.
    //
    // **Do not move this below a branch, a `return`, or the fetch.** Nothing else clears the
    // residue, so a boot this line misses is a browser that keeps it.
    crate::editor::state::session::purge_legacy_markers();
    if !is_uuid(&id) {
        return;
    }
    let path = format!("/missions/{id}");
    let detail = match get_mission_measured(auth, &path, report.as_ref()).await {
        Ok(d) => d,
        Err((404, _)) => return, // ad-hoc/local-only id — stay local, silently
        Err(_) => {
            crate::v2::core::ui::toast::use_toasts()
                .error("Could not load the saved version — editing your local copy.");
            return;
        }
    };

    // Hand the row to Export before any branch below can return. This is the ONLY place the editor
    // ever sees the author and the player cap, and the compiled export is built from them; every
    // path past here — fresh mission, warm reopen, conflict prompt — is a state in which the author
    // may still hit Export. Recording it once here rather than per-branch is what makes the
    // recorded row's `None` mean exactly what it claims: the row never arrived, not "it arrived
    // down a branch nobody wired".
    crate::editor::state::commands_hotkeys::set_row_meta(&detail);

    let row = row_meta_from_detail(&detail);
    let version = detail.current_version.as_ref();
    let semver = version.map(|v| v.semver.clone());
    current_semver.set(semver.clone());

    // The editor superset lives in `current_version.json_payload`; empty `{}` = a fresh mission.
    let payload = version.map(|v| &v.json_payload);
    let is_empty = payload
        .map(|p| p.as_object().is_none_or(serde_json::Map::is_empty))
        .unwrap_or(true);

    if is_empty {
        // A real mission with no saved version yet. The editor mounts on a fixture seed, so on the
        // FIRST open — no local record — the seed is cleared to match what a save would mean: a
        // save must not round-trip fixture data. A warm reopen keeps the operator's local work.
        if !loaded_from_idb {
            adopt_payload(&doc, "{}", &row, Adopt::Init, &after_local_edit);
            crate::editor::state::history::set_dirty(false);
        } else {
            apply_row_meta_only(&doc, &row);
        }
        return;
    }
    let server = payload.unwrap();
    let payload_json = serde_json::to_string(server).unwrap_or_default();

    if loaded_from_idb {
        // A warm reopen. The decision is what the two documents CONTAIN, and the engine owns it.
        let Some(local) = classify_local_draft(&doc, server, &payload_json) else {
            // No document to classify — the editor unmounted mid-boot and cleared the `Option`.
            // Adopting and prompting would both act on something that is gone.
            return;
        };
        match local {
            // Nothing authored locally: the record decoded to an empty document. That is the cold
            // boot's situation, so it takes the cold boot's treatment — an adopt that is not an
            // undo step (the first undo would otherwise restore an empty document) and no snapshot,
            // because there is nothing to lose.
            LocalDraftVerdict::Empty => {
                adopt_payload(&doc, &payload_json, &row, Adopt::Init, &after_local_edit);
                crate::editor::state::history::set_dirty(false);
            }
            // Local IS the server's document. Nothing to choose between, so nothing to ask —
            // correcting `dirty` is the whole of this branch's work.
            //
            // That clean flag is EARNED rather than assumed. A restore from the local record is
            // marked dirty on arrival because nothing on that path can prove the restored blob was
            // ever saved; here the two documents have been compared and found equal, which is that
            // proof, so a save-then-reopen no longer shows an unsaved dot over a zero delta.
            LocalDraftVerdict::MatchesServer => {
                crate::editor::state::history::set_dirty(false);
            }
            // Two different documents — ask. The price of asking on every real difference is that
            // reopening a tab holding unsaved edits against the current version prompts. That is
            // deliberate: "Keep local" is one click and "Load server" is reversible twice over,
            // while a document silently replaced by one it never derived from is neither.
            LocalDraftVerdict::Diverged => {
                // Describe BOTH options before asking. The counts are the same numbers the
                // classification just compared, so the prompt cannot disagree with the decision
                // that raised it, and the instants are measured rather than guessed: the local one
                // is the write stamp the draft save leaves on the record it actually landed (absent
                // means this browser has no draft stamp, which is the honest answer rather than
                // "now"), and the server one is the version row's own creation time.
                let local_objects = doc.borrow().as_ref().map_or(0, MissionDocCore::slot_count);
                let local_saved = crate::editor::state::persist::draft_written_at(&id).map_or_else(
                    || "not recorded on this browser".to_string(),
                    |at| tab_lock::ago(js_sys::Date::now(), at),
                );
                let server_saved = version.map_or_else(
                    || tab_lock::short_utc(&detail.updated_at),
                    |v| tab_lock::short_utc(&v.created_at),
                );
                conflict.set(Some(crate::editor::mission_editor::ConflictInfo {
                    local_objects,
                    server_objects: server_slot_count(server),
                    local_saved,
                    server_saved,
                    payload_json,
                    semver,
                }));
            }
        }
    } else {
        // Empty local → adopt the server payload (replaces the seed). Cold doc: INIT, no snapshot.
        adopt_payload(&doc, &payload_json, &row, Adopt::Init, &after_local_edit);
        crate::editor::state::history::set_dirty(false);
    }
}

/// The "Load server" resolution: adopt the offered payload, mark clean, clear the prompt.
///
/// This is the only adopt that runs over *live local work*, so it is the only one that takes a
/// [`snapshot_local`] and the only one that is an undo step. Both happen before the prompt signal is
/// cleared, so a failure to encode cannot leave the dialog gone AND the work unrecoverable.
pub fn resolve_conflict_server(
    id: String,
    conflict: RwSignal<Option<crate::editor::mission_editor::ConflictInfo>>,
) {
    if let (Some(c), Some(doc)) = (
        conflict.get_untracked(),
        crate::editor::state::history::doc_handle(),
    ) {
        // A new adopt opens a new restore cycle, so the counterpart slot — which holds whatever
        // document the PREVIOUS restore displaced — is now stale: `undoRestore()` would put a server
        // version from a cycle ago over current work. Drop it before the new pair is written. Safe
        // to delete and only this: `pre-restore` always holds an *adopted server* document, which is
        // one refetch away; `pre-adopt` is local work that exists nowhere else and is never dropped
        // here.
        forget_snapshot(&id, SnapshotSlot::PreRestore);
        // Capture the WHOLE local document before `hydrate` clears it. Synchronous encode (so the
        // bytes are pre-mutation by construction), deferred IDB write, own record key.
        let saved = snapshot_local(&doc, &id, SnapshotSlot::PreAdopt);
        // The payload carries its own map.terrain; the compile drops the title, so leave the
        // existing title untouched (row meta isn't refetched here).
        adopt_payload(
            &doc,
            &c.payload_json,
            &RowMeta::default(),
            Adopt::Undoable,
            &after_local_edit,
        );
        crate::editor::state::history::set_dirty(false);
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

/// The "Keep local" resolution (React `resolveConflict('local')`): local knowingly diverges, so mark
/// it dirty. Clears the conflict signal.
///
/// T-370 — `_mission_id` is now unused. It fed `editor_session::mark_adopted`, which T-352 emptied
/// and T-370 deleted; dropping the parameter would mean editing the caller in `mission_editor.rs`,
/// which this slice does not own. Removing it is a safe follow-up for whoever next holds that file —
/// there is exactly one call site (`mission_editor.rs`, the ConflictDialog's "Keep local" arm).
pub fn resolve_conflict_local(
    _mission_id: String,
    conflict: RwSignal<Option<crate::editor::mission_editor::ConflictInfo>>,
) {
    crate::editor::state::history::set_dirty(true);
    conflict.set(None);
}

/// The mission row, as the editor's document wants it. The row arrives as the API's wire shape and
/// this is the one place that shape is read; `briefing` is the library blurb string on the row, not
/// the per-faction briefing object the payload carries.
fn row_meta_from_detail(d: &MissionDetail) -> RowMeta {
    RowMeta {
        title: d.title.clone(),
        terrain: d.terrain.clone(),
        time_of_day: d.time_of_day.clone(),
        weather: d.weather.clone(),
        briefing: d.briefing.clone().unwrap_or_default(),
    }
}

/// The in-memory half of a snapshot — an instant, IDB-independent restore for the session that took
/// the swap. Keyed by **owner**, mission id and kind: by owner so a change of account can't read the
/// previous account's document (T-338), by mission so navigating to another mission can't restore the
/// wrong doc, by kind so the two slots of the pair can't shadow each other.
struct LocalBackup {
    /// T-338 — the account signed in when this snapshot was captured, resolved by the same
    /// `yrs_persist::owner_token()` that namespaces the IndexedDB record it mirrors (`discord_id`,
    /// or `anon` while signed out).
    ///
    /// It is stored rather than re-derived because the token is a function of *now*: a snapshot
    /// taken by A must stay attributed to A across a sign-out, or the record would silently follow
    /// the page to whoever signs in next — which is the whole defect, one layer up from the IDB
    /// records T-221 scoped.
    owner: String,
    mission_id: String,
    kind: SnapshotSlot,
    bytes: Vec<u8>,
}

/// The editor mount this module's recovery surface is bound to: the mission id, plus the very
/// `DocHandle` `mission_editor::on_load` built for it (the same `Rc` it hands
/// `mission_history::set_ctx`). Re-registered on every boot by [`register_mission_backup`] — see
/// [`live_editor_is`] for what the pair proves and why the id alone is not enough.
struct LiveEditor {
    mission_id: String,
    doc: DocHandle,
}

thread_local! {
    static LOCAL_BACKUPS: RefCell<Vec<LocalBackup>> = const { RefCell::new(Vec::new()) };
    static LIVE_EDITOR: RefCell<Option<LiveEditor>> = const { RefCell::new(None) };
}

/// Write (replacing) the in-memory copy of one slot, under the account signed in **now** (T-338).
///
/// The replace is scoped to that account too, so writing a snapshot cannot evict another account's:
/// the namespaces are independent, exactly as `yrs_persist`'s `(owner, logical) → key` mapping makes
/// them independent on disk. That matters beyond tidiness — an unscoped replace would make the mere
/// *presence* of A's recovery record depend on B's activity, and the one thing this pair must never
/// do is disappear because somebody else touched the machine.
fn remember(mission_id: &str, kind: SnapshotSlot, bytes: Vec<u8>) {
    let owner = crate::editor::state::persist::owner_token();
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
/// T-338 — the owner test is the fix. Without it this lookup answered for whoever last used the page
/// rather than whoever is using it, and because [`has_snapshot`] consults it before the (already
/// scoped) IDB read, it shadowed the scoping T-221 put on the records themselves.
fn recall(mission_id: &str, kind: SnapshotSlot) -> Option<Vec<u8>> {
    let owner = crate::editor::state::persist::owner_token();
    LOCAL_BACKUPS.with(|b| {
        b.borrow()
            .iter()
            .find(|s| s.owner == owner && s.mission_id == mission_id && s.kind == kind)
            .map(|s| s.bytes.clone())
    })
}

/// Drop one slot, in memory and on disk. In-memory first and synchronously, so [`has_snapshot`]
/// tells the truth on the very next line; the IDB delete is deferred and best-effort (a failed
/// delete leaves a stale record — the pre-existing behaviour, not a new failure mode).
///
/// Both halves are scoped to the signed-in account (T-338), and that is the conservative choice
/// rather than the convenient one. Every caller is an act of one account — a Save, or a new conflict
/// cycle — and `clear_state` was already owner-scoped on the IDB side, so an unscoped in-memory
/// delete would have let one account's Save destroy another's last in-session copy. T-221 drew the
/// same line for the same reason (`yrs_persist::clear_state`: records are "dropped by an explicit
/// `adopt_orphans()`, never as a side effect of somebody else's Save"). The one operation allowed to
/// cross the namespace boundary is [`purge_local_documents`], and there the account is deleting its
/// own.
fn forget_snapshot(mission_id: &str, kind: SnapshotSlot) {
    let owner = crate::editor::state::persist::owner_token();
    LOCAL_BACKUPS.with(|b| {
        b.borrow_mut()
            .retain(|s| s.owner != owner || s.mission_id != mission_id || s.kind != kind);
    });
    let key = snapshot_key(mission_id, kind.suffix());
    spawn_local(async move {
        if let Err(e) = crate::editor::state::persist::clear_state(&key).await {
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

/// Expire **every** snapshot on record for `mission_id` — the expiry these records never had.
///
/// Called from `mission_commands::save_now` on a 201, which is the one moment the snapshots stop
/// being anybody's last copy: the document in front of the user is now an immutable server version,
/// and a server version is one refetch away. Before this, nothing deleted `<id>::pre-adopt` at all
/// (grep-verified) — it accumulated one whole-document copy per mission ever conflicted, forever,
/// and `window.__missionBackup.has()` kept answering `true` for a document from weeks ago that a
/// restore would then swap over good current work.
///
/// **The accepted cost:** if the user adopted the server version, kept working on it, and saved,
/// their pre-conflict local work is only in `pre-adopt` and this drops it. That is the deliberate
/// reading of a Save — an explicit act that names one document as the one — and the alternative
/// (never expire) is both the unbounded-growth defect and a live hazard, because the older the
/// record gets the more likely restoring it is the destructive move.
pub fn clear_local_backups(mission_id: &str) {
    forget_snapshot(mission_id, SnapshotSlot::PreAdopt);
    forget_snapshot(mission_id, SnapshotSlot::PreRestore);
}

/// T-338 — the sign-out purge: destroy every local document belonging to `owner`, in RAM and on disk.
///
/// Called from `auth::clear_session` with the `discord_id` read out of the session signal **before**
/// the signals are cleared. That ordering is load-bearing: `yrs_persist`'s token resolves from
/// `localStorage["tbd-auth"]`, which sign-out also clears, so a token resolved after the fact is
/// `anon` — the purge would then delete a signed-out visitor's drafts and leave the departing
/// account's exactly where they were.
///
/// Both halves are needed and neither is redundant. The RAM half is this module's snapshot cache,
/// which lives *in front of* the IDB records and is consulted first ([`has_snapshot`]); the disk half
/// is every record under the owner's key prefix — the live doc plus both snapshot slots, all three by
/// construction, because `yrs_persist::purge_owner` scans a prefix and the T-191 suffixes are logical
/// keys under it. Purging only the disk would leave the in-memory hit shadowing the deletion for the
/// rest of the page load; purging only RAM would leave the documents on the disk of a shared machine.
///
/// This is the **only** operation in this module that deletes across the account boundary, and it is
/// the departing account deleting its own. Unowned pre-scoping records are untouched: they match no
/// owner prefix, so the T-221 orphan contract — never returned, never destroyed, recoverable by an
/// explicit `__missionPersist.adopt_orphans()` — is intact.
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
        let gone = crate::editor::state::persist::purge_owner(&owner).await;
        web_sys::console::log_1(&JsValue::from_str(&format!(
            "[t338] sign-out purge: dropped {dropped} in-memory snapshot(s) and deleted {gone} local record(s)"
        )));
    });
}

/// Store one snapshot of the live document in both tiers — the in-session copy and the record that
/// outlives a reload — and report how many slots went into it.
///
/// The capture itself belongs to the engine: WHEN the encode happens relative to the replacement is
/// the property that makes a snapshot a snapshot, and it is the engine that holds it, running the
/// encode synchronously inside the call before the caller has touched anything. This function is
/// the transport half — where the bytes go, and that the record write is deferred so an adopt is
/// never waiting on a store round-trip.
///
/// A failed record write is non-fatal: the in-session copy, and for a pre-adopt the undo step,
/// still stand.
fn snapshot_local(doc: &DocHandle, mission_id: &str, kind: SnapshotSlot) -> Option<usize> {
    capture_document_snapshot(doc, &|bytes| {
        remember(mission_id, kind, bytes.clone());
        let key = snapshot_key(mission_id, kind.suffix());
        spawn_local(async move {
            if let Err(e) = crate::editor::state::persist::save_state(&key, &bytes).await {
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
/// T-338 — both tiers are account-scoped, so the order is a cache optimisation and nothing more. It
/// used to be the leak: [`recall`] answered for any account, so the fast path could report a document
/// the slow path would (correctly) refuse to return, and `restore()` took the same fast path.
async fn has_snapshot(mission_id: &str, kind: SnapshotSlot) -> bool {
    if recall(mission_id, kind).is_some() {
        return true;
    }
    crate::editor::state::persist::load_state(&snapshot_key(mission_id, kind.suffix()))
        .await
        .is_some_and(|b| !b.is_empty())
}

/// Bind the recovery surface to the editor mount that is booting. Called from
/// [`register_mission_backup`], i.e. once per editor boot, with the `DocHandle` `on_load` created
/// for this mission — so the pair is the live editor by construction, never a stale capture.
fn set_live_editor(mission_id: &str, doc: &DocHandle) {
    LIVE_EDITOR.with(|e| {
        *e.borrow_mut() = Some(LiveEditor {
            mission_id: mission_id.to_string(),
            doc: doc.clone(),
        });
    });
}

/// Is `mission_id` the mission the live editor is showing — and is the document
/// `mission_history::doc_handle()` resolves the one that mission booted with?
///
/// Both halves are load-bearing. [`restore_snapshot`] is not handed a document: it is handed an id,
/// and it asks `HISTORY_CTX` for somewhere to put the bytes. `HISTORY_CTX` follows the LIVE editor
/// and is never cleared, so before this guard a call carrying mission A's id while mission B was
/// open wrote A's entire document into B's `Rc` — and B's own debounced persist then committed it
/// under B's key. Silent, total data loss on a mission the user never even conflicted on.
///
///   * The **id** check catches the stale caller (a `.forget()`'d closure from a previous mount, or
///     the in-product "Undo this" button this slice's surface is a placeholder for).
///   * The **`Rc::ptr_eq`** check catches the window the id alone cannot see: between
///     `mission_history::set_ctx(B)` (synchronous in `on_load`) and this module's re-registration
///     for B (an IDB round-trip later, inside the boot task), the id still reads `A` while the ctx
///     doc is already B's — exactly the case that must be refused. A fresh `DocHandle` per mount
///     makes pointer identity the exact test; the in-place `*doc.borrow_mut() = …` swaps that the
///     IDB restore and this module perform do not disturb it.
fn live_editor_is(mission_id: &str) -> bool {
    let Some(ctx_doc) = crate::editor::state::history::doc_handle() else {
        return false;
    };
    LIVE_EDITOR.with(|e| {
        e.borrow()
            .as_ref()
            .is_some_and(|live| live.mission_id == mission_id && Rc::ptr_eq(&live.doc, &ctx_doc))
    })
}

/// Restore the pre-adopt snapshot over the live document — the "I did not mean that" lever for the
/// conflict adopt. Prefers the in-session copy, falls back to the IDB record. `true` when the
/// document was replaced.
///
/// The snapshot is **not** consumed: after a restore the server version is still one refetch away,
/// while the local work exists nowhere else, so the safer record to keep is this one. What the
/// restore displaces is written to `<id>::pre-restore` first — see [`restore_snapshot`].
pub async fn restore_local_backup(mission_id: String) -> bool {
    restore_snapshot(mission_id, SnapshotSlot::PreAdopt).await
}

/// Undo a [`restore_local_backup`]: put back the (server) document that restore displaced.
///
/// The inverse verb, and the reason a restore is now as reversible as the adopt it recovers from.
/// It is a true inverse, not a rollback — it snapshots the document *it* displaces into
/// `<id>::pre-adopt` on the way through, so a user who restores, edits for an hour and then changes
/// their mind again does not lose the hour.
pub async fn undo_local_restore(mission_id: String) -> bool {
    restore_snapshot(mission_id, SnapshotSlot::PreRestore).await
}

/// The shared body of both restore verbs: refuse unless this is the live editor's own mission, swap
/// the requested snapshot in as a fresh core, and bank whatever that swap displaced in the
/// counterpart slot.
async fn restore_snapshot(mission_id: String, want: SnapshotSlot) -> bool {
    // The mismatch says so, loudly, on both channels — a silent `false` here is indistinguishable
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
        None => {
            crate::editor::state::persist::load_state(&snapshot_key(&mission_id, want.suffix()))
                .await
                .unwrap_or_default()
        }
    };
    if bytes.is_empty() {
        return false;
    }
    let Some(doc) = crate::editor::state::history::doc_handle() else {
        return false;
    };
    // Rebuild as a FRESH core and swap, exactly like the boot IDB restore. Applying the update over
    // the live doc would MERGE the two states — yrs is a CRDT, and replaying an old update can
    // never delete the rows the live state inserted — which is the one thing a restore must not do.
    let fresh = MissionDocCore::new();
    fresh.set_origin_init(true);
    let ok = fresh.apply_update(&bytes).is_ok();
    fresh.set_origin_init(false);
    if !ok {
        return false;
    }
    // T-191 fix — bank what this swap is about to destroy BEFORE destroying it. The first pass went
    // straight from here to the swap below, which drops the previous core (and with it the adopt's
    // undo step: a fresh core's stack is empty, so `can_undo()` is false the moment a restore lands)
    // and then lets `schedule_edit_persist` overwrite the plain `<id>` record — the last remaining
    // copy. A restore the user did not mean left them nothing.
    //
    // Placed after the `apply_update` check rather than literally first so a corrupt blob costs
    // nothing; the encode is still pre-swap by construction, because `fresh` is a separate core and
    // nothing has touched `doc` yet.
    let displaced = snapshot_local(&doc, &mission_id, want.counterpart());
    *doc.borrow_mut() = Some(fresh);
    // The local doc no longer derives from the server semver that was adopted. That used to mean
    // clearing the `tbd-editor-adopted:<id>` marker here so the next cold boot would not silently
    // trust local against the wrong version — but T-223 replaced that test with `classify_local`,
    // which compares the two *documents* and cannot be misled by a stale semver, so there is nothing
    // left to clear. `set_dirty(true)` below carries the whole of the signal now.
    // Wholesale document swap: rebind glyphs/HUD/docks (`after_local_edit` would be wrong — it
    // rebinds from a doc it assumes was edited in place), then mark dirty and re-arm the persist so
    // the restored document becomes the local record rather than the displaced one.
    crate::editor::state::history::rebind_engine_from_doc();
    crate::editor::state::history::set_dirty(true);
    crate::editor::state::persist::schedule_edit_persist(doc, &mission_id);
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

/// Toast without `expect_context`. [`restore_snapshot`] can be driven from a JS bridge closure,
/// which has no reactive Owner, and `use_toasts()` would panic there — a panic in the middle of a
/// recovery being the worst possible time for one.
fn notify(msg: &str) {
    if let Some(toasts) = use_context::<crate::v2::core::ui::toast::Toasts>() {
        toasts.message(msg);
    }
}

/// Install `window.__missionBackup` — the recovery surface for the snapshot pair, and the peer of
/// `__missionDoc` / `__missionPersist` / `__editorHistory` (a `js_sys::Object` of `.forget()`'d
/// closures). Four Promise-returning verbs, two symmetric halves:
///   * `has()`           → bool — is a pre-adopt snapshot on record for this mission?
///   * `restore()`       → bool — swap it back over the live document.
///   * `hasUndoRestore()`→ bool — is the document a restore displaced still on record?
///   * `undoRestore()`   → bool — swap *that* back; the exact inverse of `restore()`.
///
/// Unlike its read-only peers this one mutates, on purpose: a backup nobody can restore is not a
/// backup. It is also the only surface this slice can offer — the conflict modal lives in
/// `mission_editor.rs`, which another slice owns, so the in-product buttons are a follow-up that can
/// call [`restore_local_backup`] / [`undo_local_restore`] directly. Both refuse a mission that is
/// not the live one, so that follow-up cannot reintroduce the cross-mission write.
fn register_mission_backup(mission_id: String, doc: &DocHandle) {
    set_live_editor(&mission_id, doc);
    let obj = js_sys::Object::new();

    let has_fn = {
        let id = mission_id.clone();
        Closure::wrap(Box::new(move || -> JsValue {
            let id = id.clone();
            wasm_bindgen_futures::future_to_promise(async move {
                Ok(JsValue::from_bool(
                    has_snapshot(&id, SnapshotSlot::PreAdopt).await,
                ))
            })
            .into()
        }) as Box<dyn FnMut() -> JsValue>)
    };
    let restore_fn = {
        let id = mission_id.clone();
        Closure::wrap(Box::new(move || -> JsValue {
            let id = id.clone();
            wasm_bindgen_futures::future_to_promise(async move {
                Ok(JsValue::from_bool(restore_local_backup(id).await))
            })
            .into()
        }) as Box<dyn FnMut() -> JsValue>)
    };
    let has_undo_fn = {
        let id = mission_id.clone();
        Closure::wrap(Box::new(move || -> JsValue {
            let id = id.clone();
            wasm_bindgen_futures::future_to_promise(async move {
                Ok(JsValue::from_bool(
                    has_snapshot(&id, SnapshotSlot::PreRestore).await,
                ))
            })
            .into()
        }) as Box<dyn FnMut() -> JsValue>)
    };
    let undo_restore_fn = Closure::wrap(Box::new(move || -> JsValue {
        let id = mission_id.clone();
        wasm_bindgen_futures::future_to_promise(async move {
            Ok(JsValue::from_bool(undo_local_restore(id).await))
        })
        .into()
    }) as Box<dyn FnMut() -> JsValue>);

    let _ = js_sys::Reflect::set(&obj, &JsValue::from_str("has"), has_fn.as_ref());
    let _ = js_sys::Reflect::set(&obj, &JsValue::from_str("restore"), restore_fn.as_ref());
    let _ = js_sys::Reflect::set(
        &obj,
        &JsValue::from_str("hasUndoRestore"),
        has_undo_fn.as_ref(),
    );
    let _ = js_sys::Reflect::set(
        &obj,
        &JsValue::from_str("undoRestore"),
        undo_restore_fn.as_ref(),
    );
    if let Some(win) = web_sys::window() {
        let _ = js_sys::Reflect::set(&win, &JsValue::from_str("__missionBackup"), &obj);
    }
    // Read across the page lifetime; leak like every other editor bridge.
    has_fn.forget();
    restore_fn.forget();
    has_undo_fn.forget();
    undo_restore_fn.forget();
}

// T-505 / T-522 / T-554 Class-R live in `mission_title_prefer` so they run on native
// `cargo test -p website-frontend` (this file is `#![cfg(target_arch = "wasm32")]`).
// T-554 pins both briefing Option wires into apply_row_meta (W62: None at both sites
// stayed green on website-frontend until this ratchet).
