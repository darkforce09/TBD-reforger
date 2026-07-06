//! Field tools — mortar fire missions + mission injection. Rust port of
//! `handlers/field_tools.go`.

use std::fs;

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::Json;
use serde::Deserialize;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::error::ApiError;
use crate::handlers::missions::build_mission_doc;
use crate::handlers::{load_mission, username};
use crate::middleware::{AdminUser, AuthUser};
use crate::models::{AuditSeverity, FireMission, MissionStatus};
use crate::services::{solve_fire_mission, write_audit};
use crate::state::AppState;

/// Staging dir for injected mission.json files (game-server bridge pickup).
const MISSION_STAGE_DIR: &str = "missions";
/// Local upload storage dir (also served at `/uploads`).
pub(crate) const UPLOAD_DIR: &str = "uploads";

#[derive(Debug, Deserialize)]
pub struct SolveInput {
    #[serde(default)]
    weapon_system: String,
    #[serde(default)]
    fp_x: f64,
    #[serde(default)]
    fp_y: f64,
    #[serde(default)]
    tgt_x: f64,
    #[serde(default)]
    tgt_y: f64,
}

/// `POST /api/v1/fire-missions/solve` — live firing solution (no persist).
///
/// @route POST /api/v1/fire-missions/solve
pub async fn solve_fire(
    _u: AuthUser,
    body: Result<Json<SolveInput>, JsonRejection>,
) -> Result<Json<Value>, ApiError> {
    let Json(input) = body.map_err(|_| ApiError::bad_request("invalid body"))?;
    let (sol, ok) = solve_fire_mission(
        &input.weapon_system,
        input.fp_x,
        input.fp_y,
        input.tgt_x,
        input.tgt_y,
    );
    if !ok {
        return Err(ApiError::with_details(
            StatusCode::UNPROCESSABLE_ENTITY,
            "target out of range",
            serde_json::to_value(&sol).unwrap_or(Value::Null),
        ));
    }
    Ok(Json(serde_json::to_value(&sol).unwrap()))
}

#[derive(Debug, Deserialize)]
pub struct SaveFireInput {
    #[serde(flatten)]
    solve: SolveInput,
    event_id: Option<String>,
    #[serde(default)]
    fp_grid: String,
    #[serde(default)]
    target_grid: String,
}

/// `POST /api/v1/fire-missions` — compute + persist a fire mission.
///
/// @route POST /api/v1/fire-missions
pub async fn save_fire(
    State(state): State<AppState>,
    user: AuthUser,
    body: Result<Json<SaveFireInput>, JsonRejection>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let Json(input) =
        body.map_err(|_| ApiError::bad_request("fp_grid and target_grid are required"))?;
    if input.fp_grid.is_empty() || input.target_grid.is_empty() {
        return Err(ApiError::bad_request(
            "fp_grid and target_grid are required",
        ));
    }
    let s = &input.solve;
    let (sol, ok) = solve_fire_mission(&s.weapon_system, s.fp_x, s.fp_y, s.tgt_x, s.tgt_y);
    if !ok {
        return Err(ApiError::with_details(
            StatusCode::UNPROCESSABLE_ENTITY,
            "target out of range",
            serde_json::to_value(&sol).unwrap_or(Value::Null),
        ));
    }
    let event_id = input
        .event_id
        .as_deref()
        .filter(|v| !v.is_empty())
        .and_then(|v| Uuid::parse_str(v).ok());
    let fm: FireMission = sqlx::query_as(
        "INSERT INTO fire_missions \
         (event_id, created_by, weapon_system, fp_grid, target_grid, distance_m, azimuth_deg, elevation_mils, created_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7::float8::numeric, $8, now()) \
         RETURNING id, event_id, created_by, weapon_system, fp_grid, target_grid, distance_m, \
          azimuth_deg::float8 AS azimuth_deg, elevation_mils, created_at",
    )
    .bind(event_id)
    .bind(&user.discord_id)
    .bind(&sol.weapon_system)
    .bind(&input.fp_grid)
    .bind(&input.target_grid)
    .bind(sol.distance_m)
    .bind(sol.azimuth_deg)
    .bind(sol.elevation_mils)
    .fetch_one(&state.pool)
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(json!({ "solution": sol, "fire_mission": fm })),
    ))
}

/// `GET /api/v1/events/:id/fire-missions` — saved fire missions on an event.
///
/// @route GET /api/v1/events/:id/fire-missions
pub async fn list_event_fire_missions(
    State(state): State<AppState>,
    _u: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let Ok(eid) = Uuid::parse_str(&id) else {
        return Err(ApiError::bad_request("invalid id"));
    };
    let fms: Vec<FireMission> = sqlx::query_as(
        "SELECT id, event_id, created_by, weapon_system, fp_grid, target_grid, distance_m, \
         azimuth_deg::float8 AS azimuth_deg, elevation_mils, created_at \
         FROM fire_missions WHERE event_id = $1 ORDER BY created_at ASC",
    )
    .bind(eid)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(json!({ "data": fms })))
}

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
    let actor_name = username(&state.pool, actor).await;
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
