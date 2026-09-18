//! Leave of absence: a member files an LOA against a date range, reads back their own queue, and
//! an admin approves or denies from the review console.

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::Json;
use chrono::NaiveDate;
use serde::Deserialize;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::core::application_state::AppState;
use crate::core::error_handling::api_error::ApiError;
use crate::core::http::pagination::PageParams;
use crate::core::middleware::{AdminUser, AuthUser};
use crate::operations::models::LeaveRequest;

/// LOA create body.
///
/// **`reason` is deliberately required — do not add `#[serde(default)]` to it.** A defaulted
/// `reason` turns `{}` or a missing key into an affirmative empty string that lands in
/// `leave_requests.reason`, the same shape the ban / reject / warn bodies refuse. Dates keep
/// `#[serde(default)]` so the empty-string date guard below still owns that half; reason follows
/// the ban/reject/warn contract instead.
#[derive(Debug, Deserialize)]
pub struct CreateLeaveInput {
    #[serde(default)]
    starts_on: String,
    #[serde(default)]
    ends_on: String,
    reason: String,
}

/// `POST /api/v1/me/leave-requests` — file an LOA.
///
/// @route POST /api/v1/me/leave-requests
pub async fn submit_leave(
    State(state): State<AppState>,
    user: AuthUser,
    body: Result<Json<CreateLeaveInput>, JsonRejection>,
) -> Result<(StatusCode, Json<LeaveRequest>), ApiError> {
    let Json(input) =
        body.map_err(|_| ApiError::bad_request("starts_on, ends_on and reason are required"))?;
    if input.starts_on.is_empty() || input.ends_on.is_empty() {
        return Err(ApiError::bad_request("starts_on and ends_on are required"));
    }
    // Whitespace-only is the same lie as no reason. Trim once; store the trimmed form so the
    // column and any future audit line cannot disagree.
    let reason = input.reason.trim();
    if reason.is_empty() {
        return Err(ApiError::bad_request("reason is required"));
    }
    let (Ok(start), Ok(end)) = (
        NaiveDate::parse_from_str(&input.starts_on, "%Y-%m-%d"),
        NaiveDate::parse_from_str(&input.ends_on, "%Y-%m-%d"),
    ) else {
        return Err(ApiError::bad_request("dates must be YYYY-MM-DD"));
    };
    if end < start {
        return Err(ApiError::bad_request(
            "ends_on must be on or after starts_on",
        ));
    }

    let loa: LeaveRequest = sqlx::query_as(
        "INSERT INTO leave_requests (discord_id, starts_on, ends_on, reason, status, created_at) \
         VALUES ($1, $2, $3, $4, 'pending', now()) RETURNING id, discord_id, starts_on, ends_on, COALESCE(reason, '') AS reason, status, reviewed_by, COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at",
    )
    .bind(&user.discord_id)
    .bind(start)
    .bind(end)
    .bind(reason)
    .fetch_one(&state.pool)
    .await?;
    Ok((StatusCode::CREATED, Json(loa)))
}

/// `GET /api/v1/me/leave-requests` — the caller's LOA requests.
///
/// @route GET /api/v1/me/leave-requests
pub async fn list_my_leave(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<Json<Value>, ApiError> {
    let loas: Vec<LeaveRequest> = sqlx::query_as(
        "SELECT id, discord_id, starts_on, ends_on, COALESCE(reason, '') AS reason, status, reviewed_by, COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at FROM leave_requests WHERE discord_id = $1 ORDER BY created_at DESC",
    )
    .bind(&user.discord_id)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(json!({ "data": loas })))
}

/// `GET /api/v1/admin/leave-requests` — LOA review queue (admin), pending first.
///
/// @route GET /api/v1/admin/leave-requests
pub async fn list_all_leave(
    State(state): State<AppState>,
    _a: AdminUser,
    Query(page): Query<PageParams>,
) -> Result<Json<Value>, ApiError> {
    let (limit, offset) = page.bounds();
    let total: i64 = sqlx::query_scalar("SELECT count(*) FROM leave_requests")
        .fetch_one(&state.pool)
        .await?;
    let loas: Vec<LeaveRequest> = sqlx::query_as(
        "SELECT id, discord_id, starts_on, ends_on, COALESCE(reason, '') AS reason, status, reviewed_by, COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at FROM leave_requests ORDER BY (status::text = 'pending') DESC, created_at DESC \
         LIMIT $1 OFFSET $2",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(
        json!({ "data": loas, "total": total, "limit": limit, "offset": offset }),
    ))
}

#[derive(Debug, Deserialize)]
pub struct ReviewLeaveInput {
    #[serde(default)]
    status: String,
}

/// `PATCH /api/v1/admin/leave-requests/:id` — approve/deny an LOA (admin).
///
/// @route PATCH /api/v1/admin/leave-requests/:id
pub async fn review_leave(
    State(state): State<AppState>,
    admin: AdminUser,
    Path(id): Path<String>,
    body: Result<Json<ReviewLeaveInput>, JsonRejection>,
) -> Result<Json<Value>, ApiError> {
    let Ok(id) = Uuid::parse_str(&id) else {
        return Err(ApiError::bad_request("invalid id"));
    };
    let Json(input) = body.map_err(|_| ApiError::bad_request("status required"))?;
    if input.status.is_empty() {
        return Err(ApiError::bad_request("status required"));
    }
    if input.status != "approved" && input.status != "denied" {
        return Err(ApiError::bad_request("status must be approved or denied"));
    }
    let res = sqlx::query(
        "UPDATE leave_requests SET status = $1::leave_status, reviewed_by = $2 WHERE id = $3",
    )
    .bind(&input.status)
    .bind(&admin.0.discord_id)
    .bind(id)
    .execute(&state.pool)
    .await?;
    if res.rows_affected() == 0 {
        return Err(ApiError::not_found("LOA not found"));
    }
    Ok(Json(json!({ "status": input.status })))
}
