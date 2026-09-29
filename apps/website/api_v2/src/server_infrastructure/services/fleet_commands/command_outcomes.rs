//! Typed validation of the outcome an executor reports with a command's result.
//!
//! **Role:** the outcome gate of the executor result route: an action whose outcome has a
//! contract accepts exactly that shape within its bounds; every other action's outcome stays the
//! free-form object the executor reported.
//! **Position:** called by
//! [`crate::server_infrastructure::services::fleet_commands::executor_claims::record_result`]
//! once the claim is locked and its action known, before anything is written; a refusal answers
//! 400 and leaves the command as it was.
//! **Signals & state:** none; pure functions.
//! **Invariants:** a succeeded console command carries exactly
//! [`ConsoleCommandOutcome`] with a response of at most
//! [`ConsoleCommandOutcome::RESPONSE_MAX_BYTES`] bytes; a failed one may carry it or nothing.

use serde_json::{Map, Value};

use crate::core::error_handling::api_error::ApiError;
use crate::server_infrastructure::models::fleet_command::{ConsoleCommandOutcome, FleetAction};

/// The outcome of `action` as the ledger stores it, or the refusal of a reported outcome that
/// breaks the action's contract.
pub fn validated_outcome(
    action: FleetAction,
    succeeded: bool,
    outcome: Option<&Map<String, Value>>,
) -> Result<Option<Value>, ApiError> {
    match action {
        FleetAction::ConsoleCommand => console_outcome(succeeded, outcome),
        _ => Ok(outcome.cloned().map(Value::Object)),
    }
}

/// The server's reply to a console line, bounded in UTF-8 bytes. The host agent cuts a longer
/// reply on a character boundary and says so in `response_truncated`, so a longer response is a
/// broken executor, not a long reply.
fn console_outcome(
    succeeded: bool,
    outcome: Option<&Map<String, Value>>,
) -> Result<Option<Value>, ApiError> {
    let Some(outcome) = outcome else {
        return if succeeded {
            Err(ApiError::bad_request(
                "a succeeded console command reports outcome {response, response_truncated}",
            ))
        } else {
            Ok(None)
        };
    };
    let reported: ConsoleCommandOutcome = serde_json::from_value(Value::Object(outcome.clone()))
        .map_err(|_| {
            ApiError::bad_request(
                "a console command's outcome is {response, response_truncated} and nothing else",
            )
        })?;
    if reported.response.len() > ConsoleCommandOutcome::RESPONSE_MAX_BYTES {
        return Err(ApiError::bad_request(format!(
            "a console command's response holds at most {} bytes",
            ConsoleCommandOutcome::RESPONSE_MAX_BYTES
        )));
    }
    serde_json::to_value(reported)
        .map(Some)
        .map_err(|error| ApiError::internal(format!("console outcome: {error}")))
}

#[cfg(test)]
#[path = "tests/command_outcomes.rs"]
mod tests;
