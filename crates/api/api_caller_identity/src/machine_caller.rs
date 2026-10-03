//! Verify a machine credential and name the caller it authenticates.
//!
//! A secret reads `tbdm_<credential id>_<64 hex digits of randomness>`. The id selects the row;
//! the SHA-256 of the complete secret is compared in constant time with the stored digest, so a
//! database read never yields a usable credential. Each credential authenticates one executor of
//! one server, and every operation a caller requests is checked against that server.
//!
//! **Role:** turns a presented machine secret into a [`MachineCaller`], and holds the executor
//! and server checks every machine route applies to it.
//! **Position:** called by the [`crate::machine_authentication`]
//! extractor; the caller it yields is taken by the game-host and game-runtime handlers of match
//! telemetry, missions, operations, identity and server infrastructure.
//! **Signals & state:** `server_machine_credentials.last_used_at`, stamped at minute resolution.
//! **Invariants:** malformed, unknown and mismatched secrets are indistinguishable to the caller;
//! the credential id accepts upper- and lower-case hex digits, the random part lower-case only; a
//! revoked credential or a deactivated server never authenticates.

use api_identifiers::{MachineCredentialId, ServerId};
use fleet_wire_contract::ExecutorKind;
use fleet_wire_contract::machine_credential_format::{
    CREDENTIAL_ID_HEX_DIGITS, CREDENTIAL_RANDOM_HEX_DIGITS, MACHINE_CREDENTIAL_PREFIX,
};
use sqlx::PgPool;
use uuid::Uuid;

use api_foundation::error_handling::api_error::ApiError;
use api_http_layer::authentication_primitives::{constant_time_equal, hash_token};

/// An authenticated machine: the credential, the only server it may act for, and its executor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MachineCaller {
    /// The credential the secret named.
    pub credential_id: MachineCredentialId,
    /// The one server this caller may act for.
    pub server_id: ServerId,
    /// The executor kind the credential serves.
    pub executor: ExecutorKind,
}

impl MachineCaller {
    /// Refuse with 403 unless the credential authenticates `executor`.
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
    pub fn require_server(&self, server_id: ServerId) -> Result<(), ApiError> {
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
fn secret_credential_id(secret: &str) -> Option<MachineCredentialId> {
    let rest = secret.strip_prefix(MACHINE_CREDENTIAL_PREFIX)?;
    let (id, random) = rest.split_once('_')?;
    let well_formed = id.len() == CREDENTIAL_ID_HEX_DIGITS
        && random.len() == CREDENTIAL_RANDOM_HEX_DIGITS
        && random
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));
    well_formed
        .then(|| Uuid::try_parse(id).ok().map(MachineCredentialId::new))
        .flatten()
}

#[derive(sqlx::FromRow)]
struct StoredCredential {
    server_id: ServerId,
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
#[path = "tests/machine_caller.rs"]
mod tests;
