//! Dataset availability and generation-pinned summaries.
use super::{ReadResult, failure, response};
use crate::community_content::{
    models::generated::equipment_data_viewer::dataset::EquipmentDatasetStatus,
    services::equipment_data_viewer::queries::ViewerQuery,
};
use crate::core::application_state::AppState;
use axum::extract::{Query, State};

/// @route GET /api/v1/debug/equipment-data/status
pub async fn status(
    State(state): State<AppState>,
    Query(p): Query<ViewerQuery>,
) -> ReadResult<EquipmentDatasetStatus> {
    response(
        p.dataset.clone().unwrap_or_else(|| "gameplay".into()),
        state
            .equipment_data
            .select(p.dataset.as_deref())
            .map_err(failure)?
            .status(p.offset().map_err(failure)?)
            .await
            .map_err(failure)?,
    )
}

/// @route GET /api/v1/debug/equipment-data/overview
pub async fn overview(
    State(state): State<AppState>,
    Query(p): Query<ViewerQuery>,
) -> ReadResult<EquipmentDatasetStatus> {
    let dataset = state
        .equipment_data
        .select(p.dataset.as_deref())
        .map_err(failure)?
        .dataset(p.generation())
        .await
        .map_err(failure)?;
    let mut status = state
        .equipment_data
        .select(p.dataset.as_deref())
        .map_err(failure)?
        .status(p.offset().map_err(failure)?)
        .await
        .map_err(failure)?;
    status["generation_id"] = serde_json::json!(dataset.generation_id);
    status["overview"] = dataset.overview.clone();
    response(
        p.dataset.clone().unwrap_or_else(|| "gameplay".into()),
        status,
    )
}
