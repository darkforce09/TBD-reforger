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
//! **T-191 fix pass — the recovery lever was itself a one-way door.** Two defects the first pass
//! shipped, both in the restore half:
//!
//!   * [`restore_local_backup`] swapped in a fresh core and re-armed the persist over the plain
//!     `<id>` record. That dropped the old core (and with it the adopt's undo step — a fresh core's
//!     stack is empty, so `can_undo()` was false the instant a restore landed) and then overwrote
//!     the only remaining copy of the document it had just replaced. A user who restored and was
//!     wrong about it had nothing left: this ticket's own title, one level down. The two snapshot
//!     slots are now a **pair** ([`Snapshot`]) — every swap writes what it displaces into the other
//!     slot, so restore and un-restore are exact inverses and neither record is ever consumed.
//!   * Nothing ever expired `<id>::pre-adopt`, and [`restore_local_backup`] took a `mission_id` on
//!     trust while sourcing the document to overwrite from a never-cleared `HISTORY_CTX` — so a call
//!     carrying mission A's id while mission B was open wrote A's whole document into B. Now
//!     [`clear_local_backups`] expires the records on a successful Save, and every restore refuses
//!     (loudly) unless it is the live editor's own mission ([`live_editor_is`]).
//!
//! **T-223 — the conflict test asks the document, not a marker.** The test was
//! `localStorage["tbd-editor-adopted:<id>"] == current_version.semver`: evidence about a version
//! *number*, standing in for evidence about two documents. It was wrong in both directions and both
//! were live.
//!
//!   * A marker that goes **missing** — cleared site data, a second browser profile, or a mission
//!     whose first open predates its first saved version (that open marks `adopted = None`) —
//!     prompted on a document byte-identical to the server's: a choice with no difference to choose
//!     between, and no diff shown to reveal that.
//!   * A marker that happens to **match** vouched for local content it had never seen. It is one
//!     localStorage key per mission per *browser*, not per document, so two people who both adopted
//!     `1.0.0` and then both edited each took the trust-local branch in silence — precisely the
//!     divergence the prompt exists for. (T-221 is scoping the IndexedDB records to the user this
//!     same wave; the marker is not scoped at all, which widens that hole rather than closing it.)
//!
//! [`classify_local`] replaces the marker with the only question that has an answer: **would
//! adopting this payload change the document?** Empty local → adopt, nothing to lose; identical →
//! no prompt, and a `dirty` flag that is provably clean; different → prompt. The marker is still
//! *written* (Save and both adopt paths keep it current; `resolve_conflict_local` still clears it),
//! but nothing reads it to make this decision any more.
//!
//! **T-338 — the snapshot cache is per-account, and sign-out destroys the account's copies.**
//! T-221 scoped the IndexedDB *records* to the signed-in `discord_id`; it did not scope the RAM cache
//! sitting in front of them. [`LOCAL_BACKUPS`] was keyed by `(mission_id, kind)` alone and nothing
//! ever cleared it, while [`has_snapshot`] consults it **before** the scoped IDB read — so a
//! client-side sign-out followed by a sign-in, all within one page load, left account A's whole
//! document `has()`-visible and `restore()`-able by account B. Measured before the fix:
//! `same_realm_has_after_switch = true`.
//!
//! Two changes, and both are needed because they close different halves of the same path:
//!
//!   1. **[`LocalBackup`] carries the owner** — `yrs_persist::owner_token()` at capture time — and
//!      [`remember`] / [`recall`] / [`forget_snapshot`] all key on it. Scoping rather than merely
//!      clearing on sign-out, because the clear only fires on the one transition a handler can see:
//!      a session that *expires* re-namespaces the IDB records (`load_state` resolves the token per
//!      call) while running no handler at all, so an unscoped RAM cache would still hand A's document
//!      to whoever the page belongs to next. Scoped, the two tiers of [`has_snapshot`] always agree,
//!      which is the property that matters — a `has()` that says yes about bytes a `restore()` may
//!      not read is worse than either answer alone.
//!   2. **[`purge_local_documents`]**, called from `auth::clear_session`, drops the departing
//!      account's snapshots here *and* every IndexedDB record under its owner prefix
//!      (`yrs_persist::purge_owner` — `pub` and ready since T-221 with no caller repo-wide). Scoping
//!      alone would leave the documents on the disk of a shared machine until some later editor boot
//!      ran the eviction backstop, and `yrs_persist`'s own header promised otherwise.
//!
//! `clear_session` captures the `discord_id` **before** it clears the signals; resolved afterwards the
//! token is `anon` and the purge would delete a signed-out visitor's drafts while leaving the
//! departing account's untouched.
//!
//! **What T-338 deliberately does not touch:** the pre-scoping orphan path. An unowned record is
//! still neither returned nor destroyed — [`purge_local_documents`] deletes by owner prefix, and a
//! record that carries no owner matches no prefix, so `__missionPersist.orphans()` /
//! `adopt_orphans()` remain the only way it moves.
//!
//! **Known gap, owned elsewhere:** the conflict modal itself (`mission_editor.rs`) still offers two
//! buttons with no diff and no change count, so the user still chooses blind — T-191 makes the wrong
//! choice survivable and T-223 makes the question a real one, but neither can show the answer.
//!
//! **Gate safety:** the whole path is skipped for a non-UUID id (the gate route is
//! `/missions/smoke/edit`), so the 12 editor smokes — which all run on `smoke` — are untouched.
#![cfg(target_arch = "wasm32")]

use std::cell::RefCell;
use std::rc::Rc;

use leptos::prelude::*;
use leptos::task::spawn_local;
use map_engine_core::doc::MissionDocCore;
use map_engine_core::mission::compile::compile_payload;
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
///  * no local content — no IDB record, or one that decodes to an empty document → hydrate the
///    server payload, mark adopted, refresh;
///  * local content a hydrate of the server payload would reproduce exactly → trust local silently;
///  * local content that genuinely differs → set `conflict` so the UI can prompt.
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
    // that reload is exactly when someone reaches for it. It also re-binds the live-editor identity
    // every boot (`doc` is the very `Rc` `on_load` handed `mission_history::set_ctx`), which is what
    // makes the cross-mission refusal in `restore_snapshot` exact rather than best-effort.
    register_mission_backup(id.clone(), &doc);
    // T-388 / T-370 step 1 — the adoption-residue purge, hung on the editor BOOT rather than on a
    // hydrate decision.
    //
    // `tbd-editor-adopted:<missionId>` is pre-T-352 residue under a global (un-account-scoped) key.
    // T-352 could only reach it through `mark_adopted`, and T-388 measured what that costs: three
    // paths open the editor without ever calling it — the `is_empty && loaded_from_idb` branch
    // below (`apply_row`, no adopt), `Local::Diverged` (defers to the conflict modal), and every
    // boot whose `GET /missions/:id` never lands, which is any offline / unauthenticated / 404
    // open. Clearance was therefore EVENTUAL: correct on some later hydrate or Save, absent at
    // first open.
    //
    // Three properties of THIS position, and all three are the point:
    //   * **before `is_uuid`** — a `smoke`/`draft` id returns two lines down, and residue does not
    //     care which mission you opened; it is one global key set per browser.
    //   * **before the fetch** — the purge cannot be skipped by a network failure or an expired
    //     session, which is the case T-388 could not otherwise reach.
    //   * **before every branch** — so no future branch can be added that misses it.
    //
    // Called directly rather than through `editor_session::mark_adopted`, and that is deliberate:
    // T-370 deletes the eight dead `mark_adopted` calls (six of them in this file) in a LATER
    // release, and routing the purge through the same shim would put this line in the blast radius
    // of that deletion. It is now the one call site that must survive it.
    crate::editor_session::purge_legacy_markers();
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

    // T-243 — hand the row to the server-truth Export before any branch below can return. This is
    // the ONLY place the editor ever sees `author_id` / `max_players`, and `/compiled` compiles
    // from them; every path past here (fresh mission, warm IDB, conflict prompt) is a state in
    // which the author may still hit Export, so recording it once here rather than per-branch is
    // what makes `mission_commands::ROW_META`'s `None` mean exactly what it claims — the row never
    // arrived, not "it arrived down a branch nobody wired".
    crate::mission_commands::set_row_meta(&detail);

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
    let server = payload.unwrap();
    let payload_json = serde_json::to_string(server).unwrap_or_default();

    if loaded_from_idb {
        // T-223 — new-tab / warm cold boot. The decision is what the two documents CONTAIN; the
        // `adopted` semver marker is no longer consulted (module header for why it was wrong in
        // both directions).
        let Some(local) = classify_local(&doc, server, &payload_json) else {
            // No document to classify — the editor unmounted mid-boot and cleared the `Option`.
            // Adopting and prompting would both act on something that is gone.
            return;
        };
        match local {
            // Nothing authored locally: the IDB record decoded to an empty document. That is the
            // cold boot's situation, so take the cold boot's treatment — adopt under INIT (no undo
            // step, or the first Ctrl+Z would restore an empty document) and take no pre-adopt
            // snapshot, because there is nothing to lose.
            Local::Empty => {
                adopt_payload(&doc, &payload_json, &row, Adopt::Init);
                crate::editor_session::mark_adopted(&id, semver.as_deref());
                crate::mission_history::set_dirty(false);
            }
            // Local IS the server's document. Nothing to choose between, so nothing to ask: re-arm
            // the marker (this branch is what heals one that went missing) and correct `dirty`.
            //
            // That `set_dirty(false)` is new, and it is earned. T-189 marks an IDB restore dirty
            // because nothing on this path could prove the restored blob had ever been saved, and
            // its own comment names the cost: "a save-then-immediately-reopen therefore shows the
            // dot with a zero delta … the adopted marker records a semver, not a document digest,
            // so nothing on this path can tell that case apart". A content test is that digest, so
            // the zero delta is now measured rather than assumed.
            Local::Matches => {
                crate::editor_session::mark_adopted(&id, semver.as_deref());
                crate::mission_history::set_dirty(false);
            }
            // Two different documents — ask. Note this now fires on a case the marker test
            // swallowed: local content that diverges while the marker still names the server's
            // current semver. Catching that (the second module-header defect) has a price, and it
            // is that reopening a tab holding unsaved edits against the current version prompts
            // where it used to pass straight through. Deliberate: "Keep local" is one click and
            // "Load server" is reversible twice over (T-191), while a document silently replaced
            // by one it never derived from is neither.
            Local::Diverged => {
                conflict.set(Some(crate::mission_editor::ConflictInfo {
                    payload_json,
                    semver,
                }));
            }
        }
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
/// a [`snapshot_local`] and the only one that runs [`Adopt::Undoable`]. Both happen before the
/// conflict signal is cleared, so a failure to encode cannot leave the dialog gone AND the work
/// unrecoverable.
pub fn resolve_conflict_server(
    id: String,
    conflict: RwSignal<Option<crate::mission_editor::ConflictInfo>>,
) {
    if let (Some(c), Some(doc)) = (
        conflict.get_untracked(),
        crate::mission_history::doc_handle(),
    ) {
        // A new adopt opens a new restore cycle, so the counterpart slot — which holds whatever
        // document the PREVIOUS restore displaced — is now stale: `undoRestore()` would put a server
        // version from a cycle ago over current work. Drop it before the new pair is written. Safe
        // to delete and only this: `pre-restore` always holds an *adopted server* document, which is
        // one refetch away; `pre-adopt` is local work that exists nowhere else and is never dropped
        // here.
        forget_snapshot(&id, Snapshot::PreRestore);
        // Capture the WHOLE local document before `hydrate` clears it. Synchronous encode (so the
        // bytes are pre-mutation by construction), deferred IDB write, own record key.
        let saved = snapshot_local(&doc, &id, Snapshot::PreAdopt);
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

/* ─────────── T-223 — the content test that replaced the `adopted` semver marker ─────────── */

/// What the local document holds, measured against the server's current version.
///
/// Three states, because the old test only had two and the missing one is where the spurious
/// prompts came from: "local exists" was treated as "local might be lost", when most of the time
/// local *is* the server's document and there is nothing at stake.
enum Local {
    /// [`MissionDocCore::has_content`] is false — no factions, slots, objectives, vehicles or
    /// markers. An IDB record exists but decodes to an empty document, so there is no local work
    /// and no choice to offer.
    Empty,
    /// A hydrate of the server payload reproduces this document exactly: adopting would be a no-op.
    /// Reachable with **no** adopted marker at all, which is the entire point.
    Matches,
    /// The two documents differ. The only state that warrants a prompt.
    Diverged,
}

/// Classify the live document against the server's current version. `server` is the payload as
/// fetched; `payload_json` is that same payload serialized — the exact bytes an adopt would
/// hydrate, so the comparison is against the document the adopt would actually produce.
///
/// `None` means there is no document to classify (the editor unmounted mid-boot and cleared the
/// `Option`) — distinct from [`Local::Empty`], which is a real document that happens to be empty.
///
/// Three tiers, cheapest first, because this runs on every warm boot of every saved mission and the
/// last tier is O(document):
///   1. [`MissionDocCore::has_content`] — O(1), and the only call site this predicate has ever had.
///   2. slot count vs the payload's `editor.slots` length — O(1) on both sides, and it settles the
///      common divergence (something was placed or deleted) without serializing anything. This is
///      what keeps a 367k-slot mission off the deep tier unless it genuinely might be identical.
///   3. compile both documents and compare the authored keys.
fn classify_local(
    doc: &DocHandle,
    server: &serde_json::Value,
    payload_json: &str,
) -> Option<Local> {
    let guard = doc.borrow();
    let core = guard.as_ref()?;
    if !core.has_content() {
        return Some(Local::Empty);
    }
    if core.slot_count() != server_slot_count(server) {
        return Some(Local::Diverged);
    }
    // Compare against the document the adopt WOULD produce, built by running the adopt's own
    // `hydrate` on a throwaway core. Both sides then reach `compile_payload` by the identical path,
    // so nothing about the payload's provenance can register as a difference: row order (`hydrate`
    // keys rows by id and `compile_payload` re-emits them id-sorted, so a payload written by an
    // editor that ordered them differently still matches), the default-layer reseed, the `items`
    // map `hydrate` clears and never loads, and Yjs's integer encoding all line up by construction
    // rather than by a rule restated here and left to drift.
    //
    // INIT origin for the same reason `restore_snapshot`'s fresh core uses it: a LOCAL hydrate
    // pushes an undo step, and yrs keeps in-scope deleted blocks alive for as long as the stack
    // item exists. This core is dropped at the end of the function; it should cost one document,
    // not two.
    let offered = MissionDocCore::new();
    offered.set_origin_init(true);
    offered.hydrate(payload_json, DEFAULT_LAYER_ID);
    offered.set_origin_init(false);
    let mine = compile_payload(&core.small_maps_json(), &core.slots_json(), false);
    let theirs = compile_payload(&offered.small_maps_json(), &offered.slots_json(), false);
    Some(if same_authored_content(&mine, &theirs) {
        Local::Matches
    } else {
        Local::Diverged
    })
}

/// How many slots the server payload carries. Absent / malformed → 0, which is exactly what
/// `hydrate` would make of it.
fn server_slot_count(server: &serde_json::Value) -> usize {
    server
        .pointer("/editor/slots")
        .and_then(serde_json::Value::as_array)
        .map_or(0, Vec::len)
}

/// The compiled keys that carry **authored** content — the whole of the comparison.
///
/// Deliberately excluded: `map` (terrain plus its derived bounds) and `environment` (time /
/// weather). Those are mission-**row** fields: `GET /missions/:id` supplies them and `apply_row_meta`
/// writes them into the local doc on every boot, *after* the hydrate — so local is expected to hold
/// the row's current values while the payload holds whatever they were when it was saved. Comparing
/// them would turn "somebody changed the mission's weather dropdown" into a data-loss prompt. The
/// adopt half draws the line in the same place, for the same reason: `resolve_conflict_server`
/// adopts with an empty `RowMeta` so the resolution does not touch them either. `schemaVersion` is a
/// constant, and `orbat` is Export-only (both compiles here pass `include_orbat = false`).
const AUTHORED_KEYS: [&str; 5] = ["editor", "loadouts", "objectives", "vehicles", "markers"];

/// Do two compiled payloads carry the same authored document? `serde_json::Value` equality is deep
/// and key-order-independent (serde_json's `Map` is a `BTreeMap`), and `compile_payload` emits the
/// row arrays id-sorted — so this compares content, not bytes.
fn same_authored_content(a: &serde_json::Value, b: &serde_json::Value) -> bool {
    AUTHORED_KEYS.iter().all(|k| a.get(*k) == b.get(*k))
}

/// Mission-row fields from `GET /missions/:id` (title/terrain/time/weather/briefing) — the
/// `apply_row_meta` input. `briefing` is the library blurb STRING (`missions.briefing`), not the
/// per-faction briefing object (T-418).
#[derive(Default)]
struct RowMeta {
    title: String,
    terrain: String,
    time_of_day: String,
    weather: String,
    briefing: String,
}
impl RowMeta {
    fn from(d: &MissionDetail) -> Self {
        Self {
            title: d.title.clone(),
            terrain: d.terrain.clone(),
            time_of_day: d.time_of_day.clone(),
            weather: d.weather.clone(),
            briefing: d.briefing.clone().unwrap_or_default(),
        }
    }
    fn is_empty(&self) -> bool {
        self.title.is_empty() && self.terrain.is_empty() && self.briefing.is_empty()
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
            // T-505 — prefer non-blank payload title over a stale missions-row title.
            // Helper + Class-R live in `mission_title_prefer` so native cold-gate CI can pin this
            // (T-522); do not pass `&row.title` straight into `apply_row_meta`.
            let title = crate::mission_title_prefer::prefer_payload_title(payload_json, &row.title);
            core.apply_row_meta(
                &title,
                &row.terrain,
                opt(&row.time_of_day),
                opt(&row.weather),
                opt(&row.briefing),
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
            opt(&row.briefing),
        );
        core.set_origin_init(false);
    }
}

fn opt(s: &str) -> Option<String> {
    (!s.is_empty()).then(|| s.to_string())
}

/* ─────────── T-191 — pre-adopt / pre-restore local backup + restore ─────────── */

/// Which destructive whole-document replacement a snapshot is the escape hatch from.
///
/// Both records live in the same IndexedDB DB/store as the live doc (`tbd-mission-yrs` /
/// `doc-state`, out-of-line keys) but under a **suffixed** key, which is the whole point: the
/// debounced editor persist re-arms on every swap and rewrites the plain mission id a few seconds
/// later, and that write must not be able to reach either record. A mission id is a UUID
/// (`is_uuid`), so neither suffix can collide with a real one.
///
/// The two are a **pair, not a stack.** Every swap in [`restore_snapshot`] writes the document it
/// displaces into the *other* slot ([`Snapshot::counterpart`]), so restore and un-restore are exact
/// inverses: the door swings both ways however many times it is pushed, and neither record is ever
/// consumed by reading it. This is what the first T-191 pass was missing — it built the escape hatch
/// for the adopt and then made the escape hatch itself a one-way door.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Snapshot {
    /// The local work, captured before the conflict adopt replaces it with the server payload.
    /// Local work exists nowhere else, so this is the record that actually matters — it is never
    /// deleted except by an explicit Save ([`clear_local_backups`]).
    PreAdopt,
    /// The adopted (server) document, captured before a restore replaces it with [`Self::PreAdopt`].
    ///
    /// Cheap to lose relative to its counterpart — a server version is always one refetch away —
    /// which is why this is the slot [`resolve_conflict_server`] is allowed to invalidate when a new
    /// conflict opens a new restore cycle.
    PreRestore,
}

impl Snapshot {
    /// The IDB key suffix. Distinct literals rather than a derived name: these strings are the
    /// on-disk contract for records that are read back after a reload.
    fn suffix(self) -> &'static str {
        match self {
            Self::PreAdopt => "::pre-adopt",
            Self::PreRestore => "::pre-restore",
        }
    }

    /// Human-readable name for the refusal message / warnings.
    fn label(self) -> &'static str {
        match self {
            Self::PreAdopt => "pre-adopt",
            Self::PreRestore => "pre-restore",
        }
    }

    /// The slot a restore of `self` must write its displaced document into — i.e. the source slot of
    /// the inverse verb. `PreAdopt ⇄ PreRestore`.
    fn counterpart(self) -> Self {
        match self {
            Self::PreAdopt => Self::PreRestore,
            Self::PreRestore => Self::PreAdopt,
        }
    }
}

/// IndexedDB record key for one snapshot slot of one mission.
fn backup_key(mission_id: &str, kind: Snapshot) -> String {
    format!("{mission_id}{}", kind.suffix())
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
    kind: Snapshot,
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
fn remember(mission_id: &str, kind: Snapshot, bytes: Vec<u8>) {
    let owner = crate::yrs_persist::owner_token();
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
fn recall(mission_id: &str, kind: Snapshot) -> Option<Vec<u8>> {
    let owner = crate::yrs_persist::owner_token();
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
fn forget_snapshot(mission_id: &str, kind: Snapshot) {
    let owner = crate::yrs_persist::owner_token();
    LOCAL_BACKUPS.with(|b| {
        b.borrow_mut()
            .retain(|s| s.owner != owner || s.mission_id != mission_id || s.kind != kind);
    });
    let key = backup_key(mission_id, kind);
    spawn_local(async move {
        if let Err(e) = crate::yrs_persist::clear_state(&key).await {
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
    forget_snapshot(mission_id, Snapshot::PreAdopt);
    forget_snapshot(mission_id, Snapshot::PreRestore);
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
        let gone = crate::yrs_persist::purge_owner(&owner).await;
        web_sys::console::log_1(&JsValue::from_str(&format!(
            "[t338] sign-out purge: dropped {dropped} in-memory snapshot(s) and deleted {gone} local record(s)"
        )));
    });
}

/// Capture the whole live document **before** a destructive whole-document replacement.
///
/// `encode_state()` is the same v1 update stream the persist layer stores and the boot seam replays
/// (`mission_editor` step 1), so a snapshot is restorable by exactly the path the editor already
/// proves on every warm reload — no new serialization format, no new trust.
///
/// The encode is synchronous and runs before any mutation, so the bytes are pre-swap by
/// construction; only the IDB write is deferred. Returns the slot count captured, or `None` when
/// there was nothing to write (an empty blob would only replace a good record with a bad one — the
/// `yrs_persist::run_save` rule).
fn snapshot_local(doc: &DocHandle, mission_id: &str, kind: Snapshot) -> Option<usize> {
    let (bytes, slots) = {
        let guard = doc.borrow();
        let core = guard.as_ref()?;
        (core.encode_state(), core.slot_count())
    };
    if bytes.is_empty() {
        return None;
    }
    remember(mission_id, kind, bytes.clone());
    let key = backup_key(mission_id, kind);
    spawn_local(async move {
        if let Err(e) = crate::yrs_persist::save_state(&key, &bytes).await {
            // Non-fatal: the in-memory copy (and, for a pre-adopt, the undo step) still stands.
            web_sys::console::warn_1(&JsValue::from_str(&format!(
                "[t191] backup save failed for {key}: {e:?}"
            )));
        }
    });
    Some(slots)
}

/// Is a snapshot of `kind` on record for `mission_id` **for the account signed in now**? Checks the
/// in-session copy first, then IDB (the copy that outlives a reload).
///
/// T-338 — both tiers are account-scoped, so the order is a cache optimisation and nothing more. It
/// used to be the leak: [`recall`] answered for any account, so the fast path could report a document
/// the slow path would (correctly) refuse to return, and `restore()` took the same fast path.
async fn has_snapshot(mission_id: &str, kind: Snapshot) -> bool {
    if recall(mission_id, kind).is_some() {
        return true;
    }
    crate::yrs_persist::load_state(&backup_key(mission_id, kind))
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
    let Some(ctx_doc) = crate::mission_history::doc_handle() else {
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
    restore_snapshot(mission_id, Snapshot::PreAdopt).await
}

/// Undo a [`restore_local_backup`]: put back the (server) document that restore displaced.
///
/// The inverse verb, and the reason a restore is now as reversible as the adopt it recovers from.
/// It is a true inverse, not a rollback — it snapshots the document *it* displaces into
/// `<id>::pre-adopt` on the way through, so a user who restores, edits for an hour and then changes
/// their mind again does not lose the hour.
pub async fn undo_local_restore(mission_id: String) -> bool {
    restore_snapshot(mission_id, Snapshot::PreRestore).await
}

/// The shared body of both restore verbs: refuse unless this is the live editor's own mission, swap
/// the requested snapshot in as a fresh core, and bank whatever that swap displaced in the
/// counterpart slot.
async fn restore_snapshot(mission_id: String, want: Snapshot) -> bool {
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
        None => crate::yrs_persist::load_state(&backup_key(&mission_id, want))
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
    // The local doc no longer derives from the server semver we adopted, so drop the marker or the
    // next cold boot would silently trust local against the wrong version.
    crate::editor_session::mark_adopted(&mission_id, None);
    // Wholesale document swap: rebind glyphs/HUD/docks (`after_local_edit` would be wrong — it
    // rebinds from a doc it assumes was edited in place), then mark dirty and re-arm the persist so
    // the restored document becomes the local record rather than the displaced one.
    crate::mission_history::rebind_engine_from_doc();
    crate::mission_history::set_dirty(true);
    crate::yrs_persist::schedule_edit_persist(doc, &mission_id);
    // Name the way back, for the same reason the adopt names Ctrl/Cmd+Z: a recovery nobody knows
    // about is not a recovery.
    let banked = match displaced {
        Some(n) => format!(" ({n} objects)"),
        None => String::new(),
    };
    notify(&match want {
        Snapshot::PreAdopt => format!(
            "Restored your local copy. The server version it replaced{banked} was backed up — run window.__missionBackup.undoRestore() to put it back."
        ),
        Snapshot::PreRestore => format!(
            "Put the server version back. The local copy it replaced{banked} was backed up — run window.__missionBackup.restore() to return to it."
        ),
    });
    true
}

/// Toast without `expect_context`. [`restore_snapshot`] can be driven from a JS bridge closure,
/// which has no reactive Owner, and `use_toasts()` would panic there — a panic in the middle of a
/// recovery being the worst possible time for one.
fn notify(msg: &str) {
    if let Some(toasts) = use_context::<crate::toast::Toasts>() {
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
                    has_snapshot(&id, Snapshot::PreAdopt).await,
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
                    has_snapshot(&id, Snapshot::PreRestore).await,
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
