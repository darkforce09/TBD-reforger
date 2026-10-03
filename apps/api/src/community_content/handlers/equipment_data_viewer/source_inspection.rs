//! Bounded source pages; explicit expansion exposes complete values.
use super::{ReadResult, failure, response, viewer_query};
use crate::community_content::services::equipment_data_viewer::queries::{self, ViewerQuery};
use crate::core::application_state::AppState;
use axum::extract::rejection::QueryRejection;
use axum::extract::{Query, State};
use contract_schema_types::community_content::equipment_data_viewer::source_inspection::EquipmentSourcePage;

/// @route GET /api/v1/debug/equipment-data/selection
pub async fn selection(
    State(state): State<AppState>,
    query: Result<Query<ViewerQuery>, QueryRejection>,
) -> ReadResult<EquipmentSourcePage> {
    let p = viewer_query(query)?;
    let service = state
        .equipment_data
        .select(p.dataset.as_deref())
        .map_err(failure)?;
    let dataset = service.dataset(p.generation()).await.map_err(failure)?;
    response(
        p.dataset.clone().unwrap_or_else(|| "gameplay".into()),
        queries::selection::list(service, dataset, p)
            .await
            .map_err(failure)?,
    )
}

/// @route GET /api/v1/debug/equipment-data/containers
pub async fn containers(
    State(state): State<AppState>,
    query: Result<Query<ViewerQuery>, QueryRejection>,
) -> ReadResult<EquipmentSourcePage> {
    let p = viewer_query(query)?;
    let d = state
        .equipment_data
        .select(p.dataset.as_deref())
        .map_err(failure)?
        .dataset(p.generation())
        .await
        .map_err(failure)?;
    response(
        p.dataset.clone().unwrap_or_else(|| "gameplay".into()),
        queries::source_inspection::nodes(&d, &p)
            .await
            .map_err(failure)?,
    )
}

async fn inspect(
    state: AppState,
    p: ViewerQuery,
    operation: &str,
) -> ReadResult<EquipmentSourcePage> {
    let service = state
        .equipment_data
        .select(p.dataset.as_deref())
        .map_err(failure)?;
    let d = state
        .equipment_data
        .select(p.dataset.as_deref())
        .map_err(failure)?
        .dataset(p.generation())
        .await
        .map_err(failure)?;
    response(
        p.dataset.clone().unwrap_or_else(|| "gameplay".into()),
        queries::source_inspection::inspect(service, d, p, operation)
            .await
            .map_err(failure)?,
    )
}

/// @route GET /api/v1/debug/equipment-data/properties
pub async fn properties(
    State(state): State<AppState>,
    query: Result<Query<ViewerQuery>, QueryRejection>,
) -> ReadResult<EquipmentSourcePage> {
    let p = viewer_query(query)?;
    inspect(state, p, "properties").await
}
/// @route GET /api/v1/debug/equipment-data/values
pub async fn values(
    State(state): State<AppState>,
    query: Result<Query<ViewerQuery>, QueryRejection>,
) -> ReadResult<EquipmentSourcePage> {
    let p = viewer_query(query)?;
    inspect(state, p, "values").await
}
/// @route GET /api/v1/debug/equipment-data/documents
pub async fn documents(
    State(state): State<AppState>,
    query: Result<Query<ViewerQuery>, QueryRejection>,
) -> ReadResult<EquipmentSourcePage> {
    let p = viewer_query(query)?;
    inspect(state, p, "document").await
}

/// @route GET /api/v1/debug/equipment-data/resource-cards
pub async fn resource_cards(
    State(state): State<AppState>,
    query: Result<Query<ViewerQuery>, QueryRejection>,
) -> ReadResult<contract_schema_types::community_content::equipment_data_viewer::resource_cards::EquipmentResourceCardPage>{
    let p = viewer_query(query)?;
    let service = state
        .equipment_data
        .select(p.dataset.as_deref())
        .map_err(failure)?;
    let d = state
        .equipment_data
        .select(p.dataset.as_deref())
        .map_err(failure)?
        .dataset(p.generation())
        .await
        .map_err(failure)?;
    response(
        p.dataset.clone().unwrap_or_else(|| "gameplay".into()),
        queries::resource_cards::list(service, d, p)
            .await
            .map_err(failure)?,
    )
}
