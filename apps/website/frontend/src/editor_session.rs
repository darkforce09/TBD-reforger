//! T-159.17 — warm editor-session marker (`sessionStorage`) for the Leptos Mission Creator editor.
//!
//! Byte-for-byte parity port of the React `editorSession.ts` **warm-session** marker (T-062.2): a
//! single `sessionStorage["tbd-editor-session"]` record so a same-tab return knows the local doc is
//! warm (the gate that — in a later slice with server hydrate — skips the multi-MB `GET
//! /missions/:id`). This slice ships only the marker read/write/clear + TTL; the server-skip wiring
//! is a T-159.17 non-goal.
//!
//! Scope: the warm-session half of `editorSession.ts`. The separate localStorage "adopted-server"
//! marker (`tbd-editor-adopted:*`, the T-130.5 conflict path) was ported at T-159.26, **emptied at
//! T-352**, and its last remnant — the `mark_adopted` shim and its eight dead call sites — **deleted
//! at T-370**. All that survives of it is [`purge_legacy_markers`], which erases what earlier builds
//! wrote into users' browsers. Whole module is `wasm32`-gated in `main.rs`.
//!
//! **T-352 — neither key was account-scoped, and they needed opposite fixes.** Both held per-account
//! editor state under a global key, the class of bug T-221 and T-338 spent two slices closing for the
//! IndexedDB records and the snapshot cache. What separates them is whether anything *reads* them:
//!
//!   * `tbd-editor-adopted:<id>` (localStorage) had **no reader at all** — T-223 replaced the marker
//!     test with a content comparison and left only the writes. Scoping it would have implied a reader
//!     exists and invited someone to add one, so the storage is gone, and T-370 removed the writes
//!     too; only [`purge_legacy_markers`] remains, to clean up what earlier builds wrote.
//!   * `tbd-editor-session` (sessionStorage) **is** read, by [`read_warm`] through the
//!     `__missionPersist.warm()` bridge, so deleting it would break a live reader. It is scoped
//!     instead — see [`session_key`].
#![allow(dead_code)] // read_warm is exercised via the `__missionPersist` smoke bridge, not Rust callers yet.

use serde::{Deserialize, Serialize};

/// The **logical** sessionStorage key — the React `SESSION_KEY`. Singleton (one record; last write
/// across missions wins), exactly as `editorSession.ts`. No longer a physical key (T-352 — see
/// [`session_key`]); kept as the scoped suffix, and to recognise what a pre-T-352 build left behind.
const SESSION_KEY: &str = "tbd-editor-session";

/// The physical sessionStorage key for the signed-in account — `u{len}:{owner}|{logical}`, the scheme
/// T-221 established for the IndexedDB records and T-338 extended to the snapshot RAM cache.
///
/// **Why this one is scoped rather than deleted (T-352).** The record holds
/// `{missionId, readyAt, slotCount, currentSemver}` — per-account editor state — under a global key.
/// `sessionStorage` is scoped to the *tab*, not to the session, so it outlives a client-side sign-out:
/// A opens the editor, signs out, B signs in **in that same tab**, and A's mission id and object count
/// are still on record. Unlike the `tbd-editor-adopted:*` marker removed below, this one genuinely **is**
/// read — [`read_warm`], via `yrs_persist`'s `__missionPersist.warm()` bridge — so deleting it would
/// break a live reader. Scope it instead.
///
/// The length prefix is not decoration: it makes `(owner, logical) → key` **injective** for arbitrary
/// owner bytes. A plain `{owner}|{logical}` join collides the moment an id contains the separator, and a
/// `discord_id` is whatever the backend sends, not a shape this module gets to assume. The format is
/// restated here rather than shared because `yrs_persist::scoped_key` is private;
/// [`crate::yrs_persist::owner_token`] is `pub` (T-338) and is the one token every per-account cache has
/// to agree on.
///
/// Scoping alone closes the hole — B resolves a different physical key and cannot even name A's record —
/// so no sign-out purge is needed, which keeps this clear of the T-338 ordering trap (a token resolved
/// *after* `clear_session` is `anon`, i.e. the wrong owner at exactly the wrong moment). Every caller
/// goes through this module's three functions, so nothing outside it changed.
fn session_key() -> String {
    let owner = crate::yrs_persist::owner_token();
    format!("u{}:{owner}|{SESSION_KEY}", owner.len())
}

/// 24h in ms — the React `TTL_MS = 24 * 60 * 60 * 1000`.
const TTL_MS: f64 = 24.0 * 60.0 * 60.0 * 1000.0;

/// The persisted warm-session record. Field names serialize to the EXACT React shape
/// `{ missionId, readyAt, slotCount, currentSemver }` (the V-gate parity contract). `readyAt` is a
/// `Date.now()` epoch-ms value; `currentSemver` is `null` this slice (no server semver yet).
#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorSession {
    pub mission_id: String,
    pub ready_at: f64,
    pub slot_count: u32,
    pub current_semver: Option<String>,
}

/// Write the warm marker after the doc is ready (React `markEditorSessionReady`). Silent no-op on
/// any storage failure (private-mode / quota / serialize) — matching the React try/catch.
pub fn mark_ready(mission_id: &str, slot_count: u32, current_semver: Option<String>) {
    let session = EditorSession {
        mission_id: mission_id.to_string(),
        ready_at: js_sys::Date::now(),
        slot_count,
        current_semver,
    };
    if let (Some(storage), Ok(json)) = (
        web_sys::window().and_then(|w| w.session_storage().ok().flatten()),
        serde_json::to_string(&session),
    ) {
        let _ = storage.set_item(&session_key(), &json);
        // Drop the pre-T-352 unscoped record while we are here. `read_warm` reads the scoped key now,
        // so the old one is unreachable — but leaving it would park A's missionId/slotCount in a tab
        // B may already be using, which is the whole point of scoping. Same reasoning as
        // `purge_legacy_markers`, one storage down.
        let _ = storage.remove_item(SESSION_KEY);
    }
}

/// Read the warm marker for `mission_id` (React `readWarmEditorSession`). Returns `None` when the
/// record is absent / for a different mission / stale (`Date.now() - readyAt > TTL_MS`, strict `>`)
/// / unparseable — the four React guards, in order. Any failure short-circuits to `None`.
#[must_use]
pub fn read_warm(mission_id: &str) -> Option<EditorSession> {
    let storage = web_sys::window()?.session_storage().ok()??;
    let json = storage.get_item(&session_key()).ok()??;
    let session: EditorSession = serde_json::from_str(&json).ok()?;
    if session.mission_id != mission_id {
        return None;
    }
    if js_sys::Date::now() - session.ready_at > TTL_MS {
        return None;
    }
    Some(session)
}

/// Clear the warm marker (React `clearEditorSession`). Silent no-op on failure.
pub fn clear() {
    if let Some(storage) = web_sys::window().and_then(|w| w.session_storage().ok().flatten()) {
        let _ = storage.remove_item(&session_key());
        let _ = storage.remove_item(SESSION_KEY); // and the pre-T-352 unscoped record
    }
}

/* ────────── adopted-server marker — REMOVED (T-352; reader removed by T-223) ────────── */

/// The key prefix this module wrote until T-352 (React `tbd-editor-adopted:${missionId}`), kept
/// solely so [`purge_legacy_markers`] can still recognise the residue.
const ADOPTED_KEY_PREFIX: &str = "tbd-editor-adopted:";

/// Erase every `tbd-editor-adopted:*` key written before T-352.
///
/// Two passes, because `Storage::key(i)` is index-addressed and `remove_item` renumbers everything
/// after the hit — a single forward scan that deleted as it went would skip the key following each
/// match. Collect first, delete second.
///
/// **Why this deletes rather than warns.** T-221's legacy-record path warns once and offers adoption
/// instead of deleting, because an unowned IndexedDB record may be the only copy of somebody's
/// document. That reasoning does not transfer here, and the difference is provable rather than a
/// judgement call: this marker has **no reader** (T-223 removed the last one; T-352 confirmed zero
/// repo-wide), so no behaviour anywhere can observe its absence. Its value was a server semver —
/// derived state the server still holds — never authored content. Removal is unobservable by
/// construction. *Keeping* it is not: a `<missionId>` sitting under a global key is precisely the
/// cross-account disclosure T-221 and T-338 spent two slices closing, and it would otherwise persist
/// on every browser that ever opened the editor.
///
/// **Deliberately unguarded** — no once-per-page-load latch. `localStorage` is shared across tabs, so
/// during a deploy rollover a tab still running pre-T-352 wasm can write *fresh* residue at any
/// moment; a latched purge would run once, before that write, and never look again. Re-scanning is
/// affordable because the sole caller is one-shot per editor boot (never a loop), so this is a prefix
/// scan of a few dozen keys once per mission open.
///
/// **T-388 / T-370 — this is now the only path to the residue, and it has exactly one caller.**
/// T-352 hung the purge off `mark_adopted` because that slice did not own the files the call sites
/// live in, and T-388 then measured the consequence: three editor paths reach the document without
/// ever calling `mark_adopted` (the `is_empty && loaded_from_idb` branch, `Local::Diverged`, and
/// every boot whose server fetch fails — offline, unauthenticated, 404), so clearance was *eventual*
/// rather than guaranteed. Wave 81 moved the call to `mission_hydrate::hydrate_from_server`, before
/// its `is_uuid` guard and before the fetch, so it runs on every editor boot on every branch; T-370
/// then deleted the eight dead `mark_adopted` writes and the shim, which is why nothing else calls
/// this. **Keep it that way:** if that one call ever moves below a branch or a `return`, residue
/// stops being cleared and there is no longer a second path that would eventually catch it.
pub fn purge_legacy_markers() {
    let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) else {
        return;
    };
    let stale: Vec<String> = (0..storage.length().unwrap_or(0))
        .filter_map(|i| storage.key(i).ok().flatten())
        .filter(|k| k.starts_with(ADOPTED_KEY_PREFIX))
        .collect();
    for key in &stale {
        let _ = storage.remove_item(key);
    }
}
