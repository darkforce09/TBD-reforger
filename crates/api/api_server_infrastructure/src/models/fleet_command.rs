//! Fleet commands as the ledger stores them: the command states and the stored row a receipt is
//! read from.
//!
//! **Role:** the [`FleetCommandState`] vocabulary of the `fleet_commands.state` column and
//! `FleetCommandReceiptRow`, the database row of one command, which converts into the wire
//! receipt [`fleet_wire_contract::operator_messages::FleetCommandReceipt`].
//! **Position:** read by the ledger in
//! [`crate::services::fleet_commands`]; the wire shapes, the action rules
//! and `ExecutorKind` live in the `fleet_wire_contract` crate, which the host agent shares;
//! `generated/fleet_command` is typify's rendering of the same `fleet-command.schema.json`
//! definitions for the contract tests.
//! **Signals & state:** none; plain data.
//! **Invariants:** a state's snake_case name is one of the values the `fleet_commands` state check
//! admits and of the receipt's `state` enum; converting a row into a receipt copies every column
//! unchanged, so the receipt's JSON is the row's.

use chrono::{DateTime, Utc};
use fleet_wire_contract::operator_messages::FleetCommandReceipt;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// The state of one command in the ledger.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FleetCommandState {
    /// Waiting for its executor to claim it.
    Queued,
    /// Claimed by an executor that has not started it yet.
    Claimed,
    /// Started by its executor.
    Executing,
    /// Finished with the effect it asked for.
    Succeeded,
    /// Finished without the effect it asked for.
    Failed,
    /// Not completed by any executor before its expiry passed.
    Expired,
    /// Cancelled by an administrator before any executor claimed it.
    Cancelled,
    /// An executor stopped reporting while a non-idempotent effect may have happened; nothing
    /// repeats it and an operator decides.
    Indeterminate,
}

impl FleetCommandState {
    /// The state's snake_case spelling, as stored and on the wire.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Claimed => "claimed",
            Self::Executing => "executing",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Expired => "expired",
            Self::Cancelled => "cancelled",
            Self::Indeterminate => "indeterminate",
        }
    }
}

/// The stored row of one command, selected with the ledger's `RECEIPT_COLUMNS`.
#[derive(Debug, Clone, sqlx::FromRow)]
pub(crate) struct FleetCommandReceiptRow {
    id: Uuid,
    server_id: Uuid,
    executor_kind: String,
    action: String,
    arguments: sqlx::types::Json<Value>,
    requested_by: String,
    requested_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
    state: String,
    attempts: i32,
    claimed_at: Option<DateTime<Utc>>,
    executing_at: Option<DateTime<Utc>>,
    finished_at: Option<DateTime<Utc>>,
    outcome: Option<sqlx::types::Json<Value>>,
    failure_reason: Option<String>,
}

impl From<FleetCommandReceiptRow> for FleetCommandReceipt {
    fn from(row: FleetCommandReceiptRow) -> Self {
        Self {
            id: row.id,
            server_id: row.server_id,
            executor_kind: row.executor_kind,
            action: row.action,
            arguments: row.arguments.0,
            requested_by: row.requested_by,
            requested_at: row.requested_at,
            expires_at: row.expires_at,
            state: row.state,
            attempts: row.attempts,
            claimed_at: row.claimed_at,
            executing_at: row.executing_at,
            finished_at: row.finished_at,
            outcome: row.outcome.map(|outcome| outcome.0),
            failure_reason: row.failure_reason,
        }
    }
}
