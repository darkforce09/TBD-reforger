//! The vehicle database reads any signed-in member makes: the list and one row.
//!
//! **Role:** answers `GET /api/v1/vehicle-database` with every live row and
//! `GET /api/v1/vehicle-database/{id}` with one.
//! **Position:** registered by [`crate::community_content::routes::routes`]; reads through
//! [`super::vehicle_rows`] for the doctrine vehicle pages.
//! **Signals & state:** none; one pool query per request.
//! **Invariants:** the list answers `{"data": [...]}` ordered by `name`, then `id`, without the
//! deleted rows; a deleted or unknown id answers 404, a malformed one 400; an empty optional
//! column is left off the wire.

use axum::extract::State;
use axum::response::Json;

use super::vehicle_rows::{find_live_vehicle, list_live_vehicles, parse_vehicle_id};
use crate::community_content::models::VehicleDatabase;
use crate::community_content::models::vehicle_database::VehicleDatabaseList;
use crate::core::application_state::AppState;
use crate::core::error_handling::api_error::ApiError;
use crate::core::http::path_parameters::PathParams;
use crate::core::middleware::AuthUser;

/// `GET /api/v1/vehicle-database`: every live row of the identification table.
///
/// @route GET /api/v1/vehicle-database
pub async fn list_vehicles(
    State(state): State<AppState>,
    _member: AuthUser,
) -> Result<Json<VehicleDatabaseList>, ApiError> {
    let data = list_live_vehicles(&state.pool).await?;
    Ok(Json(VehicleDatabaseList { data }))
}

/// `GET /api/v1/vehicle-database/{id}`: one live row.
///
/// @route GET /api/v1/vehicle-database/{id}
pub async fn get_vehicle(
    State(state): State<AppState>,
    _member: AuthUser,
    PathParams(id): PathParams<String>,
) -> Result<Json<VehicleDatabase>, ApiError> {
    let id = parse_vehicle_id(&id)?;
    Ok(Json(find_live_vehicle(&state.pool, id).await?))
}
