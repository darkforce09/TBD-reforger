//! The fleet scenario registry (administrator): which scenario header the fleet runs for each
//! terrain. Deployments of an artifact are refused for a terrain with no registered scenario.

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::Json;

use crate::administration::services::required_audit::append_actor_audit;
use crate::core::application_state::AppState;
use crate::core::error_handling::api_error::ApiError;
use crate::core::middleware::{AdminUser, role_rank};
use crate::identity_and_access::services::identity_ownership::lock_accounts;
use crate::identity_and_access::services::session_authorization::authorize_on_connection;
use crate::server_infrastructure::models::fleet_scenario::{
    FleetScenario, FleetScenarioList, FleetScenarioUpdate,
};

const SCENARIO_COLUMNS: &str = "terrain_key, scenario_id, display_name, updated_by, updated_at";

/// A terrain key as the compiler writes it: lowercase ASCII, digits and underscores, starting
/// with a letter, at most 64 bytes.
fn valid_terrain_key(key: &str) -> bool {
    key.len() <= 64
        && key.starts_with(|c: char| c.is_ascii_lowercase())
        && key
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

/// A scenario header resource: `{16 uppercase hex}` then a `.conf` path.
fn valid_scenario_id(id: &str) -> bool {
    let Some(rest) = id.strip_prefix('{') else {
        return false;
    };
    let Some((guid, path)) = rest.split_once('}') else {
        return false;
    };
    guid.len() == 16
        && guid
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'A'..=b'F').contains(&b))
        && path.len() > ".conf".len()
        && path.ends_with(".conf")
        && path
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b'/' | b'-'))
}

async fn reauthorize(
    connection: &mut sqlx::PgConnection,
    state: &AppState,
    admin: &AdminUser,
) -> Result<String, ApiError> {
    lock_accounts(connection, std::slice::from_ref(&admin.0.discord_id)).await?;
    let current = authorize_on_connection(connection, &state.cfg, &admin.0.session_claims).await?;
    if role_rank(&current.role) < role_rank("admin") {
        return Err(ApiError::forbidden("insufficient role"));
    }
    Ok(current.discord_id)
}

/// `GET /api/v1/fleet/scenarios` — every registered scenario, by terrain.
///
/// @route GET /api/v1/fleet/scenarios
pub async fn list_fleet_scenarios(
    State(state): State<AppState>,
    _admin: AdminUser,
) -> Result<Json<FleetScenarioList>, ApiError> {
    let items: Vec<FleetScenario> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT {SCENARIO_COLUMNS} FROM fleet_scenarios ORDER BY terrain_key"
    )))
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(FleetScenarioList { items }))
}

/// `PUT /api/v1/fleet/scenarios/:terrainKey` — register or replace the terrain's scenario.
///
/// @route PUT /api/v1/fleet/scenarios/:terrainKey
pub async fn put_fleet_scenario(
    State(state): State<AppState>,
    admin: AdminUser,
    Path(terrain_key): Path<String>,
    body: Result<Json<FleetScenarioUpdate>, JsonRejection>,
) -> Result<Json<FleetScenario>, ApiError> {
    let Json(update) =
        body.map_err(|_| ApiError::bad_request("scenario_id and display_name are required"))?;
    if !valid_terrain_key(&terrain_key) {
        return Err(ApiError::bad_request(
            "the terrain key is lowercase letters, digits and underscores, starting with a letter",
        ));
    }
    if !valid_scenario_id(update.scenario_id.trim()) {
        return Err(ApiError::bad_request(
            "scenario_id is a scenario header resource: {16 uppercase hex}path.conf",
        ));
    }
    let display_name = update.display_name.trim();
    if display_name.is_empty() || display_name.len() > 128 {
        return Err(ApiError::bad_request(
            "display_name must contain 1 to 128 bytes",
        ));
    }
    let mut transaction = state.pool.begin().await?;
    let actor = reauthorize(&mut transaction, &state, &admin).await?;
    let scenario: FleetScenario = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "INSERT INTO fleet_scenarios (terrain_key, scenario_id, display_name, updated_by)
         VALUES ($1, $2, $3, $4)
         ON CONFLICT (terrain_key) DO UPDATE SET scenario_id = EXCLUDED.scenario_id,
             display_name = EXCLUDED.display_name, updated_by = EXCLUDED.updated_by,
             updated_at = clock_timestamp()
         RETURNING {SCENARIO_COLUMNS}"
    )))
    .bind(&terrain_key)
    .bind(update.scenario_id.trim())
    .bind(display_name)
    .bind(&actor)
    .fetch_one(&mut *transaction)
    .await?;
    append_actor_audit(
        &mut transaction,
        &actor,
        "fleet.scenario_registered",
        "fleet_scenario",
        &terrain_key,
        &format!(
            "Terrain {terrain_key} runs scenario {}",
            scenario.scenario_id
        ),
    )
    .await?;
    transaction.commit().await?;
    Ok(Json(scenario))
}

/// `DELETE /api/v1/fleet/scenarios/:terrainKey` — stop offering the terrain to new deployments.
///
/// @route DELETE /api/v1/fleet/scenarios/:terrainKey
pub async fn delete_fleet_scenario(
    State(state): State<AppState>,
    admin: AdminUser,
    Path(terrain_key): Path<String>,
) -> Result<StatusCode, ApiError> {
    let mut transaction = state.pool.begin().await?;
    let actor = reauthorize(&mut transaction, &state, &admin).await?;
    let removed = sqlx::query("DELETE FROM fleet_scenarios WHERE terrain_key = $1")
        .bind(&terrain_key)
        .execute(&mut *transaction)
        .await?;
    if removed.rows_affected() == 0 {
        return Err(ApiError::not_found(
            "no scenario is registered for the terrain",
        ));
    }
    append_actor_audit(
        &mut transaction,
        &actor,
        "fleet.scenario_removed",
        "fleet_scenario",
        &terrain_key,
        &format!("Terrain {terrain_key} is no longer offered to new deployments"),
    )
    .await?;
    transaction.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
#[path = "tests/fleet_scenarios.rs"]
mod tests;
