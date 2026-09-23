//! Wire messages of the executor routes (`contracts_v2/definitions/fleet-command.schema.json`:
//! ClaimedFleetCommand, ExecutionStart, ExecutionResult) and the API's error envelope.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use uuid::Uuid;

use crate::action_verdict::ActionVerdict;

/// `POST /api/v1/fleet-executor/commands/claim` answer: one claimed command and the fencing
/// token every later report of it carries.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ClaimedFleetCommand {
    pub command_id: Uuid,
    pub server_id: Uuid,
    pub action: String,
    pub arguments: Value,
    pub fencing_token: i64,
    /// The claim returns to the queue unless `executing` is reported before this instant.
    pub lease_expires_at: DateTime<Utc>,
}

/// `POST .../commands/{commandId}/executing` body.
#[derive(Debug, Clone, Serialize)]
pub(super) struct ExecutionStart {
    pub(super) fencing_token: i64,
}

/// `POST .../commands/{commandId}/result` body. An absent outcome or failure reason is omitted
/// from the JSON rather than sent as null.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ExecutionResult {
    pub fencing_token: i64,
    pub succeeded: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outcome: Option<Map<String, Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure_reason: Option<String>,
}

impl ExecutionResult {
    pub fn from_verdict(fencing_token: i64, verdict: ActionVerdict) -> Self {
        let (succeeded, outcome, failure_reason) = verdict.into_parts();
        Self {
            fencing_token,
            succeeded,
            outcome,
            failure_reason,
        }
    }
}

/// The API's error body: `{"error": message, "details": {...}}`.
#[derive(Debug, Default, Deserialize)]
pub(super) struct ErrorEnvelope {
    #[serde(default)]
    pub(super) error: Option<String>,
    #[serde(default)]
    pub(super) details: Option<Value>,
}
