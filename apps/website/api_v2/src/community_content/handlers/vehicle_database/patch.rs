//! The administrator's partial vehicle write.
//!
//! **Role:** answers `PATCH /api/v1/vehicle-database/{id}`: changes the fields the body names and
//! answers the stored row.
//! **Position:** registered by [`crate::community_content::routes::routes`]; checks the body with
//! [`super::validation`], merges it onto the locked row and writes through
//! [`super::vehicle_rows`].
//! **Signals & state:** none; one transaction per accepted request.
//! **Invariants:** an absent key leaves its field as stored, `null` clears an optional field and is
//! refused on a required one; a refused body stores nothing; the merge reads the row under its
//! `FOR UPDATE` lock, so a concurrent write cannot slip between the read and the store; the row
//! write and its audit line, which names the changed fields, commit together.

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::response::Json;

use super::validation::VehiclePatchBody;
use super::vehicle_rows::{
    VehicleWrite, append_vehicle_audit, lock_live_vehicle, parse_vehicle_id, store_vehicle_fields,
};
use crate::community_content::models::VehicleDatabase;
use crate::core::application_state::AppState;
use crate::core::error_handling::api_error::ApiError;
use crate::core::middleware::AdminUser;

/// `PATCH /api/v1/vehicle-database/{id}`: changes the named fields of a live row.
///
/// @route PATCH /api/v1/vehicle-database/{id}
pub async fn patch_vehicle(
    State(state): State<AppState>,
    administrator: AdminUser,
    Path(id): Path<String>,
    body: Result<Json<VehiclePatchBody>, JsonRejection>,
) -> Result<Json<VehicleDatabase>, ApiError> {
    let id = parse_vehicle_id(&id)?;
    let Json(body) = body.map_err(ApiError::from_json_rejection)?;
    let changes = body.validate()?;
    let changed_keys = changes.changed_keys();
    let actor = administrator.0.discord_id.as_str();

    let mut transaction = state.pool.begin().await?;
    let stored = lock_live_vehicle(&mut transaction, id).await?;
    let fields = changes.applied_to(&stored);
    let vehicle = store_vehicle_fields(&mut transaction, id, &fields, actor).await?;
    append_vehicle_audit(
        &mut transaction,
        actor,
        VehicleWrite::Updated,
        &vehicle,
        Some(&changed_keys),
    )
    .await?;
    transaction.commit().await?;
    Ok(Json(vehicle))
}
