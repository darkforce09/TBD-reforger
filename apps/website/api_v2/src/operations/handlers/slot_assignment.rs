//! Leader and admin writes on an ORBAT: seating a member in a slot, clearing a seat, and
//! holding or releasing a whole squad.
//!
//! The squad hold gates the first two. Outside an admin, only the leader currently holding a
//! squad may seat or clear its slots, which is a stricter rule than self-service registration
//! applies — see [`can_manage_squad`].

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json, Response};
use serde::Deserialize;
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

use super::slot_registration::{release_other_seats, squad_reserved_by};
use crate::core::application_state::AppState;
use crate::core::error_handling::api_error::ApiError;
use crate::core::middleware::LeaderUser;
use crate::operations::models::event::{OrbatReservation, OrbatSlot};
use crate::operations::services::event_lookup::load_em;

/// Admin, or the leader holding this squad. This is stricter than the gate in
/// `register_for_event_mission`: an *unreserved* squad is freely claimable there and NOT
/// manageable here, so the two share [`squad_reserved_by`] but not the decision. The reservation
/// lookup is name-scoped, not faction-scoped — that limit is the schema's and is documented on
/// [`squad_reserved_by`].
async fn can_manage_squad(
    pool: &PgPool,
    is_admin: bool,
    me: &str,
    em_id: Uuid,
    squad: &str,
) -> bool {
    if is_admin {
        return true;
    }
    let res = squad_reserved_by(pool, em_id, squad).await.ok().flatten();
    res.as_deref() == Some(me)
}

#[derive(Debug, Deserialize)]
pub struct AssignSlotInput {
    #[serde(default)]
    discord_id: String,
}

/// `PUT /api/v1/event-missions/:emid/slots/:slotId/assign` — assign a user (leader/admin).
///
/// @route PUT /api/v1/event-missions/:emid/slots/:slotId/assign
pub async fn assign_slot(
    State(state): State<AppState>,
    leader: LeaderUser,
    Path((emid, slot_id_s)): Path<(String, String)>,
    body: Result<Json<AssignSlotInput>, JsonRejection>,
) -> Result<Json<Value>, ApiError> {
    let em = load_em(&state.pool, &emid).await?;
    let Ok(slot_id) = Uuid::parse_str(&slot_id_s) else {
        return Err(ApiError::bad_request("invalid slot id"));
    };
    let Json(input) = body.map_err(|_| ApiError::bad_request("discord_id required"))?;
    if input.discord_id.is_empty() {
        return Err(ApiError::bad_request("discord_id required"));
    }
    let exists: Option<i32> = sqlx::query_scalar("SELECT 1 FROM users WHERE discord_id = $1")
        .bind(&input.discord_id)
        .fetch_optional(&state.pool)
        .await?;
    if exists.is_none() {
        return Err(ApiError::bad_request("user not found"));
    }
    let slot: Option<OrbatSlot> =
        sqlx::query_as("SELECT id, event_mission_id, faction, squad, COALESCE(callsign, '') AS callsign, role, COALESCE(loadout, '') AS loadout, COALESCE(tag, '') AS tag, slot_index, assigned_to, assigned_at FROM orbat_slots WHERE id = $1 AND event_mission_id = $2")
            .bind(slot_id)
            .bind(em.id)
            .fetch_optional(&state.pool)
            .await?;
    let Some(slot) = slot else {
        return Err(ApiError::not_found("slot not found"));
    };
    let is_admin = leader.0.role == "admin";
    if !can_manage_squad(
        &state.pool,
        is_admin,
        &leader.0.discord_id,
        em.id,
        &slot.squad,
    )
    .await
    {
        return Err(ApiError::forbidden(
            "reserve this squad to assign its slots",
        ));
    }

    let mut tx = state.pool.begin().await?;
    // ══ A LEADER ASSIGNMENT IS A SEAT MOVE TOO ════════════════════════════════════════════
    // The same seat-move register performs, reached by a different door: the claim below writes
    // the new seat and the upsert under it repoints the registration at that seat, so any seat
    // the assignee already holds in this operation has to be released in the same breath or they
    // end up with one person, two seats and one registration row. A leader filling a squad from
    // the member directory is the likeliest way to reach it, because the directory does not show
    // that the person is already seated elsewhere. A test drives PUT .../slots/:id/assign against
    // a user already holding another seat in the same operation and asserts they end up with one.
    //
    // The mission-row lock is needed for the same reason register takes it: release-then-claim is
    // a check-then-write pair, so without it a leader assignment and a self-registration can
    // interleave between the release and the claim. Same row, same order as the other two
    // handlers, so there is no lock-ordering cycle.
    sqlx::query("SELECT id FROM event_missions WHERE id = $1 FOR UPDATE")
        .bind(em.id)
        .fetch_one(&mut *tx)
        .await?;
    release_other_seats(&mut tx, em.id, &input.discord_id, Some(slot_id)).await?;
    sqlx::query("UPDATE orbat_slots SET assigned_to = $1, assigned_at = now() WHERE id = $2")
        .bind(&input.discord_id)
        .bind(slot_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "INSERT INTO event_registrations (event_mission_id, discord_id, slot_id, state) \
         VALUES ($1, $2, $3, 'registered') \
         ON CONFLICT (event_mission_id, discord_id) DO UPDATE SET slot_id = EXCLUDED.slot_id, state = EXCLUDED.state",
    )
    .bind(em.id)
    .bind(&input.discord_id)
    .bind(slot_id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(json!({ "assigned_to": input.discord_id })))
}

/// `DELETE /api/v1/event-missions/:emid/slots/:slotId/assign` — unassign (leader/admin).
///
/// @route DELETE /api/v1/event-missions/:emid/slots/:slotId/assign
pub async fn clear_slot(
    State(state): State<AppState>,
    leader: LeaderUser,
    Path((emid, slot_id_s)): Path<(String, String)>,
) -> Result<Json<Value>, ApiError> {
    let em = load_em(&state.pool, &emid).await?;
    let Ok(slot_id) = Uuid::parse_str(&slot_id_s) else {
        return Err(ApiError::bad_request("invalid slot id"));
    };
    let slot: Option<OrbatSlot> =
        sqlx::query_as("SELECT id, event_mission_id, faction, squad, COALESCE(callsign, '') AS callsign, role, COALESCE(loadout, '') AS loadout, COALESCE(tag, '') AS tag, slot_index, assigned_to, assigned_at FROM orbat_slots WHERE id = $1 AND event_mission_id = $2")
            .bind(slot_id)
            .bind(em.id)
            .fetch_optional(&state.pool)
            .await?;
    let Some(slot) = slot else {
        return Err(ApiError::not_found("slot not found"));
    };
    let is_admin = leader.0.role == "admin";
    if !can_manage_squad(
        &state.pool,
        is_admin,
        &leader.0.discord_id,
        em.id,
        &slot.squad,
    )
    .await
    {
        return Err(ApiError::forbidden(
            "reserve this squad to manage its slots",
        ));
    }
    let mut tx = state.pool.begin().await?;
    sqlx::query("UPDATE orbat_slots SET assigned_to = NULL, assigned_at = NULL WHERE id = $1 AND event_mission_id = $2")
        .bind(slot_id)
        .bind(em.id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE event_registrations SET slot_id = NULL WHERE event_mission_id = $1 AND slot_id = $2")
        .bind(em.id)
        .bind(slot_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(Json(json!({ "cleared": true })))
}

// --- Squad reservation (leader) ---

#[derive(Debug, Deserialize)]
pub struct SquadBody {
    #[serde(default)]
    squad: String,
}

/// `POST /api/v1/event-missions/:emid/squads/reserve` — hold a squad (leader).
///
/// @route POST /api/v1/event-missions/:emid/squads/reserve
pub async fn reserve_squad(
    State(state): State<AppState>,
    leader: LeaderUser,
    Path(emid): Path<String>,
    body: Result<Json<SquadBody>, JsonRejection>,
) -> Result<Response, ApiError> {
    let em = load_em(&state.pool, &emid).await?;
    let Json(input) = body.map_err(|_| ApiError::bad_request("squad is required"))?;
    if input.squad.is_empty() {
        return Err(ApiError::bad_request("squad is required"));
    }
    let me = &leader.0.discord_id;

    let n: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM orbat_slots WHERE event_mission_id = $1 AND squad = $2",
    )
    .bind(em.id)
    .bind(&input.squad)
    .fetch_one(&state.pool)
    .await?;
    if n == 0 {
        return Err(ApiError::not_found("squad not found in this ORBAT"));
    }

    let existing: Option<OrbatReservation> = sqlx::query_as(
        "SELECT * FROM orbat_reservations WHERE event_mission_id = $1 AND squad = $2",
    )
    .bind(em.id)
    .bind(&input.squad)
    .fetch_optional(&state.pool)
    .await?;
    if let Some(existing) = existing {
        if existing.reserved_by != *me {
            return Err(ApiError::conflict("squad is already reserved"));
        }
        return Ok((StatusCode::OK, Json(existing)).into_response());
    }

    let res: OrbatReservation = sqlx::query_as(
        "INSERT INTO orbat_reservations (event_mission_id, squad, reserved_by) VALUES ($1, $2, $3) RETURNING *",
    )
    .bind(em.id)
    .bind(&input.squad)
    .bind(me)
    .fetch_one(&state.pool)
    .await?;
    Ok((StatusCode::CREATED, Json(res)).into_response())
}

/// `POST /api/v1/event-missions/:emid/squads/release` — lift a squad hold (leader/admin).
///
/// @route POST /api/v1/event-missions/:emid/squads/release
pub async fn release_squad(
    State(state): State<AppState>,
    leader: LeaderUser,
    Path(emid): Path<String>,
    body: Result<Json<SquadBody>, JsonRejection>,
) -> Result<Json<Value>, ApiError> {
    let em = load_em(&state.pool, &emid).await?;
    let Json(input) = body.map_err(|_| ApiError::bad_request("squad is required"))?;
    if input.squad.is_empty() {
        return Err(ApiError::bad_request("squad is required"));
    }
    let res: Option<OrbatReservation> = sqlx::query_as(
        "SELECT * FROM orbat_reservations WHERE event_mission_id = $1 AND squad = $2",
    )
    .bind(em.id)
    .bind(&input.squad)
    .fetch_optional(&state.pool)
    .await?;
    let Some(res) = res else {
        return Err(ApiError::not_found("squad is not reserved"));
    };
    let is_admin = leader.0.role == "admin";
    if res.reserved_by != leader.0.discord_id && !is_admin {
        return Err(ApiError::forbidden(
            "only the reserver or an admin can release this squad",
        ));
    }
    sqlx::query("DELETE FROM orbat_reservations WHERE event_mission_id = $1 AND squad = $2")
        .bind(em.id)
        .bind(&input.squad)
        .execute(&state.pool)
        .await?;
    Ok(Json(json!({ "released": true })))
}
