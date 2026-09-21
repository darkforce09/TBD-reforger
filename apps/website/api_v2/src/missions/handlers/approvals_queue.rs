//! The admin-tier mission approval queue: list what is awaiting review, promote a mission to the
//! live library, or return it to its author with a reason.

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, Query, State};
use axum::response::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::administration::models::audit_log::AuditSeverity;
use crate::administration::services::audit_writer::{actor_display_name, write_audit};
use crate::core::application_state::AppState;
use crate::core::error_handling::api_error::ApiError;
use crate::core::http::pagination::PageParams;
use crate::core::middleware::AdminUser;
use crate::core::wire_format::rfc3339_utc;
use crate::missions::models::mission::{Mission, MissionStatus, TerrainType};
use crate::missions::services::mission_lookup::load_mission;

/// The `list_approvals` projection.
///
/// **Every field is non-optional, so the query must `COALESCE` anything that can arrive NULL —
/// and two of these six can.** `author_name` because the `LEFT JOIN` yields NULL for a mission
/// whose author row is gone, and `submitted_at` because both columns it can read
/// (`missions.updated_at`, `missions.created_at`) are nullable with no default. The other four
/// are `NOT NULL` base-table columns on the driving table, so they cannot.
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
}

#[derive(Debug, Serialize)]
struct ApprovalRow {
    mission_id: String,
    title: String,
    terrain: String,
    author_id: String,
    author_name: String,
    #[serde(with = "rfc3339_utc")]
    submitted_at: DateTime<Utc>,
}

/// List-queue SQL for `GET /api/v1/approvals`.
///
/// Kept as a named const so the unique-order contract (the trailing `, m.id ASC`) is unit-testable
/// without standing up Postgres. The COALESCE commentary below the handler owns *why* each
/// fallback exists.
const LIST_APPROVALS_SQL: &str = "SELECT m.id, m.title, m.terrain, m.author_id, \
         COALESCE(u.username, '') AS author_name, \
         COALESCE(m.updated_at, m.created_at, '0001-01-01 00:00:00+00'::timestamptz) AS submitted_at \
         FROM missions m LEFT JOIN users u ON u.discord_id = m.author_id \
         WHERE m.status = 'pending_approval' AND m.deleted_at IS NULL \
         ORDER BY COALESCE(m.updated_at, m.created_at, '0001-01-01 00:00:00+00'::timestamptz) ASC, \
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
    // `submitted_at` reads `m.updated_at`, which is `timestamp with time zone` with **no
    // NOT NULL and no DEFAULT** (`migrations/0001_initial_schema.sql:375`), so any INSERT that
    // omits the column stores NULL and a bare `m.updated_at` fails to decode into the
    // non-`Option` field above. The COALESCE is what keeps the queue readable for such a row.
    //
    // Both links of the fallback chain are load-bearing:
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

/// Approve UPDATE — a named const so the `updated_at = now()` pin is unit-testable.
///
/// A review action is a status write, and every sibling status write (`submit_mission`,
/// `create_version`, PATCH) bumps `updated_at`. Approve and reject must too, or the queue's
/// `ORDER BY` / `submitted_at` projection and any cache keyed on `updated_at` never see the review,
/// and a NULL `updated_at` survives a real state change.
const APPROVE_MISSION_SQL: &str = "UPDATE missions SET status = 'live', reviewed_by = $1, \
         reviewed_at = now(), updated_at = now() WHERE id = $2";

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
    sqlx::query(APPROVE_MISSION_SQL)
        .bind(reviewer)
        .bind(m.id)
        .execute(&state.pool)
        .await?;
    let reviewer_name = actor_display_name(&state.pool, reviewer).await;
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

/// The rejection body.
///
/// **`reason` is deliberately required — do not add `#[serde(default)]` to it.** A default is not
/// "no data": it decodes as an affirmative empty value and is bound straight into the `UPDATE`.
///
/// This column is the only thing the author is ever told about why their mission came back, so
/// `""` is strictly worse than a 400 — the reviewer believes they explained themselves and the
/// author sees a blank rejection. Requiring the field turns `{}` into a decode error, which the
/// handler maps to 400 instead of a silent clobber.
#[derive(Debug, Deserialize)]
pub struct RejectInput {
    reason: String,
}

/// Reject UPDATE — a named const so the `updated_at = now()` pin is unit-testable.
///
/// Same always-bump rule as [`APPROVE_MISSION_SQL`]: review actions are status writes.
const REJECT_MISSION_SQL: &str = "UPDATE missions SET status = 'rejected', rejection_reason = $1, \
         reviewed_by = $2, reviewed_at = now(), updated_at = now() WHERE id = $3";

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
    // Every extractor failure — a missing body, a wrong `Content-Type`, malformed JSON — is a 400
    // here rather than an `.ok()` collapse to `""` that would blank the column. `map_err` is what
    // the other handlers in this crate do.
    let Json(input) = body.map_err(|_| ApiError::bad_request("reason is required"))?;
    // A reason of spaces is the same lie as no reason, and `trim` is what the frontend guard
    // checks — the two ends have to agree or the client is the only guard.
    let reason = input.reason.trim();
    if reason.is_empty() {
        return Err(ApiError::bad_request("reason is required"));
    }
    let reviewer = &admin.0.discord_id;
    sqlx::query(REJECT_MISSION_SQL)
        .bind(reason)
        .bind(reviewer)
        .bind(m.id)
        .execute(&state.pool)
        .await?;
    let reviewer_name = actor_display_name(&state.pool, reviewer).await;
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

// ═══ WHY THERE IS NO REVIEW-COMMENT THREAD HERE ═══════════════════════════════════════════════
//
// `reject_mission` above is the only channel a reviewer has to the author: one text field,
// overwritten on every round. The SPA keeps the reviewer's comment box local to the browser tab
// (`frontend/src/v2/pages/administration/approvals/review_drawer.rs`) because nothing stores a
// thread: `grep -i comment` over `migrations/` matches only `COMMENT ON` statements and prose, and
// `audit_logs` cannot stand in — every reader of it is `AdminUser`-gated
// (`administration/handlers/audit_logs.rs`), so the author could never read a row written about
// their own mission. A thread therefore needs its own table (one foreign key to `missions(id)`,
// named `<table>_<column>_fkey` so the SQLSTATE 23503 predicates in
// `core/database/postgres_errors.rs` can answer 400 by constraint name, and no key on the actor
// stamp, which must outlive the person), an endpoint pair under `/missions/{id}/comments`, and
// the SPA half that sends the box. None of that exists, and this file says so rather than
// implying a thread that is not there.

#[cfg(test)]
#[path = "tests/approvals_queue.rs"]
mod tests;
