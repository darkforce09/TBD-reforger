//! The game-server roster read: the identity → slot map a running server seats players from.
//!
//! `mod_runtime` machine-credential tier: a runtime reads only the roster of an event bound to
//! its own server. The roster is the slot bindings of the deployment the server runs: the
//! compiled slot uid every ORBAT seat stands for was recorded when the deployment paired the
//! event mission's seats with its artifact, so the roster, the game and deployment authorization
//! all name the same artifact and nothing is compiled or paired at read time.

use axum::extract::{Path, State};
use axum::response::Json;
use uuid::Uuid;

use crate::core::application_state::AppState;
use crate::core::error_handling::api_error::ApiError;
use crate::missions::services::mission_deployments::deployment_reads::deployment_in_effect;
use crate::missions::services::mission_deployments::deployment_settlement::lock_and_settle;
use crate::operations::models::game_runtime_roster::{EventRoster, RosterAssignment, RosterSlot};
use crate::operations::services::event_lookup::load_event;
use crate::server_infrastructure::models::machine_credential::ExecutorKind;
use crate::server_infrastructure::services::machine_credentials::MachineCaller;

/// `GET /api/v1/game-runtime/events/:id/roster` — identity → slot map for a running event
/// (`mod_runtime` machine credential of the event's bound server).
///
/// The roster lists the bindings of the deployment the server runs — the one in flight, else the
/// latest confirmed one — when that deployment runs a mission of this event; otherwise it lists
/// nothing. Every slot carries the `orbatSlotId` and `eventMissionId` a deployment request
/// names.
///
/// ══ THE KEY IS `users.arma_id`, AND THAT IS LOAD-BEARING ═══════════════════════════════
/// The mod looks a player up with `TBD_RosterLoader.GetSlotForIdentity(bindKey)`, where
/// `bindKey` is the raw engine identity UUID `TBD_PlayerIdentity.GetArmaId` puts on every wire,
/// and the only thing besides the dev seed that ever writes `users.arma_id` is
/// `POST /api/v1/ingest/link-confirm`
/// ([`crate::identity_and_access::handlers::arma_link_confirmation::ingest_link_confirm`]) writing
/// exactly that value. Any other column — `discord_id`, `arma_character`, the `orbat_slots` UUID —
/// would match nobody and every player would fall through to round-robin seating. The key is
/// filtered and emitted as `btrim(u.arma_id)`, so a whitespace-only identity never seats and the
/// map key is the trimmed value link-confirm and telemetry use.
///
/// `assigned_to` is the seat claim itself: every writer sets it together with the matching
/// `event_registrations` row, so reading it covers leader-assigned and self-registered seats
/// alike and never serves a waitlisted player a seat.
///
/// @route GET /api/v1/game-runtime/events/:id/roster
pub async fn event_roster(
    State(state): State<AppState>,
    caller: MachineCaller,
    Path(id): Path<String>,
) -> Result<Json<EventRoster>, ApiError> {
    caller.require_executor(ExecutorKind::ModRuntime)?;
    let ev = load_event(&state.pool, &id).await?;
    match ev.server_id {
        Some(server) => caller.require_server(server)?,
        None => return Err(ApiError::forbidden("the event is not bound to this server")),
    }
    let mut transaction = state.pool.begin().await?;
    lock_and_settle(&mut transaction, caller.server_id).await?;
    let running = deployment_in_effect(&mut transaction, caller.server_id)
        .await?
        .filter(|deployment| deployment.event_id == Some(ev.id));
    let Some(deployment) = running else {
        transaction.commit().await?;
        return Ok(Json(EventRoster {
            version: 2,
            event_id: ev.id,
            mission_id: String::new(),
            assignments: Vec::new(),
            slots: Vec::new(),
        }));
    };
    let slots: Vec<(Uuid, String, Uuid)> = sqlx::query_as(
        "SELECT os.event_mission_id, b.slot_uid, b.orbat_slot_id
         FROM mission_deployment_slots b JOIN orbat_slots os ON os.id = b.orbat_slot_id
         WHERE b.deployment_id = $1 ORDER BY os.faction, os.squad, os.slot_index, os.id",
    )
    .bind(deployment.deployment_id)
    .fetch_all(&mut *transaction)
    .await?;
    let claims: Vec<(String, String, Uuid, Uuid)> = sqlx::query_as(
        "SELECT btrim(u.arma_id), b.slot_uid, b.orbat_slot_id, os.event_mission_id
         FROM mission_deployment_slots b JOIN orbat_slots os ON os.id = b.orbat_slot_id
         JOIN users u ON u.discord_id = os.assigned_to
         WHERE b.deployment_id = $1 AND os.assigned_to IS NOT NULL
           AND u.arma_id IS NOT NULL AND btrim(u.arma_id) <> '' AND u.deleted_at IS NULL
         ORDER BY btrim(u.arma_id)",
    )
    .bind(deployment.deployment_id)
    .fetch_all(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(Json(EventRoster {
        version: 2,
        event_id: ev.id,
        mission_id: deployment.mission_id.to_string(),
        assignments: claims
            .into_iter()
            .map(
                |(arma_id, slot_uid, orbat_slot_id, event_mission_id)| RosterAssignment {
                    arma_id,
                    slot_uid,
                    orbat_slot_id,
                    event_mission_id,
                },
            )
            .collect(),
        slots: slots
            .into_iter()
            .map(|(event_mission_id, slot_uid, orbat_slot_id)| RosterSlot {
                event_mission_id,
                slot_uid,
                orbat_slot_id,
            })
            .collect(),
    }))
}

#[cfg(test)]
#[path = "tests/game_runtime_roster.rs"]
mod tests;
