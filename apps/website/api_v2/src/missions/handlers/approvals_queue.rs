//! The admin-tier mission approval queue: list what is awaiting review with the artifact under
//! review, approve it (optionally with conditions) into the live library, or return it to its
//! author with a reason. Each decision names the exact artifact it decides.

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, Query, State};
use axum::response::Json;
use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::core::application_state::AppState;
use crate::core::error_handling::api_error::ApiError;
use crate::core::http::pagination::PageParams;
use crate::core::middleware::{AdminUser, role_rank};
use crate::core::wire_format::rfc3339_utc;
use crate::identity_and_access::services::session_authorization::authorize_on_connection;
use crate::missions::models::mission::{Mission, MissionStatus, TerrainType};
use crate::missions::models::mission_review::{ApprovalDecision, RejectionDecision};
use crate::missions::services::mission_lookup::load_mission_on;
use crate::missions::services::mission_reviews::{ReviewDecision, decide_review};

/// The `list_approvals` projection.
///
/// **The six mission fields are non-optional, so the query must `COALESCE` any of them that can
/// arrive NULL — and two can.** `author_name` because the `LEFT JOIN` yields NULL for a mission
/// whose author row is gone, and `submitted_at` because a mission without a review row falls back
/// to columns (`missions.updated_at`, `missions.created_at`) that are nullable with no default.
/// The other four are `NOT NULL` base-table columns on the driving table, so they cannot. The
/// four review fields are optional because a mission submitted before reviews existed has none.
///
/// `Option` is rejected here for the reason `models/telemetry.rs` records for
/// `Match::winning_faction`: the safety belongs in the query, not the type. The case against
/// `Option` is stronger still, because `submitted_at` has no `skip_serializing_if` — an optional
/// field would emit a literal `"submitted_at": null` and change the wire shape for every reviewer
/// client, not just the NULL row.
#[derive(Debug, sqlx::FromRow)]
struct ApprovalRaw {
    id: Uuid,
    title: String,
    terrain: TerrainType,
    author_id: String,
    author_name: String,
    submitted_at: DateTime<Utc>,
    review_id: Option<Uuid>,
    artifact_id: Option<Uuid>,
    artifact_digest: Option<String>,
    version_semver: Option<String>,
}

/// One queue row. The review fields are absent for a mission submitted before artifacts existed:
/// it must be resubmitted so its current version compiles into an artifact to decide.
#[derive(Debug, Serialize)]
struct ApprovalRow {
    mission_id: String,
    title: String,
    terrain: String,
    author_id: String,
    author_name: String,
    #[serde(with = "rfc3339_utc")]
    submitted_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    review_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    artifact_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    artifact_digest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    version_semver: Option<String>,
}

/// List-queue SQL for `GET /api/v1/approvals`.
///
/// Kept as a named const so the unique-order contract (the trailing `, m.id ASC`) is unit-testable
/// without standing up Postgres. The COALESCE commentary below the handler owns *why* each
/// fallback exists.
const LIST_APPROVALS_SQL: &str = "SELECT m.id, m.title, m.terrain, m.author_id, \
         COALESCE(u.username, '') AS author_name, \
         COALESCE(r.submitted_at, m.updated_at, m.created_at, '0001-01-01 00:00:00+00'::timestamptz) AS submitted_at, \
         r.id AS review_id, r.artifact_id, a.artifact_digest, v.semver AS version_semver \
         FROM missions m LEFT JOIN users u ON u.discord_id = m.author_id \
         LEFT JOIN mission_reviews r ON r.mission_id = m.id AND r.state = 'pending' \
         LEFT JOIN mission_artifacts a ON a.id = r.artifact_id \
         LEFT JOIN mission_versions v ON v.id = a.mission_version_id \
         WHERE m.status = 'pending_approval' AND m.deleted_at IS NULL \
         ORDER BY COALESCE(r.submitted_at, m.updated_at, m.created_at, '0001-01-01 00:00:00+00'::timestamptz) ASC, \
         m.id ASC \
         LIMIT $1 OFFSET $2";

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
    // `submitted_at` is the pending review's submission time. A mission submitted before reviews
    // existed has no review row, so the chain falls back to `m.updated_at`, which is `timestamp
    // with time zone` with **no NOT NULL and no DEFAULT** (`migrations/0001_initial_schema.sql:375`):
    // any INSERT that omits the column stores NULL and a bare `m.updated_at` fails to decode into
    // the non-`Option` field above. The COALESCE is what keeps the queue readable for such a row.
    //
    // Both fallback links after the review are load-bearing:
    //
    // 1. **`m.created_at`** — this row projects one timestamp onto one field the reviewer reads
    //    as "when did this land in my queue". When the mission's own creation time is on the row
    //    it is a real fact and a strictly better answer than a sentinel. `now()` is not an option:
    //    it renders as "submitted just now" and sorts an unknown-age submission to the *bottom* of
    //    an oldest-first review queue — a lie that also hides the row it lies about.
    // 2. **`'0001-01-01 00:00:00+00'`** — the crate-wide "unknown timestamp" sentinel, which
    //    `rfc3339_utc` renders as `0001-01-01T00:00:00Z` and `tests/null_tolerance_reads.rs` asserts for a
    //    NULL timestamp. It is what makes the fallback **total**: `missions.created_at` is *also*
    //    nullable with no default (`0001_initial_schema.sql:374`), so a two-argument COALESCE
    //    would still fail to decode a row with both timestamps NULL.
    //
    // `ORDER BY` uses the same expression so the queue order is the order of the timestamp the
    // reviewer is shown. For every non-NULL row the expression *is* `m.updated_at`; it only
    // decides where the otherwise-undecodable rows land — with the sentinel they sort oldest-first
    // and surface for a human, rather than being buried past the last page by NULLS LAST.
    //
    // Trailing `, m.id ASC` is the unique tiebreaker. Without it, rows that share the same
    // COALESCE key — including the null_tolerance sentinel cluster — have no total order, so
    // LIMIT/OFFSET paging can duplicate or skip a row across successive requests.
    let raw: Vec<ApprovalRaw> = sqlx::query_as(LIST_APPROVALS_SQL)
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
            submitted_at: r.submitted_at,
            review_id: r.review_id,
            artifact_id: r.artifact_id,
            artifact_digest: r.artifact_digest,
            version_semver: r.version_semver,
        })
        .collect();
    Ok(Json(
        json!({ "data": rows, "total": total, "limit": limit, "offset": offset }),
    ))
}

/// Lock a mission that must be pending approval, then confirm the reviewer is still an
/// administrator after the lock wait.
async fn lock_pending(
    connection: &mut sqlx::PgConnection,
    state: &AppState,
    admin: &AdminUser,
    id: &str,
) -> Result<(Mission, String), ApiError> {
    let Ok(id) = Uuid::parse_str(id) else {
        return Err(ApiError::bad_request("invalid id"));
    };
    sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM missions WHERE id = $1 AND deleted_at IS NULL FOR NO KEY UPDATE",
    )
    .bind(id)
    .fetch_optional(&mut *connection)
    .await?
    .ok_or_else(|| ApiError::not_found("mission not found"))?;
    let reviewer = authorize_on_connection(connection, &state.cfg, &admin.0.session_claims).await?;
    if role_rank(&reviewer.role) < role_rank("admin") {
        return Err(ApiError::forbidden("insufficient role"));
    }
    let mission = load_mission_on(connection, id)
        .await?
        .ok_or_else(|| ApiError::not_found("mission not found"))?;
    if mission.status != MissionStatus::PendingApproval {
        return Err(ApiError::conflict("mission is not pending approval"));
    }
    Ok((mission, reviewer.discord_id))
}

/// `POST /api/v1/approvals/:id/approve` — approve the artifact under review, optionally with
/// conditions, and promote the mission to the live library with that artifact. Answers the
/// decided mission; its review record is in `GET /missions/:id/reviews`.
///
/// @route POST /api/v1/approvals/:id/approve
pub async fn approve_mission(
    State(state): State<AppState>,
    admin: AdminUser,
    Path(id): Path<String>,
    body: Result<Json<ApprovalDecision>, JsonRejection>,
) -> Result<Json<Mission>, ApiError> {
    let Json(decision) = body.map_err(|_| ApiError::bad_request("artifact_id is required"))?;
    let mut transaction = state.pool.begin().await?;
    let (mission, reviewer) = lock_pending(&mut transaction, &state, &admin, &id).await?;
    decide_review(
        &mut transaction,
        &mission,
        ReviewDecision::Approve {
            artifact: decision.artifact_id,
            conditions: decision.conditions,
        },
        &reviewer,
    )
    .await?;
    let decided = load_mission_on(&mut transaction, mission.id)
        .await?
        .ok_or_else(|| ApiError::not_found("mission not found"))?;
    transaction.commit().await?;
    Ok(Json(decided))
}

/// `POST /api/v1/approvals/:id/reject` — return the mission to its author. The reason is required
/// (a missing or blank reason answers 400) and becomes the rejection comment of the review.
/// Answers the decided mission.
///
/// @route POST /api/v1/approvals/:id/reject
pub async fn reject_mission(
    State(state): State<AppState>,
    admin: AdminUser,
    Path(id): Path<String>,
    body: Result<Json<RejectionDecision>, JsonRejection>,
) -> Result<Json<Mission>, ApiError> {
    let Json(decision) =
        body.map_err(|_| ApiError::bad_request("artifact_id and reason are required"))?;
    let mut transaction = state.pool.begin().await?;
    let (mission, reviewer) = lock_pending(&mut transaction, &state, &admin, &id).await?;
    decide_review(
        &mut transaction,
        &mission,
        ReviewDecision::Reject {
            artifact: decision.artifact_id,
            reason: decision.reason,
        },
        &reviewer,
    )
    .await?;
    let decided = load_mission_on(&mut transaction, mission.id)
        .await?
        .ok_or_else(|| ApiError::not_found("mission not found"))?;
    transaction.commit().await?;
    Ok(Json(decided))
}

#[cfg(test)]
#[path = "tests/approvals_queue.rs"]
mod tests;
