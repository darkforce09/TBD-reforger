//! Submission to the approval queue: the mission's current version compiles into an immutable
//! artifact and a review of exactly that artifact opens, in one transaction with the status change
//! and its audit record.

use axum::extract::{Path, State};
use axum::response::Json;

use crate::administration::services::audit_writer::actor_display_name;
use crate::administration::services::required_audit::append_actor_audit;
use crate::core::application_state::AppState;
use crate::core::error_handling::api_error::ApiError;
use crate::core::middleware::MissionMakerUser;
use crate::missions::models::mission::{Mission, MissionStatus};
use crate::missions::services::mission_lookup::{load_mission_on, load_mission_or_404};
use crate::missions::services::mission_reviews::open_review;
use crate::missions::services::mission_write_lock::lock_editable_mission;

/// `POST /api/v1/missions/:id/submit` — compile the current version into an immutable artifact and
/// open its review (author/admin).
///
/// **The only writer of `pending_approval` in the crate.** The metadata PATCH refuses that value
/// outright (`{"status":"pending_approval"}` → 400), so the admin approvals queue can only ever
/// show rows this handler wrote. It is the one door in, and a surface that wants a
/// mission reviewed has to call it.
///
/// ── Authorisation ───────────────────────────────────────────────────────────────────────────────
/// `can_edit` = author **or** admin, the same predicate PATCH and DELETE use: a `mission_maker`
/// who is not the author gets **403 "not your mission"**; an admin who is not the author gets
/// **200** and the mission moves to `pending_approval`. The admin override is deliberate and
/// consistent with the rest of the domain (an admin can already retitle, archive and delete any
/// mission), and `GET /approvals` is admin-only, so the reviewer tier is unchanged either way.
/// The extractor is [`MissionMakerUser`], same tier as PATCH — demotion revokes submit — and the
/// role and ownership are checked again after the mission lock wait.
///
/// ── Transitions ─────────────────────────────────────────────────────────────────────────────────
/// `draft` and `rejected` are submittable, and so is a `pending_approval` mission with no review
/// under way (one submitted before artifacts existed), which gains its review. A mission already
/// under review, `live` or `archived` answers 409, so a double submit cannot enqueue it twice. A
/// version that does not compile answers 422 and changes nothing. The artifact, the review, the
/// status and the audit record commit together.
///
/// @route POST /api/v1/missions/:id/submit
pub async fn submit_mission(
    State(state): State<AppState>,
    maker: MissionMakerUser,
    Path(id): Path<String>,
) -> Result<Json<Mission>, ApiError> {
    let user = &maker.0;
    let mut m = load_mission_or_404(&state.pool, &id).await?;
    let mut transaction = state.pool.begin().await?;
    lock_editable_mission(&mut transaction, &mut m, user, &state.cfg).await?;
    let mission = load_mission_on(&mut transaction, m.id)
        .await?
        .ok_or_else(|| ApiError::not_found("mission not found"))?;
    let under_review: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM mission_reviews WHERE mission_id = $1 AND state = 'pending')",
    )
    .bind(mission.id)
    .fetch_one(&mut *transaction)
    .await?;
    let submittable = match mission.status {
        MissionStatus::Draft | MissionStatus::Rejected => true,
        MissionStatus::PendingApproval => !under_review,
        MissionStatus::Live | MissionStatus::Archived => false,
    };
    if !submittable {
        return Err(ApiError::conflict(
            "only draft or rejected missions can be submitted",
        ));
    }
    let actor = &user.discord_id;
    let review = open_review(&mut transaction, &mission, actor).await?;
    sqlx::query(
        "UPDATE missions SET status = 'pending_approval', rejection_reason = '', \
         reviewed_by = NULL, reviewed_at = NULL, updated_at = now() WHERE id = $1",
    )
    .bind(mission.id)
    .execute(&mut *transaction)
    .await?;
    // Submission attribution identifies the caller, including an administrator acting for the author.
    let message = if *actor == mission.author_id {
        format!(
            "Submitted mission '{}' for approval (version {}, artifact {})",
            mission.title, review.semver, review.artifact_id
        )
    } else {
        let author_name = actor_display_name(&state.pool, &mission.author_id).await;
        format!(
            "Submitted {author_name}'s mission '{}' for approval (version {}, artifact {})",
            mission.title, review.semver, review.artifact_id
        )
    };
    append_actor_audit(
        &mut transaction,
        actor,
        "mission.submit",
        "mission",
        &mission.id.to_string(),
        &message,
    )
    .await?;
    let submitted = load_mission_on(&mut transaction, mission.id)
        .await?
        .ok_or_else(|| ApiError::not_found("mission not found"))?;
    transaction.commit().await?;
    Ok(Json(submitted))
}
