//! T-159.17 — warm editor-session marker (`sessionStorage`) for the Leptos Mission Creator editor.
//!
//! Byte-for-byte parity port of the React `editorSession.ts` **warm-session** marker (T-062.2): a
//! single `sessionStorage["tbd-editor-session"]` record so a same-tab return knows the local doc is
//! warm (the gate that — in a later slice with server hydrate — skips the multi-MB `GET
//! /missions/:id`). This slice ships only the marker read/write/clear + TTL; the server-skip wiring
//! is a T-159.17 non-goal.
//!
//! Scope: only the warm-session half of `editorSession.ts`. The separate localStorage
//! "adopted-server" marker (`tbd-editor-adopted:*`, the T-130.5 conflict path) is deliberately NOT
//! ported — server hydrate/conflict is out of scope. Whole module is `wasm32`-gated in `main.rs`.
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

/* ─────────────── adopted-server marker (T-159.26 — the T-130.5 conflict path) ─────────────── */

/// localStorage key per mission — React `tbd-editor-adopted:${missionId}`. Records the server
/// semver the LOCAL doc currently derives from, so a new-tab cold boot whose IDB matches that exact
/// version trusts local (the delta is the user's own unsaved edits) instead of re-prompting the
/// conflict.
fn adopted_key(mission_id: &str) -> String {
    format!("tbd-editor-adopted:{mission_id}")
}

/// Mark the local doc as derived from `semver` (React `markServerVersionAdopted`) — after an
/// initial hydrate, a "load server" resolution, or our own Save. `None` clears it.
pub fn mark_adopted(mission_id: &str, semver: Option<&str>) {
    if let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
        match semver {
            Some(s) => {
                let _ = storage.set_item(&adopted_key(mission_id), s);
            }
            None => {
                let _ = storage.remove_item(&adopted_key(mission_id));
            }
        }
    }
}

/// Read the adopted server semver for `mission_id` (React `readAdoptedServerVersion`).
#[must_use]
pub fn read_adopted(mission_id: &str) -> Option<String> {
    let storage = web_sys::window()?.local_storage().ok()??;
    storage.get_item(&adopted_key(mission_id)).ok()?
}
