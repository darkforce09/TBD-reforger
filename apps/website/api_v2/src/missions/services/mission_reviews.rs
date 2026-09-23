//! Reviews of immutable artifacts: opening a review on submission, deciding exactly the artifact
//! under review, and the per-mission review thread. A decision, its comment, the mission's status
//! and the audit record commit together. Lock order: mission, then its reviews.

use sqlx::PgConnection;
use uuid::Uuid;

use crate::administration::services::required_audit::append_actor_audit;
use crate::core::error_handling::api_error::ApiError;
use crate::missions::models::mission::Mission;
use crate::missions::models::mission_review::{MissionReview, MissionReviewHistory, ReviewComment};
use crate::missions::services::mission_artifacts::artifact_store::compile_artifact;

const REVIEW_COLUMNS: &str = "r.id, r.mission_id, r.artifact_id, a.artifact_digest, a.mission_version_id, \
     v.semver, r.submitted_by, r.submitted_at, r.state, r.decided_by, r.decided_at";
const REVIEW_SOURCE: &str = "mission_reviews r JOIN mission_artifacts a ON a.id = r.artifact_id \
     JOIN mission_versions v ON v.id = a.mission_version_id";

fn review_conflict(code: &str, message: &str, details: serde_json::Value) -> ApiError {
    let mut details = details;
    details["code"] = serde_json::Value::from(code);
    ApiError::with_details(axum::http::StatusCode::CONFLICT, message, details)
}

fn bounded_body(raw: &str, field: &str) -> Result<String, ApiError> {
    let body = raw.trim();
    if body.is_empty() || body.len() > 8000 {
        return Err(ApiError::bad_request(format!(
            "{field} must contain 1 to 8000 bytes"
        )));
    }
    Ok(body.to_owned())
}

async fn load_review(
    connection: &mut PgConnection,
    review: Uuid,
) -> Result<MissionReview, ApiError> {
    Ok(sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT {REVIEW_COLUMNS} FROM {REVIEW_SOURCE} WHERE r.id = $1"
    )))
    .bind(review)
    .fetch_one(connection)
    .await?)
}

/// Compile the locked mission's current version and open a review of that artifact, superseding
/// an earlier pending review.
pub async fn open_review(
    connection: &mut PgConnection,
    mission: &Mission,
    actor: &str,
) -> Result<MissionReview, ApiError> {
    let version = mission
        .current_version_id
        .ok_or_else(|| ApiError::conflict("the mission has no saved version to review"))?;
    let artifact = compile_artifact(connection, mission, version, actor).await?;
    sqlx::query(
        "UPDATE mission_reviews SET state = 'superseded', decided_at = clock_timestamp()
         WHERE mission_id = $1 AND state = 'pending'",
    )
    .bind(mission.id)
    .execute(&mut *connection)
    .await?;
    let review: Uuid = sqlx::query_scalar(
        "INSERT INTO mission_reviews (mission_id, artifact_id, submitted_by) VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(mission.id)
    .bind(artifact.id)
    .bind(actor)
    .fetch_one(&mut *connection)
    .await?;
    load_review(connection, review).await
}

/// How a reviewer decides the artifact under review.
pub enum ReviewDecision {
    Approve {
        artifact: Uuid,
        conditions: Option<String>,
    },
    Reject {
        artifact: Uuid,
        reason: String,
    },
}

/// Decide the pending review of a mission the caller has locked. Deciding an artifact that is no
/// longer the one under review answers 409 with the artifact that is.
pub async fn decide_review(
    connection: &mut PgConnection,
    mission: &Mission,
    decision: ReviewDecision,
    reviewer: &str,
) -> Result<MissionReview, ApiError> {
    let pending: Option<(Uuid, Uuid, Uuid)> = sqlx::query_as(
        "SELECT r.id, r.artifact_id, a.mission_version_id FROM mission_reviews r
         JOIN mission_artifacts a ON a.id = r.artifact_id
         WHERE r.mission_id = $1 AND r.state = 'pending' FOR NO KEY UPDATE OF r",
    )
    .bind(mission.id)
    .fetch_optional(&mut *connection)
    .await?;
    let Some((review, reviewed, version)) = pending else {
        return Err(review_conflict(
            "NO_PENDING_REVIEW",
            "the mission has no artifact under review; resubmit it to compile the current version",
            serde_json::json!({}),
        ));
    };
    let (decided, state, comment) = match &decision {
        ReviewDecision::Approve {
            artifact,
            conditions,
        } => {
            let conditions = conditions
                .as_deref()
                .map(str::trim)
                .filter(|text| !text.is_empty())
                .map(|text| bounded_body(text, "conditions"))
                .transpose()?;
            let state = if conditions.is_some() {
                "approved_with_conditions"
            } else {
                "approved"
            };
            (
                *artifact,
                state,
                conditions.map(|text| ("approval_conditions", text)),
            )
        }
        ReviewDecision::Reject { artifact, reason } => (
            *artifact,
            "rejected",
            Some(("rejection", bounded_body(reason, "reason")?)),
        ),
    };
    if decided != reviewed {
        return Err(review_conflict(
            "REVIEWED_ARTIFACT_CHANGED",
            "the artifact under review is not the one decided",
            serde_json::json!({ "artifact_id": reviewed }),
        ));
    }
    sqlx::query(
        "UPDATE mission_reviews SET state = $2, decided_by = $3, decided_at = clock_timestamp() WHERE id = $1",
    )
    .bind(review)
    .bind(state)
    .bind(reviewer)
    .execute(&mut *connection)
    .await?;
    if let Some((kind, body)) = &comment {
        sqlx::query(
            "INSERT INTO mission_review_comments (mission_id, review_id, mission_version_id, artifact_id, author_id, kind, body)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(mission.id)
        .bind(review)
        .bind(version)
        .bind(reviewed)
        .bind(reviewer)
        .bind(kind)
        .bind(body)
        .execute(&mut *connection)
        .await?;
    }
    let approved = state != "rejected";
    sqlx::query(
        "UPDATE missions SET status = CASE WHEN $2 THEN 'live'::mission_status ELSE 'rejected'::mission_status END,
             approved_artifact_id = CASE WHEN $2 THEN $3 ELSE approved_artifact_id END,
             rejection_reason = CASE WHEN $2 THEN '' ELSE $4 END,
             reviewed_by = $5, reviewed_at = now(), updated_at = now()
         WHERE id = $1",
    )
    .bind(mission.id)
    .bind(approved)
    .bind(reviewed)
    .bind(comment.as_ref().map(|(_, body)| body.as_str()).unwrap_or_default())
    .bind(reviewer)
    .execute(&mut *connection)
    .await?;
    append_actor_audit(
        connection,
        reviewer,
        if approved {
            "mission.approve"
        } else {
            "mission.reject"
        },
        "mission",
        &mission.id.to_string(),
        &format!(
            "Decided review of mission '{}': {state} (artifact {reviewed})",
            mission.title
        ),
    )
    .await?;
    load_review(connection, review).await
}

/// Add a comment to the mission's review thread, optionally about one of its artifacts.
pub async fn add_review_comment(
    connection: &mut PgConnection,
    mission: &Mission,
    author: &str,
    body: &str,
    artifact: Option<Uuid>,
) -> Result<ReviewComment, ApiError> {
    let body = bounded_body(body, "body")?;
    let version: Option<Uuid> = match artifact {
        None => None,
        Some(artifact) => Some(
            sqlx::query_scalar("SELECT mission_version_id FROM mission_artifacts WHERE id = $1 AND mission_id = $2")
                .bind(artifact)
                .bind(mission.id)
                .fetch_optional(&mut *connection)
                .await?
                .ok_or_else(|| ApiError::bad_request("the artifact does not belong to this mission"))?,
        ),
    };
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO mission_review_comments (mission_id, mission_version_id, artifact_id, author_id, kind, body)
         VALUES ($1, $2, $3, $4, 'comment', $5) RETURNING id",
    )
    .bind(mission.id)
    .bind(version)
    .bind(artifact)
    .bind(author)
    .bind(&body)
    .fetch_one(&mut *connection)
    .await?;
    Ok(sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "{COMMENT_SELECT} WHERE c.id = $1"
    )))
    .bind(id)
    .fetch_one(connection)
    .await?)
}

const COMMENT_SELECT: &str = "SELECT c.id, c.mission_id, c.review_id, c.mission_version_id, c.artifact_id, \
     c.author_id, COALESCE(u.username, '') AS author_name, c.kind, c.body, c.created_at \
     FROM mission_review_comments c LEFT JOIN users u ON u.discord_id = c.author_id";

async fn review_thread(
    connection: &mut PgConnection,
    mission: Uuid,
) -> Result<Vec<ReviewComment>, ApiError> {
    Ok(sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "{COMMENT_SELECT} WHERE c.mission_id = $1 ORDER BY c.created_at, c.id"
    )))
    .bind(mission)
    .fetch_all(connection)
    .await?)
}

/// Every review of the mission, newest first, and its thread in order.
pub async fn review_history(
    connection: &mut PgConnection,
    mission: Uuid,
) -> Result<MissionReviewHistory, ApiError> {
    let reviews: Vec<MissionReview> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT {REVIEW_COLUMNS} FROM {REVIEW_SOURCE} WHERE r.mission_id = $1
         ORDER BY r.submitted_at DESC, r.id DESC"
    )))
    .bind(mission)
    .fetch_all(&mut *connection)
    .await?;
    let comments = review_thread(connection, mission).await?;
    Ok(MissionReviewHistory { reviews, comments })
}
