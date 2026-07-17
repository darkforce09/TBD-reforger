//! T-159.26 — server hydrate / conflict / dirty (the useMissionEditor `onSynced` + `resolveConflict`
//! port). The **data-safety** slice: before this the editor opened every real mission on the fixed
//! 8-slot seed, so a Save would overwrite the server version with seed data. Now a real (UUID)
//! mission's `current_version.json_payload` is fetched and hydrated into the doc (replacing the
//! seed), with a Keep-local / Load-server prompt when local IDB content genuinely diverges.
//!
//! **Gate safety:** the whole path is skipped for a non-UUID id (the gate route is
//! `/missions/smoke/edit`), so the 12 editor smokes — which all run on `smoke` — are untouched.
#![cfg(target_arch = "wasm32")]

use leptos::prelude::*;
use map_engine_core::doc::MissionDocCore;

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
///  * empty local (no IDB content) → hydrate the server payload, mark adopted, refresh;
///  * local content that derives from this exact server semver → trust local silently;
///  * genuinely divergent local content → set `conflict` so the UI can prompt.
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
            adopt_payload(&doc, "{}", &row);
            crate::editor_session::mark_adopted(&id, semver.as_deref());
            crate::mission_history::set_dirty(false);
        } else {
            apply_row(&doc, &row);
        }
        return;
    }
    let payload_json = serde_json::to_string(payload.unwrap()).unwrap_or_default();

    if loaded_from_idb {
        // New-tab / warm cold boot: if local derives from this exact server version, the delta is
        // the user's own unsaved edits — trust local. Otherwise prompt.
        if let (Some(adopted), Some(sv)) = (crate::editor_session::read_adopted(&id), &semver) {
            if &adopted == sv {
                return;
            }
        }
        conflict.set(Some(crate::mission_editor::ConflictInfo { payload_json, semver }));
    } else {
        // Empty local → adopt the server payload (replaces the seed).
        adopt_payload(&doc, &payload_json, &row);
        crate::editor_session::mark_adopted(&id, semver.as_deref());
        crate::mission_history::set_dirty(false);
    }
}

/// The "Load server" conflict resolution (React `resolveConflict('server')`): hydrate the offered
/// payload, adopt it, and mark clean. Clears the conflict signal.
pub fn resolve_conflict_server(
    id: String,
    conflict: RwSignal<Option<crate::mission_editor::ConflictInfo>>,
) {
    if let (Some(c), Some(doc)) = (conflict.get_untracked(), crate::mission_history::doc_handle()) {
        // The payload carries its own map.terrain; the compile drops the title, so leave the
        // existing title untouched (row meta isn't refetched here).
        adopt_payload(&doc, &c.payload_json, &RowMeta::default());
        crate::editor_session::mark_adopted(&id, c.semver.as_deref());
        crate::mission_history::set_dirty(false);
    }
    conflict.set(None);
}

/// The "Keep local" resolution (React `resolveConflict('local')`): local knowingly diverges, so
/// drop the adopted marker and mark dirty. Clears the conflict signal.
pub fn resolve_conflict_local(id: String, conflict: RwSignal<Option<crate::mission_editor::ConflictInfo>>) {
    crate::editor_session::mark_adopted(&id, None);
    crate::mission_history::set_dirty(true);
    conflict.set(None);
}

/// Mission-row fields from `GET /missions/:id` (title/terrain/time/weather) — the `apply_row_meta`
/// input.
#[derive(Default)]
struct RowMeta {
    title: String,
    terrain: String,
    time_of_day: String,
    weather: String,
}
impl RowMeta {
    fn from(d: &MissionDetail) -> Self {
        Self {
            title: d.title.clone(),
            terrain: d.terrain.clone(),
            time_of_day: d.time_of_day.clone(),
            weather: d.weather.clone(),
        }
    }
    fn is_empty(&self) -> bool {
        self.title.is_empty() && self.terrain.is_empty()
    }
}

/// Hydrate a compiled payload into the doc under INIT, then rebind the engine glyphs + persist via
/// the shared tail. Runs under INIT so the hydrate itself pushes no undo step (it replaces the whole
/// document); `after_local_edit` then rebinds/persists (and marks dirty — the caller clears it).
fn adopt_payload(doc: &DocHandle, payload_json: &str, row: &RowMeta) {
    {
        let guard = doc.borrow();
        let Some(core) = guard.as_ref() else {
            return;
        };
        core.set_origin_init(true);
        core.hydrate(payload_json, DEFAULT_LAYER_ID);
        if !row.is_empty() {
            core.apply_row_meta(&row.title, &row.terrain, opt(&row.time_of_day), opt(&row.weather));
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
        core.apply_row_meta(&row.title, &row.terrain, opt(&row.time_of_day), opt(&row.weather));
        core.set_origin_init(false);
    }
}

fn opt(s: &str) -> Option<String> {
    (!s.is_empty()).then(|| s.to_string())
}

#[allow(dead_code)]
fn _touch(_c: &MissionDocCore) {}
