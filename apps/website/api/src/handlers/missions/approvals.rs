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

/// The `list_approvals` projection.
///
/// **Every field is non-optional, so the query must `COALESCE` anything that can arrive NULL —
/// and two of these six can (T-330).** `author_name` because the `LEFT JOIN` yields NULL for a
/// mission whose author row is gone, and `submitted_at` because both columns it can read
/// (`missions.updated_at`, `missions.created_at`) are nullable with no default. The other four
/// are `NOT NULL` base-table columns on the driving table, so they cannot.
///
/// `Option` was considered and rejected for the reason `models/telemetry.rs` records for
/// `Match::winning_faction` (T-325): the safety belongs in the query, not the type. Here the
/// case against `Option` is stronger still, because `submitted_at` has no
/// `skip_serializing_if` — making the field optional would emit a literal `"submitted_at":
/// null` and change the wire shape for every reviewer client, not just the NULL row.
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
    #[serde(with = "go_time")]
    submitted_at: DateTime<Utc>,
}

/// List-queue SQL for `GET /api/v1/approvals`.
///
/// Kept as a named const so the unique-order contract (T-414 trailing `, m.id ASC`) is
/// unit-testable without standing up Postgres. The COALESCE commentary below the handler
/// still owns *why* each fallback exists.
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
    // NOT NULL and no DEFAULT** (`migrations/0001_initial_schema.sql:375`) — so any INSERT that
    // omits the column stores NULL, and a bare `m.updated_at` decoded a 500:
    // *"error occurred while decoding column `updated_at`: unexpected null; try decoding as an
    // `Option`"*. `handlers/mod.rs:82` (`load_mission`, the canonical mission read) already
    // coalesced this exact column; this query was the outlier that didn't. Its author clearly
    // understood *join* nullability — `u.username` is NOT NULL in the schema and is coalesced
    // anyway, because the LEFT JOIN makes it NULL for a mission whose author row is gone — and
    // simply missed the base table's own nullability one column over.
    //
    // The fallback chain is deliberate and both links are load-bearing:
    //
    // 1. **`m.created_at`** — unlike `load_mission`, which serves `created_at` and `updated_at`
    //    as separate wire fields, this row projects one timestamp onto one field the reviewer
    //    reads as "when did this land in my queue". When the mission's own creation time is on
    //    the row it is a real fact and a strictly better answer than a sentinel. `now()` was
    //    rejected outright: it renders as "submitted just now" and sorts an unknown-age
    //    submission to the *bottom* of an oldest-first review queue — a lie that also hides the
    //    row it lies about.
    // 2. **`'0001-01-01 00:00:00+00'`** — the Go zero `time.Time`, the crate-wide "unknown
    //    timestamp" sentinel (`handlers/mod.rs:81`, `deployments.rs:127`, `:134`), which
    //    `go_time` renders as `0001-01-01T00:00:00Z` and `tests/null_tolerance.rs` already
    //    asserts for a NULL timestamp. It is what makes the fix **total**: `missions.created_at`
    //    is *also* nullable with no default (`0001_initial_schema.sql:374`), so a two-argument
    //    COALESCE would still 500 on a row with both timestamps NULL — which is exactly the
    //    shape `tests/null_tolerance.rs:77`'s mission INSERT produces today (it omits both).
    //
    // `ORDER BY` uses the same expression so the queue order is the order of the timestamp the
    // reviewer is shown. For every non-NULL row the expression *is* `m.updated_at`, so ordering
    // and payload are byte-identical to before for all normal data; it only decides where the
    // previously-undecodable rows land (with the sentinel: oldest-first, i.e. surfaced for a
    // human, rather than Postgres's default NULLS LAST burying them past the last page).
    //
    // Trailing `, m.id ASC` is the unique tiebreaker (T-414). Without it, rows that share the
    // same COALESCE key — including the null_tolerance sentinel cluster — have no total order,
    // so LIMIT/OFFSET paging can duplicate or skip a row across successive requests.
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

/// Approve UPDATE — named const so T-337's `updated_at = now()` pin is unit-testable.
///
/// Sibling status writes (`submit_mission`, `create_version`, PATCH) all bump `updated_at`.
/// Approve/reject were the outliers: the queue's `ORDER BY` / `submitted_at` projection and
/// any cache keyed on `updated_at` never saw the review action, and a NULL `updated_at`
/// survived a real state change. Always bump — same contract as every other status write.
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

/// The rejection body.
///
/// **`reason` is deliberately required — do not add `#[serde(default)]` to it (T-218).**
/// This is the third time tonight a defaulted field turned a malformed request into a
/// destructive write (T-185 wiped a user's roles off a 200 with no `roles`; T-218's first
/// pass had this page POST `{}` and overwrite `rejection_reason` with `""`). The shape is
/// always the same: the default is not "no data", it decodes as an affirmative empty value
/// and gets bound straight into an `UPDATE`.
///
/// Here the column is the only thing the author is ever told about why their mission came
/// back, so `""` is strictly worse than a 400 — the reviewer believes they explained
/// themselves and the author sees a blank rejection. Requiring the field turns `{}` into a
/// decode error, which the handler maps to 400 instead of a silent clobber.
#[derive(Debug, Deserialize)]
pub struct RejectInput {
    reason: String,
}

/// Reject UPDATE — named const so T-337's `updated_at = now()` pin is unit-testable.
///
/// Same always-bump decision as [`APPROVE_MISSION_SQL`]: review actions are status writes.
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
    // `.ok()` here collapsed *every* extractor failure — a missing body, a wrong
    // `Content-Type`, malformed JSON — into `""` and wrote it. The client guard added in
    // T-218 was the only thing standing between a fat-fingered request and a blanked
    // column. `map_err` is what the other ~25 handlers in this crate do; this one was the
    // outlier.
    let Json(input) = body.map_err(|_| ApiError::bad_request("reason is required"))?;
    // A reason of spaces is the same lie as no reason, and `trim` is what the frontend
    // guard checks — the two ends have to agree or the client is still the only guard.
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

// ═══ T-283 — THE REVIEW-COMMENT THREAD, AND WHY IT IS NOT AN ENDPOINT IN THIS FILE ══════════
//
// `reject_mission` above is the only channel a reviewer has to the author, and it is one text
// field, overwritten on every round. The SPA already admits this in as many words: the reviewer's
// comment box at `frontend/src/approvals.rs:279-290` is an `RwSignal<Vec<String>>` that is never
// POSTed, captioned at `:443` "Local to this browser tab. Not saved, not sent to the author",
// and the author's side (`frontend/src/missions.rs:2390`) is an empty state reading "Comments
// coming soon."
//
// **The blocker is a table, and `migrations/` is not this slice's file.** Verified on main:
// `grep -i comment` over `apps/website/api/migrations/` matches only `COMMENT ON` statements and
// prose — none of the 33 tables in 0001…0019 is a comment store. `audit_logs` cannot stand in:
// every reader of it is `AdminUser`-gated (`handlers/audit.rs:66`), so the author — the one
// person the thread exists for — cannot read a row written about their own mission.
//
// The migration is specified here rather than left to be re-derived, in the shape 0018 uses.
// REHEARSED, not drafted: applied to a clone of the populated `tbd_gate_migrate_persist` on
// 2026-07-31, with every claim below driven.
//
//     mission_comments(id uuid pk default gen_random_uuid(), mission_id uuid NOT NULL,
//                      author_id text NOT NULL, body text NOT NULL,
//                      created_at timestamptz NOT NULL DEFAULT now())
//     index (mission_id, created_at, id)          -- the trailing unique key is T-414's lesson
//
// ONE foreign key, and its NAME IS THE CONTRACT:
//
//     mission_comments_mission_id_fkey  →  missions(id)  ON DELETE CASCADE
//
// Tier A ownership, matching constraints 5/6/7 of 0018 (`mission_versions`, `mission_armories`,
// `mission_bookmarks` all CASCADE on `mission_id`). The `<table>_<column>_fkey` spelling is not
// cosmetic: `handlers/telemetry.rs:68`'s mapper matches 23503 on the constraint name to answer
// 400 instead of 500, and any other spelling silently 500s. That mapper is private to
// telemetry.rs, but the primitives it is built from are not — `is_foreign_key_violation` and
// `violated_constraint` are `pub` in `handlers/mod.rs:83,95`, so the endpoint that lands here
// builds its own two-line arm without touching a shared file.
//
// **`author_id` gets NO foreign key**, deliberately, under 0018 abstention (i): it is an actor
// stamp, and a review comment must outlive the person who wrote it — the same reason
// `missions.reviewed_by` and `audit_logs.actor_id` are unconstrained. Driven: a comment inserts
// cleanly with an `author_id` that has no `users` row.
//
// **The pre-clean is load-bearing, not decoration.** Migrations run on boot (`bin/api.rs:26`),
// so an `ADD CONSTRAINT` that meets an orphan is a dead API, not a failed deploy step — the
// T-555 lesson 0018 §0 was written for. Driven both ways on the clone: with one orphan planted,
// `ADD CONSTRAINT` fails with *"violates foreign key constraint
// mission_comments_mission_id_fkey"*; after the 0018-shaped sweep (capture the whole tuple into
// `fk_orphans` as `to_jsonb(x)` with ticket `T-283`, then DELETE where the parent is absent) the
// identical statement applies. Replaying the whole file is a clean no-op.
//
// One honest caveat on the CASCADE: `delete_mission` (`missions.rs:835`) is a SOFT delete
// (`UPDATE missions SET deleted_at = now()`), so the cascade is unreachable through the API as
// it stands — its value today is the INSERT-side check, exactly as 0018 says of its six identity
// constraints. Stated rather than implied.
//
// Not started here: the endpoint pair (`POST`/`GET /api/v1/missions/:id/comments`), which also
// needs a route in `app.rs` and a model, and the two SPA halves above. `body` takes the T-218
// treatment — required field, `trim`, empty → 400 — for the reason `RejectInput` documents.

#[cfg(test)]
mod t414_order_tiebreaker {
    use super::LIST_APPROVALS_SQL;

    /// T-414 — LIMIT/OFFSET over tied COALESCE keys needs a unique trailing key.
    #[test]
    fn list_approvals_sql_orders_by_id_after_submitted_at() {
        let order_idx = LIST_APPROVALS_SQL
            .find("ORDER BY")
            .expect("LIST_APPROVALS_SQL must ORDER BY");
        let order = &LIST_APPROVALS_SQL[order_idx..];
        assert!(
            order.contains("ASC,") && order.contains("m.id ASC"),
            "approvals queue ORDER BY must end with unique `, m.id ASC` tiebreaker; got: {order}"
        );
        // Guard against a regression that puts id first or drops the timestamp key.
        assert!(
            order.find("COALESCE").expect("COALESCE in ORDER BY")
                < order.find("m.id ASC").expect("m.id ASC in ORDER BY"),
            "m.id ASC must trail the COALESCE submitted_at key, not replace it"
        );
    }
}

#[cfg(test)]
mod t337_updated_at_bump {
    use super::{APPROVE_MISSION_SQL, REJECT_MISSION_SQL};

    /// T-337 — approve/reject must bump `missions.updated_at`, matching sibling status writes.
    ///
    /// Perturbation RED: strip `updated_at = now()` from either const → this test fails.
    #[test]
    fn approve_and_reject_sql_bump_updated_at() {
        assert!(
            APPROVE_MISSION_SQL.contains("updated_at = now()"),
            "approve UPDATE must set updated_at = now(); got: {APPROVE_MISSION_SQL}"
        );
        assert!(
            REJECT_MISSION_SQL.contains("updated_at = now()"),
            "reject UPDATE must set updated_at = now(); got: {REJECT_MISSION_SQL}"
        );
    }
}
