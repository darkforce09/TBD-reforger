//! A leader or administrator asks for the deterministic waitlist promotion of one attachment.
//! The earliest eligible waiting participants receive actual seats and places; the request
//! cannot choose who is promoted.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::Json;
use serde_json::{Value, json};

use crate::administration::services::required_audit::append_actor_audit;
use crate::core::application_state::AppState;
use crate::core::error_handling::api_error::ApiError;
use crate::core::middleware::LeaderUser;
use crate::operations::services::event_lookup::load_em;
use crate::operations::services::event_reservations::{
    mutation_authority::{require_leader, require_registration_open},
    reservation_scope::{AttachmentScope, ReservationScope},
    waitlist_promotion::promote_waiting_candidates,
};

/// @route POST /api/v1/event-missions/:emid/waitlist/promote
pub async fn promote_waitlisted_participants(
    State(state): State<AppState>,
    leader: LeaderUser,
    Path(emid): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let em = load_em(&state.pool, &emid).await?;
    let mut tx = state.pool.begin().await?;
    let scope = ReservationScope::lock(
        &mut tx,
        em.event_id,
        AttachmentScope::Active,
        Some((&leader.0, &state.cfg)),
        &[],
    )
    .await?;
    scope.require_active_mission(em.id)?;
    let actor = scope.actor()?.clone();
    require_leader(&actor)?;
    require_registration_open(&scope.event, actor.role == "admin")?;
    if scope.event.registration_locked {
        return Err(ApiError::conflict(
            "registration is locked; promotion resumes when registration reopens",
        ));
    }
    let waiting: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM event_registrations
         WHERE event_mission_id = $1 AND reservation_state = 'waitlisted'",
    )
    .bind(em.id)
    .fetch_one(&mut *tx)
    .await?;
    if waiting == 0 {
        return Err(ApiError::not_found(
            "no participant is waiting for this mission",
        ));
    }
    let promotions =
        promote_waiting_candidates(&mut tx, &scope, &state.cfg.discord_guild_id, Some(em.id))
            .await?;
    if promotions.is_empty() {
        return Err(ApiError::with_details(
            StatusCode::CONFLICT,
            "no seat and place are available for an eligible waiting participant",
            json!({"code": "EVENT_FULL"}),
        ));
    }
    append_actor_audit(
        &mut tx,
        &actor.discord_id,
        "event.waitlist_promotion_requested",
        "event_mission",
        &em.id.to_string(),
        &format!("Promoted {} waiting participant(s)", promotions.len()),
    )
    .await?;
    tx.commit().await?;
    Ok(Json(json!({
        "promoted": promotions.iter().map(|promotion| json!({
            "registration_id": promotion.registration,
            "discord_id": promotion.account,
            "slot_id": promotion.seat,
        })).collect::<Vec<_>>(),
    })))
}
