//! The strict export envelope an author downloads: a document derived from the mission row and
//! its current version. The compiled mod document a game runtime loads is an immutable artifact,
//! served by the artifact routes.

use axum::extract::{Path, State};
use axum::http::header;
use axum::response::{IntoResponse, Response};

use crate::core::application_state::AppState;
use crate::core::error_handling::api_error::ApiError;
use crate::core::middleware::MissionMakerUser;
use crate::missions::services::mission_document::build_mission_doc;
use crate::missions::services::mission_lookup::load_mission_or_404;
use crate::missions::validation::access::can_view;

/// `GET /api/v1/missions/:id/export` — strict export envelope download (mission_maker+).
///
/// @route GET /api/v1/missions/:id/export
pub async fn export_mission(
    State(state): State<AppState>,
    maker: MissionMakerUser,
    Path(id): Path<String>,
) -> Result<Response, ApiError> {
    let m = load_mission_or_404(&state.pool, &id).await?;
    if !can_view(&maker.0, &m) {
        return Err(ApiError::not_found("mission not found"));
    }
    let doc = build_mission_doc(&state.pool, &m).await?;
    let body = serde_json::to_vec_pretty(&doc)
        .map_err(|_| ApiError::internal("could not build mission export"))?;
    Ok((
        [
            (header::CONTENT_TYPE, "application/json".to_string()),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"mission.json\"".to_string(),
            ),
        ],
        body,
    )
        .into_response())
}
