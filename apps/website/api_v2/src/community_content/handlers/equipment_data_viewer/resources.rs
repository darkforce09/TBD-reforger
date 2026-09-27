//! Resource, relationship and native-field catalogs.
use super::{ReadResult, failure, response};
use crate::community_content::{
    models::generated::equipment_data_viewer::{
        field_inventory::EquipmentFieldPage, relationships::EquipmentRelationshipPage,
        resources::EquipmentResourcePage,
    },
    services::equipment_data_viewer::queries::{self, ViewerQuery},
};
use crate::core::application_state::AppState;
use axum::extract::{Query, State};

/// @route GET /api/v1/debug/equipment-data/resources
pub async fn resources(
    State(state): State<AppState>,
    Query(p): Query<ViewerQuery>,
) -> ReadResult<EquipmentResourcePage> {
    let d = state
        .equipment_data
        .select(p.dataset.as_deref())
        .map_err(failure)?
        .dataset(p.generation())
        .await
        .map_err(failure)?;
    response(
        p.dataset.clone().unwrap_or_else(|| "gameplay".into()),
        queries::resources::list(&d, &p).await.map_err(failure)?,
    )
}

/// @route GET /api/v1/debug/equipment-data/relationships
pub async fn relationships(
    State(state): State<AppState>,
    Query(p): Query<ViewerQuery>,
) -> ReadResult<EquipmentRelationshipPage> {
    let d = state
        .equipment_data
        .select(p.dataset.as_deref())
        .map_err(failure)?
        .dataset(p.generation())
        .await
        .map_err(failure)?;
    response(
        p.dataset.clone().unwrap_or_else(|| "gameplay".into()),
        queries::relationships::list(&d, &p)
            .await
            .map_err(failure)?,
    )
}

/// @route GET /api/v1/debug/equipment-data/fields
pub async fn fields(
    State(state): State<AppState>,
    Query(p): Query<ViewerQuery>,
) -> ReadResult<EquipmentFieldPage> {
    let d = state
        .equipment_data
        .select(p.dataset.as_deref())
        .map_err(failure)?
        .dataset(p.generation())
        .await
        .map_err(failure)?;
    response(
        p.dataset.clone().unwrap_or_else(|| "gameplay".into()),
        queries::fields::list(&d, &p).await.map_err(failure)?,
    )
}
