//! What a fleet command asks a server to do, and the rules each action carries.
//!
//! **Role:** the nine actions in their wire spelling and their rules: the executor that runs
//! each, whether repeating it is harmless, whether it changes the server process, whether only a
//! mission deployment issues it, and how long its executor may take.
//! **Position:** read by the API's command ledger, fleet handlers and mission deployments; the
//! receipts and claims of [`crate::operator_messages`] and [`crate::executor_messages`] carry the
//! spelling of [`FleetAction::as_str`].
//! **Signals & state:** none; plain data and pure rule functions.
//! **Invariants:** an action's snake_case spelling is one of the schema's `FleetAction` values,
//! [`FleetAction::ALL`] lists them in the schema's order, and [`FleetAction::parse`] inverts
//! [`FleetAction::as_str`].
//!
//! @contract fleet-command.schema.json#/definitions/FleetAction

use serde::{Deserialize, Serialize};

use crate::executor_kind::ExecutorKind;

/// What a command asks a server to do. The executor maps each action to a fixed program
/// invocation, RCON packet or in-game operation, and no argument ever reaches a shell; the one
/// free-text argument, a console command's line, goes only to the server's RCON console.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FleetAction {
    /// Start the dedicated server process.
    Start,
    /// Stop the dedicated server process.
    Stop,
    /// Restart the dedicated server process.
    Restart,
    /// Read the connected players over RCON.
    ListPlayers,
    /// Show a message to every player, sent by the game runtime.
    Broadcast,
    /// Remove one player from the server, done by the game runtime.
    Kick,
    /// A same-terrain transition: the game runtime loads a deployment's artifact and restarts
    /// the mission in-process.
    LoadMission,
    /// A cross-terrain transition: the host agent restarts the server process on the mission
    /// header of a deployment's terrain.
    RestartWithMission,
    /// One administrator line for the dedicated server's RCON console, which the host agent
    /// transmits once and whose reply it reports as a
    /// [`crate::console_command::ConsoleCommandOutcome`].
    ConsoleCommand,
}

impl FleetAction {
    /// Every action, in the order of the schema's `FleetAction` enum.
    pub const ALL: [FleetAction; 9] = [
        Self::Start,
        Self::Stop,
        Self::Restart,
        Self::ListPlayers,
        Self::Broadcast,
        Self::Kick,
        Self::LoadMission,
        Self::RestartWithMission,
        Self::ConsoleCommand,
    ];

    /// The wire spelling, such as `list_players`.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::Stop => "stop",
            Self::Restart => "restart",
            Self::ListPlayers => "list_players",
            Self::Broadcast => "broadcast",
            Self::Kick => "kick",
            Self::LoadMission => "load_mission",
            Self::RestartWithMission => "restart_with_mission",
            Self::ConsoleCommand => "console_command",
        }
    }

    /// The action spelled `value`, or `None` for any other text.
    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|action| action.as_str() == value)
    }

    /// The program that performs the action: process control, the RCON player list and the
    /// RCON console run on the host agent; a broadcast, a kick and an in-process mission load
    /// run inside the game runtime, which messages and knows its players by identity and
    /// restarts its own mission. Reforger's RCON has no broadcast command.
    pub fn executor(self) -> ExecutorKind {
        match self {
            Self::Broadcast | Self::Kick | Self::LoadMission => ExecutorKind::ModRuntime,
            _ => ExecutorKind::HostAgent,
        }
    }

    /// Repeating the action after an unknown outcome cannot do harm.
    pub fn idempotent(self) -> bool {
        matches!(self, Self::Start | Self::Stop | Self::ListPlayers)
    }

    /// Changes the server process or the mission it runs; at most one such command per server
    /// runs at a time. A console line counts: the server's console can stop or restart the
    /// server and its mission.
    pub fn process_changing(self) -> bool {
        matches!(
            self,
            Self::Start
                | Self::Stop
                | Self::Restart
                | Self::LoadMission
                | Self::RestartWithMission
                | Self::ConsoleCommand
        )
    }

    /// Issued only by a mission deployment, which validates the selection and builds the
    /// arguments; the operator command route refuses it.
    pub fn deployment_only(self) -> bool {
        matches!(self, Self::LoadMission | Self::RestartWithMission)
    }

    /// Seconds an executor may spend between reporting `executing` and reporting the outcome.
    pub fn execution_window_seconds(self) -> i64 {
        match self {
            Self::Start | Self::Restart | Self::RestartWithMission => 180,
            Self::Stop => 120,
            Self::LoadMission => 60,
            Self::ListPlayers | Self::Broadcast | Self::Kick | Self::ConsoleCommand => 30,
        }
    }
}

#[cfg(test)]
#[path = "tests/fleet_action.rs"]
mod tests;
