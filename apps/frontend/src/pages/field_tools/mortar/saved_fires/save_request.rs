//! Saving the solved fire mission: the request body, the post, and its answer.
//!
//! **Role:** builds the [`FireMissionSave`] body from the solved mission — the pinned catalog
//! version, the resolved positions with their heights and sources, the wind, the burst height,
//! and the page's own solution as `client_solution` — posts it to `POST /api/v1/fire-missions`,
//! and words the answer.
//! **Position:** the save button of the save area (`super::save_area`); the API re-solves the
//! inputs with the same engine and refuses a solution that differs.
//! **Signals & state:** the save status signal the save area owns.
//! **Invariants:** the body is built only from a solved mission, so the inputs and the
//! `client_solution` always describe the same solve; `event_id` is trimmed and a blank one is left
//! out; nothing is posted without an event.

#[cfg(any(target_arch = "wasm32", test))]
use crate::foundation::transport::dto::{
    FireMissionSave, GunPosition, HeightSource, MapPoint, Wind,
};
#[cfg(any(target_arch = "wasm32", test))]
use crate::pages::field_tools::mortar::solve_bridge::SolvedMission;
#[cfg(any(target_arch = "wasm32", test))]
use map_engine::data::scenario::ballistics::fire_mission::HeightSource as EngineHeightSource;

/// The save route, under the API prefix.
#[cfg(target_arch = "wasm32")]
pub(crate) const SAVE_PATH: &str = "/fire-missions";

/// The wire height source of an engine one.
#[cfg(any(target_arch = "wasm32", test))]
fn height_source(source: EngineHeightSource) -> HeightSource {
    match source {
        EngineHeightSource::Dem => HeightSource::Dem,
        EngineHeightSource::Manual => HeightSource::Manual,
    }
}

/// The save body of `solved` against `event_id`.
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) fn save_body(solved: &SolvedMission, event_id: Option<&str>) -> FireMissionSave {
    let inputs = &solved.inputs;
    FireMissionSave {
        event_id: event_id
            .map(str::trim)
            .filter(|id| !id.is_empty())
            .map(str::to_string),
        catalog_id: inputs.catalog_id.clone(),
        catalog_version: inputs.catalog_version,
        weapon_id: inputs.weapon_id.clone(),
        shell_id: inputs.shell_id.clone(),
        charge_rings: inputs.charge_rings,
        target: MapPoint {
            x: inputs.target.x,
            y: inputs.target.y,
            height_m: inputs.target.height_m,
            height_source: height_source(inputs.target.height_source),
        },
        guns: inputs
            .guns
            .iter()
            .map(|gun| GunPosition {
                label: gun.label.clone(),
                x: gun.x,
                y: gun.y,
                height_m: gun.height_m,
                height_source: height_source(gun.height_source),
            })
            .collect(),
        wind: inputs.wind.map(|wind| Wind {
            speed_m_s: wind.speed_m_s,
            from_deg: wind.from_deg,
        }),
        burst_height_m: inputs.burst_height_m,
        target_grid: solved.target_grid.clone(),
        client_solution: solved.solution.clone(),
    }
}

// `Saving` and `Failed` are constructed by the wasm32-only save request; gating them would split
// the type the native status line matches on.
/// Where the last save stands.
#[cfg(any(target_arch = "wasm32", test))]
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SaveStatus {
    /// Nothing saved since the last solve.
    Idle,
    /// The post is in flight.
    Saving,
    /// Stored; the row's `created_at`.
    Saved(String),
    /// Refused or failed; the sentence to show.
    Failed(String),
}

/// The sentence for a refused save: the status, the API's message and its `details.code`.
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) fn save_refusal_text(status: u16, message: Option<&str>, code: Option<&str>) -> String {
    match (status, code) {
        (422, Some("solution_mismatch")) => "Not saved: the server's solution differs from this \
             device's (more than 1 mil or 0.1 s). Reload the page to get the current catalog and \
             solver, then calculate again."
            .to_string(),
        (404, _) => "Not saved: the event or the catalog version no longer exists.".to_string(),
        (401 | 403, _) => "Not saved: sign in again to save fire missions.".to_string(),
        (0, _) => "Not saved: the server could not be reached.".to_string(),
        (_, _) => format!(
            "Not saved ({status}): {}",
            message.unwrap_or("the server refused the fire mission.")
        ),
    }
}

/// The line under the save button.
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) fn save_status_text(status: &SaveStatus) -> Option<String> {
    match status {
        SaveStatus::Idle => None,
        SaveStatus::Saving => Some("Saving…".to_string()),
        SaveStatus::Saved(at) => Some(format!("Saved ({at}).")),
        SaveStatus::Failed(text) => Some(text.clone()),
    }
}

/// Posts `body`; the stored row's `created_at`, or the refusal sentence.
#[cfg(target_arch = "wasm32")]
pub(crate) async fn post_save(
    store: crate::foundation::auth::AuthStore,
    body: &FireMissionSave,
) -> Result<String, String> {
    use crate::foundation::transport::dto::SavedFireMissionAnswer;
    let json = serde_json::to_value(body).map_err(|e| format!("Not saved: {e}"))?;
    crate::foundation::transport::client::api_post_keeping_refusal::<SavedFireMissionAnswer>(
        store, SAVE_PATH, json,
    )
    .await
    .map(|answer| answer.fire_mission.created_at)
    .map_err(|refusal| {
        let code = refusal
            .details
            .as_ref()
            .and_then(|d| d.get("code"))
            .and_then(|c| c.as_str());
        save_refusal_text(refusal.status, refusal.message.as_deref(), code)
    })
}
