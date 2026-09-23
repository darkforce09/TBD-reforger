//! Requesting and cancelling deployments. A request locks the server, settles its deployment in
//! flight, validates the selection, confirms the requester's authority after every lock wait, and
//! records the deployment, its slot bindings, its fleet command and the audit record in one
//! transaction. Lock order: server, mission, event mission, requester account.

use serde_json::json;
use sqlx::PgConnection;
use uuid::Uuid;

use super::deployment_reads::load_deployment;
use super::deployment_selection::{ValidatedSelection, validate_selection};
use super::deployment_settlement::settle_server_deployment;
use crate::administration::services::required_audit::append_actor_audit;
use crate::core::configuration::Config;
use crate::core::error_handling::api_error::ApiError;
use crate::core::middleware::{AuthUser, role_rank};
use crate::identity_and_access::services::account_authority::holds_administrator_authority;
use crate::identity_and_access::services::identity_ownership::lock_accounts;
use crate::identity_and_access::services::session_authorization::authorize_on_connection;
use crate::missions::models::mission_deployment::{DeploymentTransition, MissionDeployment};
use crate::server_infrastructure::models::fleet_command::FleetAction;
use crate::server_infrastructure::services::fleet_commands::command_ledger::{
    cancel_command, enqueue_deployment_command,
};

/// Who asks for a deployment and through which door.
pub enum Requester<'a> {
    /// An administrator on the website; the session and role are re-checked after the lock
    /// waits.
    Administrator(&'a AuthUser),
    /// An in-game administrator's selection relayed by the server's runtime, naming the Arma
    /// identity that made it.
    InGame { arma_id: &'a str },
}

fn conflict(code: &str, message: &str, details: serde_json::Value) -> ApiError {
    let mut details = details;
    details["code"] = json!(code);
    ApiError::with_details(axum::http::StatusCode::CONFLICT, message, details)
}

fn not_an_administrator() -> ApiError {
    ApiError::with_details(
        axum::http::StatusCode::FORBIDDEN,
        "only a platform administrator can deploy a mission",
        json!({ "code": "NOT_AN_ADMINISTRATOR" }),
    )
}

/// The account behind the request, locked and holding administrator authority now.
async fn authorize_requester(
    connection: &mut PgConnection,
    requester: &Requester<'_>,
    config: &Config,
) -> Result<(String, &'static str), ApiError> {
    match requester {
        Requester::Administrator(user) => {
            lock_accounts(connection, std::slice::from_ref(&user.discord_id)).await?;
            let current = authorize_on_connection(connection, config, &user.session_claims).await?;
            if role_rank(&current.role) < role_rank("admin") {
                return Err(not_an_administrator());
            }
            Ok((current.discord_id, "web"))
        }
        Requester::InGame { arma_id } => {
            let linked: Option<String> = sqlx::query_scalar(
                "SELECT discord_id FROM users
                 WHERE arma_id IS NOT NULL AND btrim(arma_id) = btrim($1) AND btrim($1) <> ''
                   AND deleted_at IS NULL",
            )
            .bind(arma_id)
            .fetch_optional(&mut *connection)
            .await?;
            let account = linked.ok_or_else(|| {
                ApiError::with_details(
                    axum::http::StatusCode::FORBIDDEN,
                    "the in-game identity is not linked to a platform account",
                    json!({ "code": "IDENTITY_NOT_LINKED" }),
                )
            })?;
            lock_accounts(connection, std::slice::from_ref(&account)).await?;
            if !holds_administrator_authority(connection, &account, &config.discord_guild_id)
                .await?
            {
                return Err(not_an_administrator());
            }
            Ok((account, "game_runtime"))
        }
    }
}

async fn record(
    connection: &mut PgConnection,
    server: Uuid,
    selection: &ValidatedSelection,
    account: &str,
    via: &str,
) -> Result<Uuid, ApiError> {
    let deployment = Uuid::new_v4();
    let mut arguments = json!({
        "deployment_id": deployment,
        "artifact_id": selection.artifact_id,
        "artifact_sha256": selection.artifact_sha256,
    });
    let action = match selection.transition {
        DeploymentTransition::ScenarioRestart => {
            // The runtime whose terrain was validated performs it, or nobody: a command of an
            // ended session fails instead of reaching a runtime that may run another terrain.
            arguments["runtime_session_id"] = json!(selection.running_session);
            FleetAction::LoadMission
        }
        DeploymentTransition::HostRestart => {
            arguments["scenario_id"] = json!(selection.scenario_id);
            FleetAction::RestartWithMission
        }
    };
    let command =
        enqueue_deployment_command(connection, server, action, &arguments, account).await?;
    sqlx::query(
        "INSERT INTO mission_deployments (id, server_id, mission_id, artifact_id, event_mission_id,
             terrain_key, scenario_id, transition, fleet_command_id, requested_by, requested_via,
             deadline_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11,
             clock_timestamp() + make_interval(secs => $12))",
    )
    .bind(deployment)
    .bind(server)
    .bind(selection.mission_id)
    .bind(selection.artifact_id)
    .bind(selection.event_mission_id)
    .bind(&selection.terrain_key)
    .bind(&selection.scenario_id)
    .bind(selection.transition.as_str())
    .bind(command.id)
    .bind(account)
    .bind(via)
    .bind(selection.transition.deadline_seconds() as f64)
    .execute(&mut *connection)
    .await?;
    let (seats, uids): (Vec<Uuid>, Vec<String>) = selection.bindings.iter().cloned().unzip();
    sqlx::query(
        "INSERT INTO mission_deployment_slots (deployment_id, orbat_slot_id, slot_uid)
         SELECT $1, seat, uid FROM UNNEST($2::uuid[], $3::text[]) AS binding(seat, uid)",
    )
    .bind(deployment)
    .bind(&seats)
    .bind(&uids)
    .execute(&mut *connection)
    .await?;
    append_actor_audit(
        connection,
        account,
        "mission.deployment_requested",
        "mission_deployment",
        &deployment.to_string(),
        &format!(
            "Requested deployment of artifact {} on server {server} as a {} ({} bound seats, via {via})",
            selection.artifact_id,
            selection.transition.as_str(),
            selection.bindings.len()
        ),
    )
    .await?;
    Ok(deployment)
}

/// Request a deployment of `artifact` of `mission` on `server`.
pub async fn request_deployment(
    connection: &mut PgConnection,
    server: Uuid,
    mission: Uuid,
    artifact: Uuid,
    event_mission: Option<Uuid>,
    requester: Requester<'_>,
    config: &Config,
) -> Result<MissionDeployment, ApiError> {
    let locked: Option<(bool, Option<Uuid>)> = sqlx::query_as(
        "SELECT is_active, required_modpack_id FROM servers WHERE id = $1 FOR NO KEY UPDATE",
    )
    .bind(server)
    .fetch_optional(&mut *connection)
    .await?;
    let (active, required_modpack) =
        locked.ok_or_else(|| ApiError::not_found("server not found"))?;
    if !active {
        return Err(conflict(
            "SERVER_INACTIVE",
            "the server is deactivated",
            json!({}),
        ));
    }
    settle_server_deployment(connection, server).await?;
    let in_flight: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM mission_deployments WHERE server_id = $1 AND state = 'requested'",
    )
    .bind(server)
    .fetch_optional(&mut *connection)
    .await?;
    if let Some(in_flight) = in_flight {
        return Err(conflict(
            "DEPLOYMENT_IN_PROGRESS",
            "another deployment of this server is in flight",
            json!({ "deployment_id": in_flight }),
        ));
    }
    let selection = validate_selection(
        connection,
        server,
        required_modpack,
        mission,
        artifact,
        event_mission,
    )
    .await?;
    let (account, via) = authorize_requester(connection, &requester, config).await?;
    let deployment = record(connection, server, &selection, &account, via).await?;
    load_deployment(connection, server, deployment).await
}

/// Cancel a deployment whose fleet command no executor has claimed yet.
pub async fn cancel_deployment(
    connection: &mut PgConnection,
    server: Uuid,
    deployment: Uuid,
    administrator: &AuthUser,
    config: &Config,
) -> Result<MissionDeployment, ApiError> {
    sqlx::query_scalar::<_, Uuid>("SELECT id FROM servers WHERE id = $1 FOR NO KEY UPDATE")
        .bind(server)
        .fetch_optional(&mut *connection)
        .await?
        .ok_or_else(|| ApiError::not_found("server not found"))?;
    settle_server_deployment(connection, server).await?;
    let current: Option<(String, Uuid)> = sqlx::query_as(
        "SELECT state, fleet_command_id FROM mission_deployments
         WHERE id = $1 AND server_id = $2 FOR NO KEY UPDATE",
    )
    .bind(deployment)
    .bind(server)
    .fetch_optional(&mut *connection)
    .await?;
    let (state, command) = current.ok_or_else(|| ApiError::not_found("deployment not found"))?;
    if state != "requested" {
        return Err(conflict(
            "DEPLOYMENT_NOT_IN_FLIGHT",
            "only a deployment in flight can be cancelled",
            json!({ "state": state }),
        ));
    }
    let (actor, _) =
        authorize_requester(connection, &Requester::Administrator(administrator), config).await?;
    cancel_command(connection, server, command, &actor).await?;
    sqlx::query(
        "UPDATE mission_deployments SET state = 'cancelled', finished_at = clock_timestamp(),
             failure_reason = 'cancelled by an administrator'
         WHERE id = $1",
    )
    .bind(deployment)
    .execute(&mut *connection)
    .await?;
    append_actor_audit(
        connection,
        &actor,
        "mission.deployment_cancelled",
        "mission_deployment",
        &deployment.to_string(),
        "Cancelled the deployment before its command was claimed",
    )
    .await?;
    load_deployment(connection, server, deployment).await
}
