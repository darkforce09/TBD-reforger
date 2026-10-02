//! Deployment authorization and live slot occupancy for game runtimes.
//!
//! A deployment is allowed when the player's own active reservation is for the slot, or when the
//! slot is unreserved and its effective policy admits the player under current membership
//! authority. Either way the identity must be linked, the account available and the slot free of
//! any other open life. Occupancy is independent of reservations: releasing a reservation never
//! ends a life, it only refuses the next deployment. An allowed decision is recorded under the
//! runtime's life id, so a retried request returns it unchanged.
//!
//! Lock order: event (share), attachment (share), identity, account, slot, runtime session
//! (share). Reservation writers take event, attachment and account locks exclusively and in the
//! same order, and session writers never take any of the others.

use sqlx::PgConnection;
use uuid::Uuid;

use crate::core::error_handling::api_error::ApiError;
use crate::identity_and_access::services::identity_ownership::{lock_accounts, lock_identities};
use crate::operations::models::live_occupancy::{
    DeploymentAuthority, DeploymentDecision, DeploymentDenial, DeploymentRequest, EndedLife,
    LiveOccupancy,
};
use crate::operations::services::event_access::context::EventAccessContext;
use crate::operations::services::event_access::slot_eligibility::PolicySlot;
use crate::server_infrastructure::services::machine_credentials::MachineCaller;
use crate::server_infrastructure::services::runtime_sessions::share_open_session;

const OCCUPANCY_COLUMNS: &str = "id AS occupancy_id, runtime_session_id, event_mission_id, \
     orbat_slot_id, arma_id, player_life_id, authorized_by, started_at";

fn bounded(raw: &str, field: &str) -> Result<String, ApiError> {
    let value = raw.trim();
    if value.is_empty() || value.len() > 128 {
        return Err(ApiError::bad_request(format!(
            "{field} must contain 1 to 128 bytes"
        )));
    }
    Ok(value.to_owned())
}

/// The decision already recorded for this life, if any. A reused life id naming another slot,
/// mission or player is a runtime defect and is refused.
async fn recorded_decision(
    connection: &mut PgConnection,
    session: Uuid,
    life: &str,
    request: &DeploymentRequest,
    arma_id: &str,
) -> Result<Option<DeploymentDecision>, ApiError> {
    let recorded: Option<LiveOccupancy> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT {OCCUPANCY_COLUMNS} FROM live_slot_occupancies
         WHERE runtime_session_id = $1 AND player_life_id = $2"
    )))
    .bind(session)
    .bind(life)
    .fetch_optional(connection)
    .await?;
    match recorded {
        None => Ok(None),
        Some(occupancy)
            if occupancy.event_mission_id == request.event_mission_id
                && occupancy.orbat_slot_id == request.orbat_slot_id
                && occupancy.arma_id == arma_id =>
        {
            Ok(Some(DeploymentDecision::Allowed(occupancy)))
        }
        Some(_) => Err(ApiError::conflict(
            "the player life id was already used for another deployment",
        )),
    }
}

/// Decide, and when allowed record, one deployment for the caller's runtime session.
pub async fn authorize_deployment(
    connection: &mut PgConnection,
    caller: &MachineCaller,
    session: Uuid,
    request: &DeploymentRequest,
    main_guild: &str,
) -> Result<DeploymentDecision, ApiError> {
    let arma_id = bounded(&request.arma_id, "arma_id")?;
    let life = bounded(&request.player_life_id, "player_life_id")?;
    let attachment: Option<(Uuid, Option<Uuid>)> = sqlx::query_as(
        "SELECT event_row.id, event_row.server_id FROM event_missions attachment
         JOIN events event_row ON event_row.id = attachment.event_id
         WHERE attachment.id = $1 AND attachment.deleted_at IS NULL AND event_row.deleted_at IS NULL",
    )
    .bind(request.event_mission_id)
    .fetch_optional(&mut *connection)
    .await?;
    let (event_id, server) =
        attachment.ok_or_else(|| ApiError::not_found("event mission not found"))?;
    caller.require_server(
        server.ok_or_else(|| ApiError::forbidden("the event is not bound to this server"))?,
    )?;

    sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM events WHERE id = $1 AND deleted_at IS NULL FOR SHARE",
    )
    .bind(event_id)
    .fetch_optional(&mut *connection)
    .await?
    .ok_or_else(|| ApiError::not_found("event mission not found"))?;
    sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM event_missions WHERE id = $1 AND deleted_at IS NULL FOR SHARE",
    )
    .bind(request.event_mission_id)
    .fetch_optional(&mut *connection)
    .await?
    .ok_or_else(|| ApiError::not_found("event mission not found"))?;
    lock_identities(connection, &[arma_id.as_str()]).await?;
    let account: Option<String> = sqlx::query_scalar(
        "SELECT discord_id FROM users WHERE arma_id = $1 AND deleted_at IS NULL",
    )
    .bind(&arma_id)
    .fetch_optional(&mut *connection)
    .await?;
    if let Some(account) = &account {
        lock_accounts(connection, std::slice::from_ref(account)).await?;
    }
    let slot: PolicySlot = sqlx::query_as(
        "SELECT id, event_mission_id, faction, squad, slot_index, assigned_to FROM orbat_slots
         WHERE id = $1 AND event_mission_id = $2 FOR NO KEY UPDATE",
    )
    .bind(request.orbat_slot_id)
    .bind(request.event_mission_id)
    .fetch_optional(&mut *connection)
    .await?
    .ok_or_else(|| ApiError::not_found("slot not found in event mission"))?;
    share_open_session(connection, caller, session).await?;

    if let Some(decision) = recorded_decision(connection, session, &life, request, &arma_id).await?
    {
        return Ok(decision);
    }
    // Game loading, roster derivation and authorization name one artifact: the seat must be
    // bound by a deployment of the artifact this runtime session reported loading.
    let loaded_seat: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM server_runtime_sessions s
             JOIN mission_deployments d ON d.server_id = s.server_id
                 AND d.artifact_id = s.loaded_artifact_id AND d.state IN ('requested', 'confirmed')
             JOIN mission_deployment_slots b ON b.deployment_id = d.id
             WHERE s.id = $1 AND b.orbat_slot_id = $2)",
    )
    .bind(session)
    .bind(slot.id)
    .fetch_one(&mut *connection)
    .await?;
    if !loaded_seat {
        return Ok(DeploymentDecision::denied(
            DeploymentDenial::SlotNotInLoadedMission,
        ));
    }
    let Some(account) = account else {
        return Ok(DeploymentDecision::denied(
            DeploymentDenial::IdentityNotLinked,
        ));
    };
    let authority =
        match deployment_authority(connection, event_id, &slot, &account, main_guild).await? {
            Ok(authority) => authority,
            Err(denial) => return Ok(DeploymentDecision::denied(denial)),
        };
    let occupied: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM live_slot_occupancies WHERE orbat_slot_id = $1 AND ended_at IS NULL)",
    )
    .bind(slot.id)
    .fetch_one(&mut *connection)
    .await?;
    if occupied {
        return Ok(DeploymentDecision::denied(
            DeploymentDenial::LiveSlotOccupied,
        ));
    }
    let deployed: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM live_slot_occupancies
         WHERE runtime_session_id = $1 AND arma_id = $2 AND ended_at IS NULL",
    )
    .bind(session)
    .bind(&arma_id)
    .fetch_optional(&mut *connection)
    .await?;
    if let Some(open) = deployed {
        return Ok(DeploymentDecision::Denied {
            reason: DeploymentDenial::PlayerAlreadyDeployed,
            message: DeploymentDenial::PlayerAlreadyDeployed.message().to_owned(),
            occupancy_id: Some(open),
        });
    }
    let occupancy: LiveOccupancy = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "INSERT INTO live_slot_occupancies
             (runtime_session_id, event_mission_id, orbat_slot_id, arma_id, discord_id, player_life_id, authorized_by)
         VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING {OCCUPANCY_COLUMNS}"
    )))
    .bind(session)
    .bind(request.event_mission_id)
    .bind(slot.id)
    .bind(&arma_id)
    .bind(&account)
    .bind(&life)
    .bind(authority.as_str())
    .fetch_one(&mut *connection)
    .await?;
    Ok(DeploymentDecision::Allowed(occupancy))
}

/// Whether the player's reservation or the slot's policy entitles the player to the slot.
/// The caller holds the account and slot locks.
async fn deployment_authority(
    connection: &mut PgConnection,
    event_id: Uuid,
    slot: &PolicySlot,
    account: &str,
    main_guild: &str,
) -> Result<Result<DeploymentAuthority, DeploymentDenial>, ApiError> {
    let context = EventAccessContext::load(connection, event_id).await?;
    let facts = context
        .account_facts(connection, &[account.to_owned()], main_guild)
        .await?
        .remove(account)
        .ok_or_else(|| ApiError::internal("locked account has no facts"))?;
    if !facts.available {
        return Ok(Err(DeploymentDenial::AccountUnavailable));
    }
    let own: Option<Option<Uuid>> = sqlx::query_scalar(
        "SELECT slot_id FROM event_registrations
         WHERE event_mission_id = $1 AND discord_id = $2 AND reservation_state IN ('registered', 'legacy_unknown')",
    )
    .bind(slot.event_mission_id)
    .bind(account)
    .fetch_optional(&mut *connection)
    .await?;
    if slot.assigned_to.as_deref() == Some(account) {
        return Ok(Ok(DeploymentAuthority::Reservation));
    }
    if let Some(Some(_)) = own {
        return Ok(Err(DeploymentDenial::ReservedAnotherSlot));
    }
    let claimed: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM event_registrations WHERE event_mission_id = $1 AND slot_id = $2
             AND reservation_state IN ('registered', 'legacy_unknown'))",
    )
    .bind(slot.event_mission_id)
    .bind(slot.id)
    .fetch_one(&mut *connection)
    .await?;
    if slot.assigned_to.is_some() || claimed {
        return Ok(Err(DeploymentDenial::SlotReserved));
    }
    if context.slot_admits(slot, &facts.current) {
        return Ok(Ok(DeploymentAuthority::OpenSlotPolicy));
    }
    let (policy, _) = context.effective_slot_policy(slot);
    Ok(Err(if facts.pending_evidence_admits(policy, main_guild) {
        DeploymentDenial::MembershipVerificationRequired
    } else {
        DeploymentDenial::AccessPolicy
    }))
}

/// End exactly one life of the caller's session. A delayed or repeated end names its own
/// occupancy, so it can never end a newer life in the same slot.
pub async fn end_life(
    connection: &mut PgConnection,
    caller: &MachineCaller,
    session: Uuid,
    occupancy: Uuid,
) -> Result<EndedLife, ApiError> {
    let (server_id,): (Uuid,) =
        sqlx::query_as("SELECT server_id FROM server_runtime_sessions WHERE id = $1 FOR SHARE")
            .bind(session)
            .fetch_optional(&mut *connection)
            .await?
            .ok_or_else(|| ApiError::not_found("runtime session not found"))?;
    caller.require_server(server_id)?;
    let current: Option<String> = sqlx::query_scalar::<_, Option<String>>(
        "SELECT end_reason FROM live_slot_occupancies WHERE id = $1 AND runtime_session_id = $2 FOR NO KEY UPDATE",
    )
    .bind(occupancy)
    .bind(session)
    .fetch_optional(&mut *connection)
    .await?
    .ok_or_else(|| ApiError::not_found("occupancy not found in runtime session"))?;
    let end_reason = match current {
        Some(reason) => reason,
        None => {
            sqlx::query(
                "UPDATE live_slot_occupancies SET ended_at = clock_timestamp(), end_reason = 'life_ended'
                 WHERE id = $1",
            )
            .bind(occupancy)
            .execute(connection)
            .await?;
            "life_ended".to_owned()
        }
    };
    Ok(EndedLife {
        occupancy_id: occupancy,
        ended: true,
        end_reason,
    })
}
