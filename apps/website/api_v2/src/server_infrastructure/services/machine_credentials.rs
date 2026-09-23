//! Issue, list, revoke and verify machine credentials.
//!
//! A secret reads `tbdm_<credential id>_<64 hex digits of randomness>`. The id selects the row;
//! the SHA-256 of the complete secret is compared in constant time with the stored digest, so a
//! database read never yields a usable credential. Each credential authenticates one executor of
//! one server, and every operation a caller requests is checked against that server.

use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use crate::administration::services::required_audit::append_actor_audit;
use crate::core::authentication_primitives::{constant_time_equal, hash_token, random_token};
use crate::core::error_handling::api_error::ApiError;
use crate::server_infrastructure::models::machine_credential::{
    ExecutorKind, IssuedMachineCredential, MachineCredential,
};
use crate::server_infrastructure::services::runtime_sessions::{
    SessionEndReason, end_sessions_of_credential,
};

const SECRET_PREFIX: &str = "tbdm_";
const CREDENTIAL_COLUMNS: &str = "id, server_id, executor_kind, label, created_by, created_at, \
     last_used_at, revoked_at, revoked_by, revoke_reason";

/// An authenticated machine: the credential, the only server it may act for, and its executor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MachineCaller {
    pub credential_id: Uuid,
    pub server_id: Uuid,
    pub executor: ExecutorKind,
}

impl MachineCaller {
    pub fn require_executor(&self, executor: ExecutorKind) -> Result<(), ApiError> {
        if self.executor == executor {
            Ok(())
        } else {
            Err(ApiError::forbidden(format!(
                "this operation requires a {} credential",
                executor.as_str()
            )))
        }
    }

    /// Credentials never act for another server, whatever the requested resource names.
    pub fn require_server(&self, server_id: Uuid) -> Result<(), ApiError> {
        if self.server_id == server_id {
            Ok(())
        } else {
            Err(ApiError::forbidden(
                "the credential belongs to another server",
            ))
        }
    }
}

/// The credential id a well-formed secret names.
fn secret_credential_id(secret: &str) -> Option<Uuid> {
    let rest = secret.strip_prefix(SECRET_PREFIX)?;
    let (id, random) = rest.split_once('_')?;
    let well_formed = id.len() == 32
        && random.len() == 64
        && random
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));
    well_formed.then(|| Uuid::try_parse(id).ok()).flatten()
}

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
    server_id: Uuid,
    executor: ExecutorKind,
    label: &str,
    actor: &str,
) -> Result<IssuedMachineCredential, ApiError> {
    let label = validated_text(label, "label", 128)?;
    let id = Uuid::new_v4();
    let secret = format!("{SECRET_PREFIX}{}_{}", id.simple(), random_token(32));
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

pub async fn list_machine_credentials(
    pool: &PgPool,
    server_id: Uuid,
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
    server_id: Uuid,
    credential_id: Uuid,
    actor: &str,
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

#[derive(sqlx::FromRow)]
struct StoredCredential {
    server_id: Uuid,
    executor_kind: String,
    secret_sha256: String,
    revoked: bool,
    server_active: bool,
}

/// Verify a presented secret. Malformed, unknown and mismatched secrets are indistinguishable.
pub async fn authenticate_machine(pool: &PgPool, secret: &str) -> Result<MachineCaller, ApiError> {
    let invalid = || ApiError::unauthorized("invalid machine credential");
    let credential_id = secret_credential_id(secret).ok_or_else(invalid)?;
    let stored: StoredCredential = sqlx::query_as(
        "SELECT credential.server_id, credential.executor_kind, credential.secret_sha256,
             credential.revoked_at IS NOT NULL AS revoked, server.is_active AS server_active
         FROM server_machine_credentials credential
         JOIN servers server ON server.id = credential.server_id
         WHERE credential.id = $1",
    )
    .bind(credential_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(invalid)?;
    if !constant_time_equal(&hash_token(secret), &stored.secret_sha256) {
        return Err(invalid());
    }
    if stored.revoked {
        return Err(ApiError::unauthorized("machine credential revoked"));
    }
    if !stored.server_active {
        return Err(ApiError::forbidden(
            "the credential's server is deactivated",
        ));
    }
    let executor = ExecutorKind::parse(&stored.executor_kind)
        .ok_or_else(|| ApiError::internal("unknown executor kind"))?;
    // Use is recorded at minute resolution so a heartbeat stream does not rewrite the row.
    sqlx::query(
        "UPDATE server_machine_credentials SET last_used_at = clock_timestamp()
         WHERE id = $1 AND revoked_at IS NULL
           AND (last_used_at IS NULL OR last_used_at < clock_timestamp() - interval '1 minute')",
    )
    .bind(credential_id)
    .execute(pool)
    .await?;
    Ok(MachineCaller {
        credential_id,
        server_id: stored.server_id,
        executor,
    })
}

#[cfg(test)]
#[path = "tests/machine_credentials.rs"]
mod tests;
