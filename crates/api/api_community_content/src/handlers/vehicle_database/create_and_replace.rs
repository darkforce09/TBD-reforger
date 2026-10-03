//! The administrator's whole-row vehicle writes: create and replace.
//!
//! **Role:** answers `POST /api/v1/vehicle-database` (a new row, 201) and
//! `PUT /api/v1/vehicle-database/{id}` (every field of a live row replaced, 200).
//! **Position:** registered by [`crate::routes::routes`]; checks the body with
//! [`super::validation`] and writes through [`super::vehicle_rows`], answering the stored
//! [`VehicleDatabase`].
//! **Signals & state:** none; one transaction per accepted request.
//! **Invariants:** a refused body (unreadable, unknown key, blank or over-long field, unsafe image
//! URL) answers before any statement runs and stores nothing; PUT locks the row before it writes;
//! the row write and its audit line commit together or not at all.

use axum::extract::State;
use axum::extract::rejection::JsonRejection;
use axum::http::StatusCode;
use axum::response::Json;

use super::validation::VehicleWriteBody;
use super::vehicle_rows::{
    VehicleWrite, append_vehicle_audit, insert_vehicle, lock_live_vehicle, parse_vehicle_id,
    store_vehicle_fields,
};
use crate::models::VehicleDatabase;
use api_foundation::error_handling::api_error::ApiError;
use api_foundation::http::path_parameters::PathParams;
use api_http_layer::middleware::AdminUser;
use api_state::AppState;

/// `POST /api/v1/vehicle-database`: adds a row and answers it with 201.
///
/// @route POST /api/v1/vehicle-database
pub async fn create_vehicle(
    State(state): State<AppState>,
    administrator: AdminUser,
    body: Result<Json<VehicleWriteBody>, JsonRejection>,
) -> Result<(StatusCode, Json<VehicleDatabase>), ApiError> {
    let Json(body) = body.map_err(ApiError::from_json_rejection)?;
    let fields = body.validate()?;
    let actor = &administrator.0.discord_id;

    let mut transaction = state.pool.begin().await?;
    let vehicle = insert_vehicle(&mut transaction, &fields, actor).await?;
    append_vehicle_audit(
        &mut transaction,
        actor,
        VehicleWrite::Created,
        &vehicle,
        None,
    )
    .await?;
    transaction.commit().await?;
    Ok((StatusCode::CREATED, Json(vehicle)))
}

/// `PUT /api/v1/vehicle-database/{id}`: replaces every field of a live row and answers the stored
/// row; an absent optional field is stored as none.
///
/// @route PUT /api/v1/vehicle-database/{id}
pub async fn replace_vehicle(
    State(state): State<AppState>,
    administrator: AdminUser,
    PathParams(id): PathParams<String>,
    body: Result<Json<VehicleWriteBody>, JsonRejection>,
) -> Result<Json<VehicleDatabase>, ApiError> {
    let id = parse_vehicle_id(&id)?;
    let Json(body) = body.map_err(ApiError::from_json_rejection)?;
    let fields = body.validate()?;
    let actor = &administrator.0.discord_id;

    let mut transaction = state.pool.begin().await?;
    lock_live_vehicle(&mut transaction, id).await?;
    let vehicle = store_vehicle_fields(&mut transaction, id, &fields, actor).await?;
    append_vehicle_audit(
        &mut transaction,
        actor,
        VehicleWrite::Replaced,
        &vehicle,
        None,
    )
    .await?;
    transaction.commit().await?;
    Ok(Json(vehicle))
}
