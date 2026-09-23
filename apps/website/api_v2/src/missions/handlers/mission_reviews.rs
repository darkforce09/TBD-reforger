//! The review thread of a mission and the artifacts its reviews decide: the review history with
//! its comments, a new comment, an artifact's provenance and exact compiled bytes, and the
//! read-only review workspace that opens exactly the version an artifact compiled from.
//!
//! Every route here belongs to the mission's author and to administrators. A mission the caller
//! cannot see answers 404, exactly like a missing one; a visible mission the caller does not own
//! answers 403.

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Json, Response};
use serde::Serialize;
use uuid::Uuid;

use crate::administration::services::required_audit::append_actor_audit;
use crate::core::application_state::AppState;
use crate::core::error_handling::api_error::ApiError;
use crate::core::middleware::{AuthUser, MissionMakerUser};
use crate::missions::handlers::artifact_document_response::artifact_document_response;
use crate::missions::models::mission::{Mission, MissionVersion};
use crate::missions::models::mission_review::{
    MissionReviewHistory, ReviewComment, ReviewCommentRequest,
};
use crate::missions::services::mission_artifacts::artifact_store::{
    MissionArtifact, load_artifact, load_artifact_document,
};
use crate::missions::services::mission_lookup::load_mission_or_404;
use crate::missions::services::mission_reviews::{add_review_comment, review_history};
use crate::missions::services::mission_write_lock::lock_editable_mission;
use crate::missions::validation::access::{can_edit, can_view};

/// Load a mission the caller may review: its author or an administrator.
async fn load_reviewable(state: &AppState, user: &AuthUser, id: &str) -> Result<Mission, ApiError> {
    let mission = load_mission_or_404(&state.pool, id).await?;
    if !can_view(user, &mission) {
        return Err(ApiError::not_found("mission not found"));
    }
    if !can_edit(user, &mission) {
        return Err(ApiError::forbidden("not your mission"));
    }
    Ok(mission)
}

fn parse_artifact_id(raw: &str) -> Result<Uuid, ApiError> {
    Uuid::parse_str(raw).map_err(|_| ApiError::bad_request("invalid artifact id"))
}

/// `GET /api/v1/missions/:id/reviews` — every review of the mission, newest first, with the
/// review thread in order.
///
/// @route GET /api/v1/missions/:id/reviews
pub async fn list_mission_reviews(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<MissionReviewHistory>, ApiError> {
    let mission = load_reviewable(&state, &user, &id).await?;
    let mut connection = state.pool.acquire().await?;
    Ok(Json(review_history(&mut connection, mission.id).await?))
}

/// `POST /api/v1/missions/:id/review-comments` — add a comment to the review thread, optionally
/// about one of the mission's artifacts. The comment and its audit record commit together.
///
/// @route POST /api/v1/missions/:id/review-comments
pub async fn add_mission_review_comment(
    State(state): State<AppState>,
    maker: MissionMakerUser,
    Path(id): Path<String>,
    body: Result<Json<ReviewCommentRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<ReviewComment>), ApiError> {
    let Json(request) = body.map_err(|_| ApiError::bad_request("body is required"))?;
    let user = &maker.0;
    let mut mission = load_reviewable(&state, user, &id).await?;
    let mut transaction = state.pool.begin().await?;
    lock_editable_mission(&mut transaction, &mut mission, user, &state.cfg).await?;
    let comment = add_review_comment(
        &mut transaction,
        &mission,
        &user.discord_id,
        &request.body,
        request.artifact_id,
    )
    .await?;
    append_actor_audit(
        &mut transaction,
        &user.discord_id,
        "mission.review_comment",
        "mission",
        &mission.id.to_string(),
        &format!("Commented on the review of mission '{}'", mission.title),
    )
    .await?;
    transaction.commit().await?;
    Ok((StatusCode::CREATED, Json(comment)))
}

/// `GET /api/v1/missions/:id/artifacts/:artifact_id` — an artifact's provenance: the version,
/// metadata, catalog, modpack, compiler and schema it was compiled from, and its digests.
///
/// @route GET /api/v1/missions/:id/artifacts/:artifact_id
pub async fn get_mission_artifact(
    State(state): State<AppState>,
    user: AuthUser,
    Path((id, artifact_id)): Path<(String, String)>,
) -> Result<Json<MissionArtifact>, ApiError> {
    let mission = load_reviewable(&state, &user, &id).await?;
    let artifact_id = parse_artifact_id(&artifact_id)?;
    let mut connection = state.pool.acquire().await?;
    Ok(Json(
        load_artifact(&mut connection, mission.id, artifact_id).await?,
    ))
}

/// `GET /api/v1/missions/:id/artifacts/:artifact_id/document` — the exact compiled bytes of an
/// artifact, with their SHA-256 as a strong entity tag.
///
/// @route GET /api/v1/missions/:id/artifacts/:artifact_id/document
pub async fn get_mission_artifact_document(
    State(state): State<AppState>,
    user: AuthUser,
    Path((id, artifact_id)): Path<(String, String)>,
) -> Result<Response, ApiError> {
    let mission = load_reviewable(&state, &user, &id).await?;
    let artifact_id = parse_artifact_id(&artifact_id)?;
    let mut connection = state.pool.acquire().await?;
    let artifact = load_artifact(&mut connection, mission.id, artifact_id).await?;
    let document = load_artifact_document(&mut connection, artifact.id).await?;
    Ok(artifact_document_response(document))
}

/// `GET /missions/:id/artifacts/:artifact_id/workspace` response: the artifact and the exact
/// version it was compiled from, which the editor opens read-only.
#[derive(Debug, Serialize)]
pub struct ReviewWorkspace {
    pub artifact: MissionArtifact,
    pub version: MissionVersion,
}

/// `GET /api/v1/missions/:id/artifacts/:artifact_id/workspace` — the version an artifact compiled
/// from, verified against the payload digest the artifact recorded.
///
/// @route GET /api/v1/missions/:id/artifacts/:artifact_id/workspace
pub async fn get_review_workspace(
    State(state): State<AppState>,
    user: AuthUser,
    Path((id, artifact_id)): Path<(String, String)>,
) -> Result<Json<ReviewWorkspace>, ApiError> {
    let mission = load_reviewable(&state, &user, &id).await?;
    let artifact_id = parse_artifact_id(&artifact_id)?;
    let mut connection = state.pool.acquire().await?;
    let artifact = load_artifact(&mut connection, mission.id, artifact_id).await?;
    let version: MissionVersion = sqlx::query_as(
        "SELECT id, mission_id, semver, json_payload, COALESCE(editor_notes, '') AS editor_notes, \
         created_by, COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at \
         FROM mission_versions WHERE id = $1 AND mission_id = $2",
    )
    .bind(artifact.mission_version_id)
    .bind(mission.id)
    .fetch_one(&mut *connection)
    .await?;
    let payload_sha256: String = sqlx::query_scalar(
        "SELECT encode(sha256(convert_to(json_payload::text, 'UTF8')), 'hex') \
         FROM mission_versions WHERE id = $1",
    )
    .bind(version.id)
    .fetch_one(&mut *connection)
    .await?;
    if payload_sha256 != artifact.version_payload_sha256 {
        return Err(ApiError::internal(
            "the reviewed version no longer matches its artifact",
        ));
    }
    Ok(Json(ReviewWorkspace { artifact, version }))
}
