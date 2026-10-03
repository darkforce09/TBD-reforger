//! The executor side of the fleet command ledger: the claimed command and the two reports.
//!
//! **Role:** the answer of `POST /fleet-executor/commands/claim` and the bodies of the
//! `executing` and `result` reports that follow it.
//! **Position:** the API writes [`ClaimedFleetCommand`] and reads [`ExecutionStart`] and
//! [`ExecutionResult`]; the host agent reads the claim and writes the two reports, so each shape
//! crosses the wire in both directions through one type.
//! **Signals & state:** none; plain data.
//! **Invariants:** both reports carry the claim's fencing token and admit no unknown key; an
//! absent outcome or failure reason is an absent key, never null, and reading one accepts null or
//! an absent key as `None`; the lease instant uses the spelling of [`crate::rfc3339_timestamps`].
//!
//! @contract fleet-command.schema.json#/definitions/ClaimedFleetCommand
//! @contract fleet-command.schema.json#/definitions/ExecutionStart
//! @contract fleet-command.schema.json#/definitions/ExecutionResult

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use uuid::Uuid;

use crate::rfc3339_timestamps::rfc3339_utc;

/// A claimed command as its executor receives it, with the fencing token every later report
/// must carry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClaimedFleetCommand {
    /// The claimed command.
    pub command_id: Uuid,
    /// The server the command addresses.
    pub server_id: Uuid,
    /// The action, spelled as [`crate::fleet_action::FleetAction::as_str`].
    pub action: String,
    /// The stored arguments.
    pub arguments: Value,
    /// The token every report of this claim carries.
    pub fencing_token: i64,
    /// The claim returns to the queue unless `executing` is reported before this instant.
    #[serde(with = "rfc3339_utc")]
    pub lease_expires_at: DateTime<Utc>,
}

/// `POST /fleet-executor/commands/{commandId}/executing` body.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionStart {
    /// The claim's fencing token.
    pub fencing_token: i64,
}

/// `POST /fleet-executor/commands/{commandId}/result` body.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionResult {
    /// The claim's fencing token.
    pub fencing_token: i64,
    /// Whether the effect happened as asked.
    pub succeeded: bool,
    /// What the executor observed, such as a player list or a console reply.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outcome: Option<Map<String, Value>>,
    /// Why the effect failed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure_reason: Option<String>,
}

#[cfg(test)]
#[path = "tests/executor_messages.rs"]
mod tests;
