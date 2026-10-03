//! The operator side of the fleet command ledger: the command request, the receipt and the list.
//!
//! **Role:** the bodies of `POST /servers/{id}/commands` and the receipts every operator and
//! executor route answers with.
//! **Position:** the API reads [`FleetCommandRequest`] from an operator, builds
//! [`FleetCommandReceipt`] from its stored command row, and answers with a receipt or a
//! [`FleetCommandList`]; the contract tests and any reader decode the same shapes.
//! **Signals & state:** none; plain data.
//! **Invariants:** a request admits no unknown key and reads an absent `arguments` as `{}`; a
//! receipt writes an absent optional field as an absent key, never null, and its instants in the
//! spelling of [`crate::rfc3339_timestamps`].
//!
//! @contract fleet-command.schema.json#/definitions/FleetCommandRequest
//! @contract fleet-command.schema.json#/definitions/FleetCommandReceipt
//! @contract fleet-command.schema.json#/definitions/FleetCommandList

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use uuid::Uuid;

use crate::fleet_action::FleetAction;
use crate::rfc3339_timestamps::{rfc3339_utc, rfc3339_utc_opt};

/// `POST /servers/{id}/commands` body.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FleetCommandRequest {
    /// What the command asks the server to do.
    pub action: FleetAction,
    /// The action's arguments; absent reads as an empty object.
    #[serde(default)]
    pub arguments: Map<String, Value>,
}

/// The receipt of one command: everything an operator can observe about it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FleetCommandReceipt {
    /// The command.
    pub id: Uuid,
    /// The server the command addresses.
    pub server_id: Uuid,
    /// The executor that runs it, spelled as [`crate::executor_kind::ExecutorKind::as_str`].
    pub executor_kind: String,
    /// The action, spelled as [`FleetAction::as_str`].
    pub action: String,
    /// The stored arguments.
    pub arguments: Value,
    /// Who requested the command.
    pub requested_by: String,
    /// When the command was accepted.
    #[serde(with = "rfc3339_utc")]
    pub requested_at: DateTime<Utc>,
    /// When the command expires unless an executor claims it.
    #[serde(with = "rfc3339_utc")]
    pub expires_at: DateTime<Utc>,
    /// The command state, one of the schema's `state` values.
    pub state: String,
    /// How many claims the command has seen.
    pub attempts: i32,
    /// When the latest claim was taken.
    #[serde(
        default,
        with = "rfc3339_utc_opt",
        skip_serializing_if = "Option::is_none"
    )]
    pub claimed_at: Option<DateTime<Utc>>,
    /// When the executor reported the effect starting.
    #[serde(
        default,
        with = "rfc3339_utc_opt",
        skip_serializing_if = "Option::is_none"
    )]
    pub executing_at: Option<DateTime<Utc>>,
    /// When the command reached a final state.
    #[serde(
        default,
        with = "rfc3339_utc_opt",
        skip_serializing_if = "Option::is_none"
    )]
    pub finished_at: Option<DateTime<Utc>>,
    /// What the executor observed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outcome: Option<Value>,
    /// Why the command failed, expired or was cancelled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure_reason: Option<String>,
}

/// `GET /servers/{id}/commands` response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FleetCommandList {
    /// The server's commands, newest first.
    pub items: Vec<FleetCommandReceipt>,
}

#[cfg(test)]
#[path = "tests/operator_messages.rs"]
mod tests;
