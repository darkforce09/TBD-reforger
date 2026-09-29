//! Re-validation of a claimed command.
//!
//! **Role:** Turns a claimed command's action and arguments into a [`HostCommand`] by the rules
//! the API applied when it accepted the command, before anything reaches systemctl, the server
//! config or RCON: each action accepts exactly its own argument keys; start, stop, restart and
//! list_players take none; restart_with_mission takes a mission deployment
//! ([`MissionDeployment`]); console_command takes one console line ([`ConsoleLine`]).
//!
//! **Position:** `crate::ledger_client::command_loop` calls [`HostCommand::from_claim`] on every
//! claim, then hands the command to a `FleetActionExecutor` or reports the [`CommandRefusal`] as
//! the command's failure without acting.
//!
//! **Signals & state:** none; pure functions.
//!
//! **Invariants:** an action the host agent does not perform (broadcast, kick and load_mission
//! run in the game runtime) is refused, so no text from the API can widen what this host runs.

use serde_json::Value;

use super::command_refusal::{CommandRefusal, arguments_with_only, quoted};
use super::console_line::{CONSOLE_COMMAND, ConsoleLine};
use super::mission_deployment::{MissionDeployment, RESTART_WITH_MISSION};

/// A command this host performs, with validated arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostCommand {
    Start,
    Stop,
    Restart,
    ListPlayers,
    RestartWithMission(MissionDeployment),
    /// One operator line for the game server's console, transmitted once over RCON.
    ConsoleCommand(ConsoleLine),
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
            CONSOLE_COMMAND => {
                return ConsoleLine::from_arguments(arguments).map(Self::ConsoleCommand);
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
            Self::ConsoleCommand(_) => CONSOLE_COMMAND,
        }
    }
}

#[cfg(test)]
#[path = "tests/host_command.rs"]
mod tests;
