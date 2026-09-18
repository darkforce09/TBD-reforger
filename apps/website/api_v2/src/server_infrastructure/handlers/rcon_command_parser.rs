//! The RCON request boundary: the wire body, the validated command it parses into, and the
//! mapping from that command onto the host agent's fixed verb set.
//!
//! Modelling the request as a value is what guarantees there is no "accepted and ignored" state:
//! every field the operator supplied is either represented in [`RconCommand`] or rejected here.
//! Nothing downstream ever sees raw request bytes as an action.

use serde::Deserialize;

use crate::server_infrastructure::services::game_agent::AgentAction;

/// The RCON body.
///
/// `action` is required, so it carries no `#[serde(default)]`. `map` and `command` keep theirs
/// **deliberately**: they are genuinely optional per-action (only `change_map` reads `map`, only
/// `custom` reads `command`), so for those two the default really does mean "not supplied" rather
/// than an affirmative empty answer. For `command` under `custom`, "not supplied" is a 400 — see
/// [`parse_rcon_command`].
#[derive(Debug, Deserialize)]
pub struct RconInput {
    pub(super) action: String,
    #[serde(default)]
    pub(super) map: String,
    #[serde(default)]
    pub(super) command: String,
}

/// One validated RCON request — the thing a transport has to carry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum RconCommand {
    Restart,
    /// Trimmed map name. Empty = supplied as whitespace-only, kept as a distinct case so a
    /// whitespace-only map degrades to the bare `change_map` audit line rather than inventing a
    /// destination.
    ChangeMap(String),
    Kick,
    /// Trimmed, guaranteed non-empty — see [`parse_rcon_command`].
    Custom(String),
}

impl RconCommand {
    /// The wire `action` echoed back to the client. One of the four literals, never request
    /// bytes (the exact-set match in [`parse_rcon_command`] is what guarantees that).
    pub(super) fn action(&self) -> &'static str {
        match self {
            RconCommand::Restart => "restart",
            RconCommand::ChangeMap(_) => "change_map",
            RconCommand::Kick => "kick",
            RconCommand::Custom(_) => "custom",
        }
    }

    /// What goes in the audit row — **including the operand**.
    ///
    /// The audit log is the only place an RCON request lands at all, so a row naming just the
    /// action could not tell a restart from a shutdown, and the request would be recorded
    /// nowhere in the system.
    pub(super) fn audit_detail(&self) -> String {
        match self {
            RconCommand::Restart => "restart".to_string(),
            // `map` is the one field in this handler whose raw request bytes reach a persisted
            // string. An untrimmed guard would let `{"action":"change_map","map":"   "}` write an
            // audit row reading `RCON 'change_map ->    '`: a recorded map change naming no map.
            // Trim once, test and emit the same value.
            RconCommand::ChangeMap(map) if map.is_empty() => "change_map".to_string(),
            RconCommand::ChangeMap(map) => format!("change_map -> {map}"),
            RconCommand::Kick => "kick".to_string(),
            RconCommand::Custom(cmd) => format!("custom -> {cmd}"),
        }
    }
}

/// Validate a request body into an [`RconCommand`], or return the 400 message.
///
/// `action` is deliberately **untrimmed**: the exact-set match is what normalises it, so a
/// whitespace-padded action fails closed with "unknown action" and anything downstream is
/// guaranteed to be one of the four literals.
///
/// `custom` requires a non-blank `command`. An operator typing a command into the console and
/// getting a success back over a request the API never even looked at is the defect this rejects.
pub(super) fn parse_rcon_command(
    action: &str,
    map: &str,
    command: &str,
) -> Result<RconCommand, &'static str> {
    if action.is_empty() {
        return Err("action required");
    }
    match action {
        "restart" => Ok(RconCommand::Restart),
        "change_map" => Ok(RconCommand::ChangeMap(map.trim().to_string())),
        "kick" => Ok(RconCommand::Kick),
        "custom" => {
            let cmd = command.trim();
            if cmd.is_empty() {
                return Err("command required for custom action");
            }
            Ok(RconCommand::Custom(cmd.to_string()))
        }
        _ => Err("unknown action"),
    }
}

/// The agent verb that carries this command, or `None` when nothing does.
///
/// Only `restart` maps. See [`super::rcon_console::RCON_ACTION_UNSUPPORTED`] for why the other
/// three do not, and why that is a different answer from "no transport".
pub(super) fn agent_action_for(cmd: &RconCommand) -> Option<AgentAction> {
    match cmd {
        RconCommand::Restart => Some(AgentAction::Restart),
        RconCommand::ChangeMap(_) | RconCommand::Kick | RconCommand::Custom(_) => None,
    }
}

#[cfg(test)]
#[path = "tests/rcon_command_parser.rs"]
mod tests;
