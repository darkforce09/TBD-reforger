//! Machine credentials: the per-server, per-executor secrets that authenticate a game host's
//! control agent or a game runtime. The stored row never carries the secret itself.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::core::wire_format::{rfc3339_utc, rfc3339_utc_opt};

/// The program a credential authenticates on its server.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutorKind {
    /// The host process supervisor: process control and RCON delivery.
    HostAgent,
    /// The game runtime itself: runtime sessions, heartbeats, roster reads and deployments.
    ModRuntime,
}

impl ExecutorKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::HostAgent => "host_agent",
            Self::ModRuntime => "mod_runtime",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "host_agent" => Some(Self::HostAgent),
            "mod_runtime" => Some(Self::ModRuntime),
            _ => None,
        }
    }
}

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
