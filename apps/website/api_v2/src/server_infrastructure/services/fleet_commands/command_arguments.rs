//! Typed validation of a command's arguments. Executors receive only arguments that passed
//! this gate, and each action accepts exactly its own keys.

use serde_json::{Map, Value};
use uuid::Uuid;

use crate::core::error_handling::api_error::ApiError;
use crate::server_infrastructure::models::fleet_command::FleetAction;

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
