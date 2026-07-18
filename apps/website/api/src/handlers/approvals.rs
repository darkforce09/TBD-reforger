//! Mission approvals queue — Rust port of `handlers/approvals.go`. Admin-tier.

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, Query, State};
use axum::response::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::error::ApiError;
use crate::handlers::{PageParams, load_mission, username};
use crate::middleware::AdminUser;
use crate::models::serde_helpers::go_time;
use crate::models::{AuditSeverity, Mission, MissionStatus, TerrainType};
use crate::services::write_audit;
use crate::state::AppState;

#[derive(Debug, sqlx::FromRow)]
struct ApprovalRaw {
    id: Uuid,
    title: String,
    terrain: TerrainType,
    author_id: String,
    author_name: String,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct ApprovalRow {
    mission_id: String,
    title: String,
    terrain: String,
    author_id: String,
    author_name: String,
    #[serde(with = "go_time")]
    submitted_at: DateTime<Utc>,
}

/// `GET /api/v1/approvals` — missions awaiting review.
///
/// @route GET /api/v1/approvals
pub async fn list_approvals(
    State(state): State<AppState>,
    _a: AdminUser,
    Query(page): Query<PageParams>,
) -> Result<Json<Value>, ApiError> {
    let (limit, offset) = page.bounds();
    let total: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM missions WHERE status = 'pending_approval' AND deleted_at IS NULL",
    )
    .fetch_one(&state.pool)
    .await?;
    let raw: Vec<ApprovalRaw> = sqlx::query_as(
        "SELECT m.id, m.title, m.terrain, m.author_id, COALESCE(u.username, '') AS author_name, m.updated_at \
         FROM missions m LEFT JOIN users u ON u.discord_id = m.author_id \
         WHERE m.status = 'pending_approval' AND m.deleted_at IS NULL \
         ORDER BY m.updated_at ASC LIMIT $1 OFFSET $2",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.pool)
    .await?;
    let rows: Vec<ApprovalRow> = raw
        .into_iter()
        .map(|r| ApprovalRow {
            mission_id: r.id.to_string(),
            title: r.title,
            terrain: r.terrain.as_str().to_string(),
            author_id: r.author_id,
            author_name: r.author_name,
            submitted_at: r.updated_at,
        })
        .collect();
    Ok(Json(
        json!({ "data": rows, "total": total, "limit": limit, "offset": offset }),
    ))
}

/// Parse `:id` and load a mission that must be pending approval.
async fn load_pending(state: &AppState, id: &str) -> Result<Mission, ApiError> {
    let Ok(id) = Uuid::parse_str(id) else {
        return Err(ApiError::bad_request("invalid id"));
    };
    let m = load_mission(&state.pool, id)
        .await?
        .ok_or_else(|| ApiError::not_found("mission not found"))?;
    if m.status != MissionStatus::PendingApproval {
        return Err(ApiError::conflict("mission is not pending approval"));
    }
    Ok(m)
}

/// `POST /api/v1/approvals/:id/approve` — promote to the live library.
///
/// @route POST /api/v1/approvals/:id/approve
pub async fn approve_mission(
    State(state): State<AppState>,
    admin: AdminUser,
    Path(id): Path<String>,
) -> Result<Json<Mission>, ApiError> {
    let m = load_pending(&state, &id).await?;
    let reviewer = &admin.0.discord_id;
    sqlx::query(
        "UPDATE missions SET status = 'live', reviewed_by = $1, reviewed_at = now() WHERE id = $2",
    )
    .bind(reviewer)
    .bind(m.id)
    .execute(&state.pool)
    .await?;
    let reviewer_name = username(&state.pool, reviewer).await;
    write_audit(
        &state.pool,
        AuditSeverity::Info,
        Some(reviewer),
        &reviewer_name,
        "mission.approve",
        &format!("{reviewer_name} approved mission '{}'", m.title),
        "mission",
        &m.id.to_string(),
    )
    .await;
    Ok(Json(load_mission(&state.pool, m.id).await?.ok_or_else(
        || ApiError::internal("could not load mission"),
    )?))
}

#[derive(Debug, Deserialize, Default)]
pub struct RejectInput {
    #[serde(default)]
    reason: String,
}

/// `POST /api/v1/approvals/:id/reject` — return to the author.
///
/// @route POST /api/v1/approvals/:id/reject
pub async fn reject_mission(
    State(state): State<AppState>,
    admin: AdminUser,
    Path(id): Path<String>,
    body: Result<Json<RejectInput>, JsonRejection>,
) -> Result<Json<Mission>, ApiError> {
    let m = load_pending(&state, &id).await?;
    let reason = body.ok().map(|Json(b)| b.reason).unwrap_or_default();
    let reviewer = &admin.0.discord_id;
    sqlx::query(
        "UPDATE missions SET status = 'rejected', rejection_reason = $1, reviewed_by = $2, reviewed_at = now() WHERE id = $3",
    )
    .bind(&reason)
    .bind(reviewer)
    .bind(m.id)
    .execute(&state.pool)
    .await?;
    let reviewer_name = username(&state.pool, reviewer).await;
    write_audit(
        &state.pool,
        AuditSeverity::Warn,
        Some(reviewer),
        &reviewer_name,
        "mission.reject",
        &format!("{reviewer_name} rejected mission '{}'", m.title),
        "mission",
        &m.id.to_string(),
    )
    .await;
    Ok(Json(load_mission(&state.pool, m.id).await?.ok_or_else(
        || ApiError::internal("could not load mission"),
    )?))
}
