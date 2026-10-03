//! Machine credentials: the per-server, per-executor secrets that authenticate a game host's
//! control agent or a game runtime. The stored row never carries the secret itself; the executor
//! kind a credential authenticates is `fleet_wire_contract::ExecutorKind`.
//!
//! @contract machine-credential.schema.json#
//! @contract machine-credential.schema.json#/definitions/MachineCredential
//! @contract machine-credential.schema.json#/definitions/MachineCredentialIssue
//! @contract machine-credential.schema.json#/definitions/MachineCredentialList

use api_identifiers::{MachineCredentialId, ServerId};
use chrono::{DateTime, Utc};
use fleet_wire_contract::ExecutorKind;
use serde::{Deserialize, Serialize};

use fleet_wire_contract::rfc3339_timestamps::{rfc3339_utc, rfc3339_utc_opt};

/// One issued credential as administrators see it: provenance, use and revocation, no secret.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct MachineCredential {
    /// The credential's id, also the first part of its secret.
    pub id: MachineCredentialId,
    /// The server the credential acts for.
    pub server_id: ServerId,
    /// The executor kind it authenticates (`host_agent` or `mod_runtime`).
    pub executor_kind: String,
    /// The label administrators gave it.
    pub label: String,
    /// The administrator who issued it.
    pub created_by: String,
    /// When it was issued.
    #[serde(with = "rfc3339_utc")]
    pub created_at: DateTime<Utc>,
    /// When it last authenticated a request, when it ever did.
    #[serde(with = "rfc3339_utc_opt", skip_serializing_if = "Option::is_none")]
    pub last_used_at: Option<DateTime<Utc>>,
    /// When it was revoked, when it was.
    #[serde(with = "rfc3339_utc_opt", skip_serializing_if = "Option::is_none")]
    pub revoked_at: Option<DateTime<Utc>>,
    /// The administrator who revoked it, when it was revoked.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revoked_by: Option<String>,
    /// Why it was revoked, when it was revoked.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revoke_reason: Option<String>,
}

/// `POST /servers/{id}/credentials` body.
#[derive(Debug, Deserialize)]
pub struct MachineCredentialIssue {
    /// The executor kind the credential authenticates.
    pub executor_kind: ExecutorKind,
    /// The label administrators give it.
    pub label: String,
}

/// `DELETE /servers/{id}/credentials/{credentialId}` query.
#[derive(Debug, Deserialize)]
pub struct MachineCredentialRevocation {
    /// Why the credential is revoked; recorded with the revocation.
    pub reason: String,
}

/// The issue response: the stored credential and its secret, which is never shown again.
#[derive(Debug, Serialize)]
pub struct IssuedMachineCredential {
    /// The stored credential.
    pub credential: MachineCredential,
    /// The secret, returned only in this answer.
    pub secret: String,
}

/// `GET /servers/{id}/credentials` response.
#[derive(Debug, Serialize)]
pub struct MachineCredentialList {
    /// The server's credentials, revoked ones included.
    pub items: Vec<MachineCredential>,
}
