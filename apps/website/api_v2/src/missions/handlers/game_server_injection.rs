//! The two surfaces a game server (rather than a person) uses to pick up a mission: the
//! service-token mission listing the in-game admin browser reads, and the admin-triggered staging
//! of a live mission's `mission.json` into the server bridge's pickup directory.

use std::fs;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::Json;
use serde::Serialize;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::administration::models::audit_log::AuditSeverity;
use crate::administration::services::audit_writer::{actor_display_name, write_audit};
use crate::core::application_state::AppState;
use crate::core::error_handling::api_error::ApiError;
use crate::core::middleware::{AdminUser, ServiceAuth};
use crate::missions::models::mission::MissionStatus;
use crate::missions::services::mission_compile::mission_terrain_key;
use crate::missions::services::mission_document::build_mission_doc;
use crate::missions::services::mission_lookup::load_mission;

/// One row of `GET /api/v1/ingest/missions`.
///
/// The field names are **camelCase on purpose** and are NOT the usual snake_case API
/// contract: they are read by `TBD_MissionListEntry`
/// (`apps/mod/tbd-framework/Scripts/Game/TBD/Backend/TBD_MissionListLoader.c:3-9`), and
/// Enfusion's `JsonLoadContext` maps JSON keys onto class fields **by name**. A key the
/// struct does not declare is not an error there — it is simply invisible, so a
/// snake_case `slot_count` would parse to `0` for every mission with no warning
/// anywhere. Renamed explicitly rather than via a container attribute so the coupling is
/// visible on the field that has it.
#[derive(Debug, Serialize)]
pub struct IngestMissionListEntry {
    /// The mission UUID — the id the mod persists via `TBD_BackendConfig.SetMissionId`
    /// and then fetches at `GET /api/v1/missions/{id}/compiled`. NOT the compiled
    /// document's `meta.id`, which is a different (schema-shaped) id space.
    id: String,
    name: String,
    /// The compiled document's `meta.terrain`, from the one shared derivation — see
    /// [`website_map_engine::data::scenario::flatten::mission_terrain_key`].
    terrain: String,
    #[serde(rename = "slotCount")]
    slot_count: i64,
}

/// `GET /api/v1/ingest/missions` — every runnable mission, for the in-game admin browser
/// (service-token tier).
///
/// ── WHY THIS IS NOT `list_missions` ────────────────────────────────────────────────────
/// `mission_library::list_missions` is **owner-scoped**: it takes an `AuthUser` and every branch
/// of `push_filters` binds `me = &user.discord_id` (mine / bookmarked / live-or-my-drafts). A
/// service token is a game server, not a person — it has no "me", so that handler cannot simply be
/// re-tiered. This one applies no owner filter at all, which is the correct answer for a machine
/// that has to be able to run any mission an admin names.
///
/// `slotCount` is the count of PLACED editor slots in the mission's current version, read
/// in SQL (`jsonb_array_length`) rather than by compiling each mission — a compile per row
/// would parse every payload in the library on one request. It is the same array the
/// flatten walks, so `slotCount == 0` predicts exactly the `409 no placed slots` the mod
/// would get from `/compiled`, which is what `TBD_FrameworkManager.SelectMissionByNumber`
/// warns on.
///
/// @route GET /api/v1/ingest/missions
pub async fn ingest_list_missions(
    State(state): State<AppState>,
    _svc: ServiceAuth,
) -> Result<Json<Value>, ApiError> {
    // LEFT JOIN: a mission with no saved version still belongs in the list (the admin can
    // see it exists and that it has 0 slots) — an INNER JOIN would make it vanish silently.
    // The `jsonb_typeof` guard is not decoration: `jsonb_array_length` RAISES on a
    // non-array, which would turn one malformed payload into a 500 for the whole list.
    let rows: Vec<(Uuid, String, String, String, i32)> = sqlx::query_as(
        "SELECT m.id, m.title, m.terrain::text, COALESCE(m.custom_terrain_name, ''), \
                CASE WHEN jsonb_typeof(v.json_payload -> 'editor' -> 'slots') = 'array' \
                     THEN jsonb_array_length(v.json_payload -> 'editor' -> 'slots') \
                     ELSE 0 END \
         FROM missions m \
         LEFT JOIN mission_versions v ON v.id = m.current_version_id \
         WHERE m.deleted_at IS NULL \
         ORDER BY m.title ASC, m.id ASC",
    )
    .fetch_all(&state.pool)
    .await?;

    let missions: Vec<IngestMissionListEntry> = rows
        .into_iter()
        .map(
            |(id, title, terrain, custom, slots)| IngestMissionListEntry {
                id: id.to_string(),
                name: title,
                terrain: mission_terrain_key(&terrain, &custom),
                slot_count: i64::from(slots),
            },
        )
        .collect();

    let count = missions.len();
    Ok(Json(json!({ "missions": missions, "count": count })))
}

/// Staging dir for injected mission.json files (game-server bridge pickup).
const MISSION_STAGE_DIR: &str = "missions";

/// `POST /api/v1/missions/:id/inject` — stage mission.json for the server bridge (admin).
///
/// @route POST /api/v1/missions/:id/inject
pub async fn inject_mission(
    State(state): State<AppState>,
    admin: AdminUser,
    Path(id): Path<String>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let Ok(mid) = Uuid::parse_str(&id) else {
        return Err(ApiError::bad_request("invalid id"));
    };
    let m = load_mission(&state.pool, mid)
        .await?
        .ok_or_else(|| ApiError::not_found("mission not found"))?;
    if m.status != MissionStatus::Live {
        return Err(ApiError::conflict("only live missions can be injected"));
    }
    let doc = build_mission_doc(&state.pool, &m).await?;
    let data = serde_json::to_vec_pretty(&doc)
        .map_err(|_| ApiError::internal("could not build mission.json"))?;
    fs::create_dir_all(MISSION_STAGE_DIR).map_err(|_| ApiError::internal("staging unavailable"))?;
    let path = format!("{MISSION_STAGE_DIR}/{}.mission.json", m.id);
    fs::write(&path, data).map_err(|_| ApiError::internal("could not stage mission"))?;

    let actor = &admin.0.discord_id;
    let actor_name = actor_display_name(&state.pool, actor).await;
    write_audit(
        &state.pool,
        AuditSeverity::Info,
        Some(actor),
        &actor_name,
        "mission.inject",
        &format!(
            "{actor_name} injected mission '{}' to the server staging directory",
            m.title
        ),
        "mission",
        &m.id.to_string(),
    )
    .await;
    Ok((
        StatusCode::ACCEPTED,
        Json(json!({ "staged_path": path, "version": doc.version })),
    ))
}
