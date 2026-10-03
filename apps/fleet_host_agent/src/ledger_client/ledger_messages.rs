//! The agent's side of the executor routes' messages: the result report built from a verdict,
//! and the API's error envelope. The wire shapes themselves (`ClaimedFleetCommand`,
//! `ExecutionStart`, `ExecutionResult` of `contracts/definitions/fleet-command.schema.json`) are
//! the `fleet_wire_contract` ones the API writes and reads.

use fleet_wire_contract::executor_messages::ExecutionResult;
use serde::Deserialize;
use serde_json::Value;

use crate::action_verdict::ActionVerdict;

/// The `result` report of the claim holding `fencing_token`: what `verdict` observed.
pub(super) fn execution_result_from_verdict(
    fencing_token: i64,
    verdict: ActionVerdict,
) -> ExecutionResult {
    let (succeeded, outcome, failure_reason) = verdict.into_parts();
    ExecutionResult {
        fencing_token,
        succeeded,
        outcome,
        failure_reason,
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
