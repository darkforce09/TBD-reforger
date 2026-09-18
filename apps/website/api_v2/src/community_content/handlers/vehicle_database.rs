//! The vehicle database / IFF catalog: the member-facing table and the admin row writer.

use axum::extract::State;
use axum::extract::rejection::JsonRejection;
use axum::response::Json;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::community_content::models::wiki::VehicleDatabase;
use crate::core::application_state::AppState;
use crate::core::error_handling::api_error::ApiError;
use crate::core::middleware::{AdminUser, AuthUser};

/// `GET /api/v1/vehicle-database` — the Vehicle Database / IFF table.
///
/// @route GET /api/v1/vehicle-database
pub async fn list_vehicles(
    State(state): State<AppState>,
    _u: AuthUser,
) -> Result<Json<Value>, ApiError> {
    let vehicles: Vec<VehicleDatabase> =
        sqlx::query_as("SELECT id, name, faction, armor_type, COALESCE(amphibious, '') AS amphibious, COALESCE(primary_threat, '') AS primary_threat, COALESCE(profile_image_url, '') AS profile_image_url FROM vehicle_databases ORDER BY name ASC")
            .fetch_all(&state.pool)
            .await?;
    Ok(Json(json!({ "data": vehicles })))
}

/// Body for authoring a vehicle-database / IFF row (admin).
///
/// `name`, `faction`, and `armor_type` are required and non-empty. The three optional columns
/// default to empty string so a create that omits them still lands a full row (the list SELECT
/// already `COALESCE`s nulls to `''`, and the model skips empty strings on serialize).
#[derive(Debug, Deserialize)]
pub struct VehicleInput {
    #[serde(default)]
    name: String,
    #[serde(default)]
    faction: String,
    #[serde(default)]
    armor_type: String,
    #[serde(default)]
    amphibious: String,
    #[serde(default)]
    primary_threat: String,
    #[serde(default)]
    profile_image_url: String,
}

/// `POST /api/v1/vehicle-database` — insert an IFF row (admin).
///
/// @route POST /api/v1/vehicle-database
pub async fn create_vehicle(
    State(state): State<AppState>,
    _admin: AdminUser,
    body: Result<Json<VehicleInput>, JsonRejection>,
) -> Result<Json<VehicleDatabase>, ApiError> {
    let Json(input) = body.map_err(|_| {
        ApiError::bad_request(
            "name, faction, armor_type, amphibious, primary_threat and profile_image_url are required",
        )
    })?;
    if input.name.trim().is_empty()
        || input.faction.trim().is_empty()
        || input.armor_type.trim().is_empty()
    {
        return Err(ApiError::bad_request(
            "name, faction and armor_type are required",
        ));
    }
    let id: uuid::Uuid = sqlx::query_scalar(
        "INSERT INTO vehicle_databases (name, faction, armor_type, amphibious, primary_threat, profile_image_url) \
         VALUES ($1, $2, $3, NULLIF($4, ''), NULLIF($5, ''), NULLIF($6, '')) \
         RETURNING id",
    )
    .bind(input.name.trim())
    .bind(input.faction.trim())
    .bind(input.armor_type.trim())
    .bind(input.amphibious.trim())
    .bind(input.primary_threat.trim())
    .bind(input.profile_image_url.trim())
    .fetch_one(&state.pool)
    .await?;

    let vehicle: VehicleDatabase = sqlx::query_as(
        "SELECT id, name, faction, armor_type, COALESCE(amphibious, '') AS amphibious, \
         COALESCE(primary_threat, '') AS primary_threat, COALESCE(profile_image_url, '') AS profile_image_url \
         FROM vehicle_databases WHERE id = $1",
    )
    .bind(id)
    .fetch_one(&state.pool)
    .await?;
    Ok(Json(vehicle))
}
