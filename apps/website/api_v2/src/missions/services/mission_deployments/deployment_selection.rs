//! Validation of a deployment selection before anything is persisted. The caller holds the
//! server row lock; this module locks the mission and, when named, the event mission, and
//! refuses a selection the fleet cannot run with a code the operator can act on.

use serde_json::json;
use sqlx::PgConnection;
use uuid::Uuid;

use super::slot_bindings::{OrbatSeat, bind_seats, compiled_slots};
use crate::core::error_handling::api_error::ApiError;
use crate::missions::models::mission_deployment::DeploymentTransition;
use crate::operations::services::parse_orbat_template;

/// A selection every check has admitted, with what the deployment records.
pub struct ValidatedSelection {
    pub mission_id: Uuid,
    pub artifact_id: Uuid,
    pub artifact_sha256: String,
    pub event_mission_id: Option<Uuid>,
    pub terrain_key: String,
    pub scenario_id: String,
    pub transition: DeploymentTransition,
    /// The open runtime session whose loaded terrain made the transition a scenario restart; the
    /// runtime command is bound to it.
    pub running_session: Option<Uuid>,
    /// `(orbat slot, compiled slot uid)` for every seat of the event mission.
    pub bindings: Vec<(Uuid, String)>,
}

fn refusal(
    status: axum::http::StatusCode,
    code: &str,
    message: &str,
    details: serde_json::Value,
) -> ApiError {
    let mut details = details;
    details["code"] = json!(code);
    ApiError::with_details(status, message, details)
}

#[derive(sqlx::FromRow)]
struct ArtifactRow {
    terrain: String,
    document_sha256: String,
    modpack_id: Option<Uuid>,
    mission_version_id: Uuid,
}

/// Check the selection against the server the caller locked and return what to record.
pub async fn validate_selection(
    connection: &mut PgConnection,
    server: Uuid,
    required_modpack: Option<Uuid>,
    mission: Uuid,
    artifact: Uuid,
    event_mission: Option<Uuid>,
) -> Result<ValidatedSelection, ApiError> {
    use axum::http::StatusCode;
    let approved: Option<(String, Option<Uuid>)> = sqlx::query_as(
        "SELECT status::text, approved_artifact_id FROM missions
         WHERE id = $1 AND deleted_at IS NULL FOR SHARE",
    )
    .bind(mission)
    .fetch_optional(&mut *connection)
    .await?;
    let (status, approved_artifact) =
        approved.ok_or_else(|| ApiError::not_found("mission not found"))?;
    if status != "live" || approved_artifact != Some(artifact) {
        return Err(refusal(
            StatusCode::CONFLICT,
            "ARTIFACT_NOT_APPROVED",
            "only the artifact a live mission's latest approval decided can be deployed",
            json!({ "approved_artifact_id": approved_artifact, "mission_status": status }),
        ));
    }
    let row: ArtifactRow = sqlx::query_as(
        "SELECT terrain, document_sha256, modpack_id, mission_version_id
         FROM mission_artifacts WHERE id = $1 AND mission_id = $2",
    )
    .bind(artifact)
    .bind(mission)
    .fetch_one(&mut *connection)
    .await?;
    if row.modpack_id != required_modpack {
        return Err(refusal(
            StatusCode::UNPROCESSABLE_ENTITY,
            "MODPACK_MISMATCH",
            "the artifact was compiled against another modpack than the server requires",
            json!({ "artifact_modpack_id": row.modpack_id, "server_modpack_id": required_modpack }),
        ));
    }
    let scenario: Option<String> =
        sqlx::query_scalar("SELECT scenario_id FROM fleet_scenarios WHERE terrain_key = $1")
            .bind(&row.terrain)
            .fetch_optional(&mut *connection)
            .await?;
    let Some(scenario_id) = scenario else {
        return Err(refusal(
            StatusCode::UNPROCESSABLE_ENTITY,
            "TERRAIN_NOT_RUNNABLE",
            "no fleet scenario is registered for the artifact's terrain",
            json!({ "terrain_key": row.terrain }),
        ));
    };
    let bindings = match event_mission {
        None => Vec::new(),
        Some(event_mission) => {
            bind_event_mission(
                connection,
                server,
                mission,
                event_mission,
                artifact,
                row.mission_version_id,
            )
            .await?
        }
    };
    let running: Option<(Uuid, String)> = sqlx::query_as(
        "SELECT s.id, a.terrain FROM server_runtime_sessions s
         JOIN mission_artifacts a ON a.id = s.loaded_artifact_id
         WHERE s.server_id = $1 AND s.ended_at IS NULL",
    )
    .bind(server)
    .fetch_optional(&mut *connection)
    .await?;
    let running_session = running
        .filter(|(_, terrain)| *terrain == row.terrain)
        .map(|(session, _)| session);
    let transition = if running_session.is_some() {
        DeploymentTransition::ScenarioRestart
    } else {
        DeploymentTransition::HostRestart
    };
    Ok(ValidatedSelection {
        mission_id: mission,
        artifact_id: artifact,
        artifact_sha256: row.document_sha256,
        event_mission_id: event_mission,
        terrain_key: row.terrain,
        scenario_id,
        transition,
        running_session,
        bindings,
    })
}

/// The event mission runs this catalog mission on an event bound to this server, and its seats
/// correspond one to one with the artifact's compiled slots.
async fn bind_event_mission(
    connection: &mut PgConnection,
    server: Uuid,
    mission: Uuid,
    event_mission: Uuid,
    artifact: Uuid,
    version: Uuid,
) -> Result<Vec<(Uuid, String)>, ApiError> {
    use axum::http::StatusCode;
    let attachment: Option<(Uuid, Option<Uuid>)> = sqlx::query_as(
        "SELECT em.mission_id, e.server_id FROM event_missions em
         JOIN events e ON e.id = em.event_id AND e.deleted_at IS NULL
         WHERE em.id = $1 AND em.deleted_at IS NULL FOR SHARE OF em",
    )
    .bind(event_mission)
    .fetch_optional(&mut *connection)
    .await?;
    let (attached_mission, event_server) =
        attachment.ok_or_else(|| ApiError::not_found("event mission not found"))?;
    if attached_mission != mission || event_server != Some(server) {
        return Err(refusal(
            StatusCode::CONFLICT,
            "EVENT_MISSION_NOT_ON_SERVER",
            "the event mission must run this mission on an event bound to this server",
            json!({ "event_mission_mission_id": attached_mission, "event_server_id": event_server }),
        ));
    }
    let payload: String =
        sqlx::query_scalar("SELECT json_payload::text FROM mission_versions WHERE id = $1")
            .bind(version)
            .fetch_one(&mut *connection)
            .await?;
    let document: Vec<u8> =
        sqlx::query_scalar("SELECT document FROM mission_artifacts WHERE id = $1")
            .bind(artifact)
            .fetch_one(&mut *connection)
            .await?;
    let document: serde_json::Value = serde_json::from_slice(&document)
        .map_err(|_| ApiError::internal("a stored artifact document is not JSON"))?;
    let seats: Vec<(Uuid, String, String, i64, String)> = sqlx::query_as(
        "SELECT id, faction, squad, slot_index, role FROM orbat_slots
         WHERE event_mission_id = $1 ORDER BY faction, squad, slot_index, id",
    )
    .bind(event_mission)
    .fetch_all(&mut *connection)
    .await?;
    let seats: Vec<OrbatSeat> = seats
        .into_iter()
        .map(|(id, faction, squad, slot_index, role)| OrbatSeat {
            id,
            faction,
            squad,
            slot_index,
            role,
        })
        .collect();
    bind_seats(
        &parse_orbat_template(payload.as_bytes()),
        &compiled_slots(&document),
        &seats,
    )
    .map_err(|mismatch| {
        refusal(
            StatusCode::UNPROCESSABLE_ENTITY,
            "ORBAT_ARTIFACT_MISMATCH",
            "the event mission's seats do not correspond one to one with the artifact's slots",
            serde_json::to_value(&mismatch).unwrap_or_default(),
        )
    })
}
