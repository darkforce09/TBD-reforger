//! The administrator's vehicle removal.
//!
//! **Role:** answers `DELETE /api/v1/vehicle-database/{id}`: soft-deletes a live row and answers
//! it as stored.
//! **Position:** registered by [`crate::community_content::routes::routes`]; writes through
//! [`super::vehicle_rows`].
//! **Signals & state:** none; one transaction per accepted request.
//! **Invariants:** the row is kept with `deleted_at` and `deleted_by` set, so it leaves the list
//! and answers 404 on every route that names it, a second DELETE included; the row is locked
//! before the stamp is written, and the stamp and its audit line commit together.

use axum::extract::{Path, State};
use axum::response::Json;

use super::vehicle_rows::{
    VehicleWrite, append_vehicle_audit, lock_live_vehicle, parse_vehicle_id, soft_delete_vehicle,
};
use crate::community_content::models::VehicleDatabase;
use crate::core::application_state::AppState;
use crate::core::error_handling::api_error::ApiError;
use crate::core::middleware::AdminUser;

/// `DELETE /api/v1/vehicle-database/{id}`: soft-deletes a live row and answers it with 200.
///
/// @route DELETE /api/v1/vehicle-database/{id}
pub async fn delete_vehicle(
    State(state): State<AppState>,
    administrator: AdminUser,
    Path(id): Path<String>,
) -> Result<Json<VehicleDatabase>, ApiError> {
    let id = parse_vehicle_id(&id)?;
    let actor = administrator.0.discord_id.as_str();

    let mut transaction = state.pool.begin().await?;
    lock_live_vehicle(&mut transaction, id).await?;
    let vehicle = soft_delete_vehicle(&mut transaction, id, actor).await?;
    append_vehicle_audit(
        &mut transaction,
        actor,
        VehicleWrite::Deleted,
        &vehicle,
        None,
    )
    .await?;
    transaction.commit().await?;
    Ok(Json(vehicle))
}
