//! The strict mission export envelope: the mission row, its current version payload and its
//! armory assembled into one camelCase document.
//!
//! Two callers share it — the export download and the live-session inject tool — so the shape
//! they hand out is defined once here rather than twice at the boundaries.

use chrono::Utc;
use serde::Serialize;
use serde_json::value::RawValue;
use sqlx::PgPool;

use crate::core::error_handling::api_error::ApiError;
use crate::missions::models::mission::{Mission, MissionArmory, MissionVersion, TerrainType};

#[derive(Debug, Serialize)]
struct ArmoryExport {
    faction: String,
    category: String,
    item: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    quantity: Option<i64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MissionJson {
    export_format_version: i64,
    mission_id: String,
    title: String,
    terrain: String,
    game_mode: String,
    weather: String,
    time_of_day: String,
    max_players: i64,
    pub(crate) version: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    briefing: String,
    armory: Vec<ArmoryExport>,
    payload: Box<RawValue>,
    #[serde(with = "crate::core::wire_format::rfc3339_utc")]
    exported_at: chrono::DateTime<Utc>,
}

/// Assemble the strict export envelope (shared by export + inject).
pub(crate) async fn build_mission_doc(pool: &PgPool, m: &Mission) -> Result<MissionJson, ApiError> {
    let (payload, version) = match m.current_version_id {
        Some(vid) => {
            let v: MissionVersion = sqlx::query_as("SELECT id, mission_id, semver, json_payload, COALESCE(editor_notes, '') AS editor_notes, created_by, COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at FROM mission_versions WHERE id = $1")
                .bind(vid)
                .fetch_one(pool)
                .await
                .map_err(|_| ApiError::internal("could not build mission export"))?;
            (v.json_payload.0, v.semver)
        }
        None => (
            RawValue::from_string("{}".into()).unwrap(),
            "0.0.0".to_string(),
        ),
    };
    let armory: Vec<MissionArmory> = sqlx::query_as(
        "SELECT id, mission_id, faction, category, item_name, quantity, COALESCE(icon, '') AS icon, sort_order FROM mission_armories WHERE mission_id = $1 ORDER BY sort_order ASC",
    )
    .bind(m.id)
    .fetch_all(pool)
    .await?;
    let export_armory = armory
        .into_iter()
        .map(|a| ArmoryExport {
            faction: a.faction,
            category: a.category,
            item: a.item_name,
            quantity: a.quantity,
        })
        .collect();
    let terrain = if m.terrain == TerrainType::Custom && !m.custom_terrain_name.is_empty() {
        m.custom_terrain_name.clone()
    } else {
        m.terrain.as_str().to_string()
    };
    Ok(MissionJson {
        export_format_version: 1,
        mission_id: m.id.to_string(),
        title: m.title.clone(),
        terrain,
        game_mode: m.game_mode_wire(),
        weather: m.weather.as_str().to_string(),
        time_of_day: m.time_of_day.clone(),
        max_players: m.max_players,
        version,
        briefing: m.briefing.clone(),
        armory: export_armory,
        payload,
        exported_at: Utc::now(),
    })
}

// Wire spelling of the game mode enum, as the export envelope carries it.
impl Mission {
    fn game_mode_wire(&self) -> String {
        self.game_mode.as_str().to_string()
    }
}
