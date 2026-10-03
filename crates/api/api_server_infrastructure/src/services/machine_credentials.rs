//! Issue, list and revoke machine credentials.
//!
//! A secret reads `tbdm_<credential id>_<64 hex digits of randomness>` and is returned exactly
//! once, at issue; only its SHA-256 is stored. Verification of a presented secret lives in
//! [`api_caller_identity::machine_caller`].

use api_identifiers::{DiscordUserId, MachineCredentialId, ServerId};
use fleet_wire_contract::ExecutorKind;
use fleet_wire_contract::machine_credential_format::MACHINE_CREDENTIAL_PREFIX;
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use crate::models::machine_credential::{IssuedMachineCredential, MachineCredential};
use crate::services::runtime_sessions::{SessionEndReason, end_sessions_of_credential};
use api_audit_log::required_audit::append_actor_audit;
use api_foundation::error_handling::api_error::ApiError;
use api_http_layer::authentication_primitives::{hash_token, random_token};

const CREDENTIAL_COLUMNS: &str = "id, server_id, executor_kind, label, created_by, created_at, \
     last_used_at, revoked_at, revoked_by, revoke_reason";

fn validated_text(raw: &str, field: &str, max_bytes: usize) -> Result<String, ApiError> {
    let value = raw.trim();
    if value.is_empty() || value.len() > max_bytes {
        return Err(ApiError::bad_request(format!(
            "{field} must contain 1 to {max_bytes} bytes"
        )));
    }
    Ok(value.to_owned())
}

/// Issue a credential for a server the caller has locked. The secret is returned exactly once.
pub async fn issue_machine_credential(
    connection: &mut PgConnection,
    server_id: ServerId,
    executor: ExecutorKind,
    label: &str,
    actor: &DiscordUserId,
) -> Result<IssuedMachineCredential, ApiError> {
    let label = validated_text(label, "label", 128)?;
    let id = Uuid::new_v4();
    let secret = format!(
        "{MACHINE_CREDENTIAL_PREFIX}{}_{}",
        id.simple(),
        random_token(32)
    );
    let credential: MachineCredential = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "INSERT INTO server_machine_credentials (id, server_id, executor_kind, secret_sha256, label, created_by)
         VALUES ($1, $2, $3, $4, $5, $6) RETURNING {CREDENTIAL_COLUMNS}"
    )))
    .bind(id)
    .bind(server_id)
    .bind(executor.as_str())
    .bind(hash_token(&secret))
    .bind(&label)
    .bind(actor)
    .fetch_one(&mut *connection)
    .await?;
    append_actor_audit(
        connection,
        actor,
        "server.credential_issued",
        "server_machine_credential",
        &id.to_string(),
        &format!(
            "Issued {} credential '{label}' for server {server_id}",
            executor.as_str()
        ),
    )
    .await?;
    Ok(IssuedMachineCredential { credential, secret })
}

/// Every credential of `server_id`, revoked ones included, oldest first.
pub async fn list_machine_credentials(
    pool: &PgPool,
    server_id: ServerId,
) -> Result<Vec<MachineCredential>, ApiError> {
    Ok(sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT {CREDENTIAL_COLUMNS} FROM server_machine_credentials
         WHERE server_id = $1 ORDER BY created_at, id"
    )))
    .bind(server_id)
    .fetch_all(pool)
    .await?)
}

/// Revoke one credential of a locked server and end the runtime sessions it authenticated.
/// Revoking an already revoked credential returns it unchanged.
pub async fn revoke_machine_credential(
    connection: &mut PgConnection,
    server_id: ServerId,
    credential_id: MachineCredentialId,
    actor: &DiscordUserId,
    reason: &str,
) -> Result<MachineCredential, ApiError> {
    let reason = validated_text(reason, "reason", 512)?;
    let current: MachineCredential = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT {CREDENTIAL_COLUMNS} FROM server_machine_credentials
         WHERE id = $1 AND server_id = $2 FOR NO KEY UPDATE"
    )))
    .bind(credential_id)
    .bind(server_id)
    .fetch_optional(&mut *connection)
    .await?
    .ok_or_else(|| ApiError::not_found("credential not found"))?;
    if current.revoked_at.is_some() {
        return Ok(current);
    }
    let revoked: MachineCredential = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "UPDATE server_machine_credentials
         SET revoked_at = clock_timestamp(), revoked_by = $2, revoke_reason = $3
         WHERE id = $1 RETURNING {CREDENTIAL_COLUMNS}"
    )))
    .bind(credential_id)
    .bind(actor)
    .bind(&reason)
    .fetch_one(&mut *connection)
    .await?;
    let ended = end_sessions_of_credential(
        connection,
        credential_id,
        SessionEndReason::CredentialRevoked,
    )
    .await?;
    append_actor_audit(
        connection,
        actor,
        "server.credential_revoked",
        "server_machine_credential",
        &credential_id.to_string(),
        &format!(
            "Revoked credential of server {server_id}: {reason}; ended {ended} runtime session(s)"
        ),
    )
    .await?;
    Ok(revoked)
}

#[cfg(test)]
#[path = "tests/machine_credentials.rs"]
mod tests;
