//! Machine credentials: the per-server, per-executor secrets that authenticate a game host's
//! control agent or a game runtime. The stored row never carries the secret itself; the executor
//! kind a credential authenticates is `fleet_wire_contract::ExecutorKind`.
//!
//! @contract machine-credential.schema.json#
//! @contract machine-credential.schema.json#/definitions/MachineCredential
//! @contract machine-credential.schema.json#/definitions/MachineCredentialIssue
//! @contract machine-credential.schema.json#/definitions/MachineCredentialList

use chrono::{DateTime, Utc};
use fleet_wire_contract::ExecutorKind;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::core::wire_format::{rfc3339_utc, rfc3339_utc_opt};

/// One issued credential as administrators see it: provenance, use and revocation, no secret.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct MachineCredential {
    pub id: Uuid,
    pub server_id: Uuid,
    pub executor_kind: String,
    pub label: String,
    pub created_by: String,
    #[serde(with = "rfc3339_utc")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "rfc3339_utc_opt", skip_serializing_if = "Option::is_none")]
    pub last_used_at: Option<DateTime<Utc>>,
    #[serde(with = "rfc3339_utc_opt", skip_serializing_if = "Option::is_none")]
    pub revoked_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revoked_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revoke_reason: Option<String>,
}

/// `POST /servers/{id}/credentials` body.
#[derive(Debug, Deserialize)]
pub struct MachineCredentialIssue {
    pub executor_kind: ExecutorKind,
    pub label: String,
}

/// `DELETE /servers/{id}/credentials/{credentialId}` query.
#[derive(Debug, Deserialize)]
pub struct MachineCredentialRevocation {
    pub reason: String,
}

/// The issue response: the stored credential and its secret, which is never shown again.
#[derive(Debug, Serialize)]
pub struct IssuedMachineCredential {
    pub credential: MachineCredential,
    pub secret: String,
}

/// `GET /servers/{id}/credentials` response.
#[derive(Debug, Serialize)]
pub struct MachineCredentialList {
    pub items: Vec<MachineCredential>,
}
