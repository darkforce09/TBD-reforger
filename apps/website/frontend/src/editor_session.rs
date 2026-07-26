//! T-159.17 — warm editor-session marker (`sessionStorage`) for the Leptos Mission Creator editor.
//!
//! Byte-for-byte parity port of the React `editorSession.ts` **warm-session** marker (T-062.2): a
//! single `sessionStorage["tbd-editor-session"]` record so a same-tab return knows the local doc is
//! warm (the gate that — in a later slice with server hydrate — skips the multi-MB `GET
//! /missions/:id`). This slice ships only the marker read/write/clear + TTL; the server-skip wiring
//! is a T-159.17 non-goal.
//!
//! Scope: the warm-session half of `editorSession.ts`. The separate localStorage "adopted-server"
//! marker (`tbd-editor-adopted:*`, the T-130.5 conflict path) was ported at T-159.26 and **removed
//! again at T-352** — see [`mark_adopted`]. Whole module is `wasm32`-gated in `main.rs`.
#![allow(dead_code)] // read_warm is exercised via the `__missionPersist` smoke bridge, not Rust callers yet.

use serde::{Deserialize, Serialize};

/// sessionStorage key — identical to the React `SESSION_KEY`. Singleton (one record; last write
/// across missions wins), exactly as `editorSession.ts`.
const SESSION_KEY: &str = "tbd-editor-session";

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
        let _ = storage.set_item(SESSION_KEY, &json);
    }
}

/// Read the warm marker for `mission_id` (React `readWarmEditorSession`). Returns `None` when the
/// record is absent / for a different mission / stale (`Date.now() - readyAt > TTL_MS`, strict `>`)
/// / unparseable — the four React guards, in order. Any failure short-circuits to `None`.
#[must_use]
pub fn read_warm(mission_id: &str) -> Option<EditorSession> {
    let storage = web_sys::window()?.session_storage().ok()??;
    let json = storage.get_item(SESSION_KEY).ok()??;
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
        let _ = storage.remove_item(SESSION_KEY);
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
/// affordable because every [`mark_adopted`] call site is one-shot per hydrate decision or per Save
/// (never a loop), so this is a prefix scan of a few dozen keys a handful of times per mission open.
fn purge_legacy_markers() {
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

/// Formerly wrote the adopted-server marker (React `markServerVersionAdopted`); **now writes nothing
/// and cleans up after its own history.**
///
/// T-223 replaced the marker with a content test (`mission_hydrate::classify_local` — "would adopting
/// this payload change the document?") and left the *writes* in place, so eight call sites went on
/// maintaining a value nothing consulted, under the one key in the editor still not account-scoped.
/// T-352 grepped for readers and found none, which makes scoping the wrong fix: there is nothing to
/// scope for, and the correct move is for the state to stop existing.
///
/// The signature survives because the call sites live in `mission_hydrate` / `mission_commands`, which
/// this slice does not own. They are dead and should go with whoever next touches those files. Until
/// then this is the right place to hang [`purge_legacy_markers`]: every path that used to *create*
/// residue now *erases* it, so the browsers that have any are reached without a new boot hook.
pub fn mark_adopted(_mission_id: &str, _semver: Option<&str>) {
    purge_legacy_markers();
}
