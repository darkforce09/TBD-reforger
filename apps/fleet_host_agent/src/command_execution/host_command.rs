//! Re-validation of a claimed command.
//!
//! The API validated the action and its arguments when it accepted the command. The agent
//! checks them again, by the same rules, before anything reaches systemctl, the server config
//! or RCON: each action accepts exactly its own argument keys; start, stop, restart and
//! list_players take none; restart_with_mission takes a mission deployment
//! ([`MissionDeployment`]). An action the host agent does not perform (broadcast, kick and
//! load_mission run in the game runtime) is refused, so no text from the API can widen what this
//! host runs.

use serde_json::Value;

use super::command_refusal::{CommandRefusal, arguments_with_only, quoted};
use super::mission_deployment::{MissionDeployment, RESTART_WITH_MISSION};

/// A command this host performs, with validated arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostCommand {
    Start,
    Stop,
    Restart,
    ListPlayers,
    RestartWithMission(MissionDeployment),
}

impl HostCommand {
    /// Validates a claimed command's action and arguments.
    pub fn from_claim(action: &str, arguments: &Value) -> Result<Self, CommandRefusal> {
        let command = match action {
            "start" => Self::Start,
            "stop" => Self::Stop,
            "restart" => Self::Restart,
            "list_players" => Self::ListPlayers,
            RESTART_WITH_MISSION => {
                return MissionDeployment::from_arguments(arguments).map(Self::RestartWithMission);
            }
            "broadcast" | "kick" | "load_mission" => {
                return Err(CommandRefusal::GameRuntimeAction(action.to_owned()));
            }
            other => return Err(CommandRefusal::UnsupportedAction(quoted(other))),
        };
        arguments_with_only(command.action_name(), arguments, &[])?;
        Ok(command)
    }

    /// The action as the ledger names it.
    pub fn action_name(&self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::Stop => "stop",
            Self::Restart => "restart",
            Self::ListPlayers => "list_players",
            Self::RestartWithMission(_) => RESTART_WITH_MISSION,
        }
    }
}

#[cfg(test)]
#[path = "tests/host_command.rs"]
mod tests;
