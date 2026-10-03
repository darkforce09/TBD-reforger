//! Typed validation of a command's arguments. Executors receive only arguments that passed
//! this gate, and each action accepts exactly its own keys.
//!
//! **Role:** the argument gate of the operator command route: the keys each action accepts,
//! their bounds, and the form the ledger stores and an executor receives.
//! **Position:** called by
//! [`crate::services::fleet_commands::command_ledger::enqueue_command`]
//! before a command row is written; each executor re-validates what it receives.
//! **Signals & state:** none; pure functions.
//! **Invariants:** an action without arguments stores `{}`; text is trimmed, holds 1 to its
//! byte bound and no control character; a console line is one line that does not start with
//! `@`; the deployment-only actions are refused here.

use fleet_wire_contract::FleetAction;
use fleet_wire_contract::console_command::ConsoleCommandArguments;
use serde_json::{Map, Value};
use uuid::Uuid;

use api_foundation::error_handling::api_error::ApiError;

/// Printable text without control characters, trimmed, of 1 to `max` bytes.
fn bounded_text(arguments: &Map<String, Value>, key: &str, max: usize) -> Result<String, ApiError> {
    let text = arguments
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .ok_or_else(|| ApiError::bad_request(format!("{key} is required")))?;
    if text.is_empty() || text.len() > max || text.chars().any(char::is_control) {
        return Err(ApiError::bad_request(format!(
            "{key} must contain 1 to {max} bytes without control characters"
        )));
    }
    Ok(text.to_owned())
}

fn only_keys(arguments: &Map<String, Value>, allowed: &[&str]) -> Result<(), ApiError> {
    match arguments
        .keys()
        .find(|key| !allowed.contains(&key.as_str()))
    {
        Some(key) => Err(ApiError::bad_request(format!(
            "argument {key} is not accepted by this action"
        ))),
        None => Ok(()),
    }
}

/// One line for the server's RCON console, trimmed of surrounding whitespace: 1 to
/// [`ConsoleCommandArguments::LINE_MAX_BYTES`] bytes, with no control character and no line or
/// paragraph separator anywhere in what was sent, so exactly one line reaches the console. A
/// leading `@` is refused: it starts Reforger's custom RCON commands (`@logout`), which act on
/// the RCON session itself, and that session belongs to the host agent.
fn console_line(arguments: &Map<String, Value>) -> Result<ConsoleCommandArguments, ApiError> {
    let sent = arguments
        .get("line")
        .and_then(Value::as_str)
        .ok_or_else(|| ApiError::bad_request("line is required"))?;
    if sent
        .chars()
        .any(|c| c.is_control() || matches!(c, '\u{2028}' | '\u{2029}'))
    {
        return Err(ApiError::bad_request(
            "line must be one line without control characters",
        ));
    }
    let line = sent.trim();
    if line.is_empty() || line.len() > ConsoleCommandArguments::LINE_MAX_BYTES {
        return Err(ApiError::bad_request(format!(
            "line must contain 1 to {} bytes",
            ConsoleCommandArguments::LINE_MAX_BYTES
        )));
    }
    if line.starts_with('@') {
        return Err(ApiError::bad_request(
            "line must not start with @: RCON session commands belong to the host agent",
        ));
    }
    Ok(ConsoleCommandArguments {
        line: line.to_owned(),
    })
}

/// The validated arguments of `action`, in their stored form. A kick names the Arma identity
/// and the runtime session it is issued against.
pub fn validated_arguments(
    action: FleetAction,
    arguments: &Map<String, Value>,
) -> Result<(Value, Option<Uuid>), ApiError> {
    match action {
        FleetAction::Start
        | FleetAction::Stop
        | FleetAction::Restart
        | FleetAction::ListPlayers => {
            only_keys(arguments, &[])?;
            Ok((Value::Object(Map::new()), None))
        }
        FleetAction::Broadcast => {
            only_keys(arguments, &["message"])?;
            let message = bounded_text(arguments, "message", 256)?;
            Ok((serde_json::json!({ "message": message }), None))
        }
        FleetAction::Kick => {
            only_keys(arguments, &["arma_id", "runtime_session_id", "reason"])?;
            let arma_id = bounded_text(arguments, "arma_id", 128)?;
            let session = arguments
                .get("runtime_session_id")
                .and_then(Value::as_str)
                .and_then(|raw| Uuid::try_parse(raw).ok())
                .ok_or_else(|| ApiError::bad_request("runtime_session_id must be a UUID"))?;
            let mut stored = serde_json::json!({
                "arma_id": arma_id,
                "runtime_session_id": session,
            });
            if arguments.contains_key("reason") {
                stored["reason"] = Value::from(bounded_text(arguments, "reason", 128)?);
            }
            Ok((stored, Some(session)))
        }
        FleetAction::ConsoleCommand => {
            only_keys(arguments, &["line"])?;
            let stored = serde_json::to_value(console_line(arguments)?)
                .map_err(|error| ApiError::internal(format!("console arguments: {error}")))?;
            Ok((stored, None))
        }
        FleetAction::LoadMission | FleetAction::RestartWithMission => {
            Err(ApiError::bad_request(format!(
                "{} is issued by mission deployments: POST /servers/{{id}}/deployments",
                action.as_str()
            )))
        }
    }
}

#[cfg(test)]
#[path = "tests/command_arguments.rs"]
mod tests;
