//! Resource, relationship and native-field catalogs.
use super::{ReadResult, failure, response, viewer_query};
use api_equipment_datasets::queries::{self, ViewerQuery};
use api_state::AppState;
use axum::extract::rejection::QueryRejection;
use axum::extract::{Query, State};
use contract_schema_types::community_content::equipment_data_viewer::{
    field_inventory::EquipmentFieldPage, relationships::EquipmentRelationshipPage,
    resources::EquipmentResourcePage,
};

/// @route GET /api/v1/debug/equipment-data/resources
pub async fn resources(
    State(state): State<AppState>,
    query: Result<Query<ViewerQuery>, QueryRejection>,
) -> ReadResult<EquipmentResourcePage> {
    let p = viewer_query(query)?;
    let d = state
        .equipment_data
        .select(p.dataset.as_deref())
        .map_err(failure)?
        .dataset(&p.generation())
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
    query: Result<Query<ViewerQuery>, QueryRejection>,
) -> ReadResult<EquipmentRelationshipPage> {
    let p = viewer_query(query)?;
    let d = state
        .equipment_data
        .select(p.dataset.as_deref())
        .map_err(failure)?
        .dataset(&p.generation())
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
    query: Result<Query<ViewerQuery>, QueryRejection>,
) -> ReadResult<EquipmentFieldPage> {
    let p = viewer_query(query)?;
    let d = state
        .equipment_data
        .select(p.dataset.as_deref())
        .map_err(failure)?
        .dataset(&p.generation())
        .await
        .map_err(failure)?;
    response(
        p.dataset.clone().unwrap_or_else(|| "gameplay".into()),
        queries::fields::list(&d, &p).await.map_err(failure)?,
    )
}
