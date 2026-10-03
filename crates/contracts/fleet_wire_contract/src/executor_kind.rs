//! The program on a game server that a machine credential authenticates and a fleet command
//! runs on.
//!
//! **Role:** names the two executors, host agent and game runtime, in their wire spelling.
//! **Position:** read by [`crate::fleet_action::FleetAction::executor`], by the API's machine
//! credentials, executor routes and game-runtime handlers, and by the staging fixtures tool's
//! credential file layout.
//! **Signals & state:** none; plain data.
//! **Invariants:** the snake_case spelling is one of the schemas' `ExecutorKind` values, and
//! [`ExecutorKind::parse`] inverts [`ExecutorKind::as_str`].
//!
//! @contract machine-credential.schema.json#/definitions/ExecutorKind
//! @contract fleet-command.schema.json#/definitions/ExecutorKind

use serde::{Deserialize, Serialize};

/// The program a credential authenticates on its server.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutorKind {
    /// The host process supervisor: process control and RCON delivery.
    HostAgent,
    /// The game runtime itself: runtime sessions, heartbeats, roster reads and deployments.
    ModRuntime,
}

impl ExecutorKind {
    /// The wire spelling: `host_agent` or `mod_runtime`.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::HostAgent => "host_agent",
            Self::ModRuntime => "mod_runtime",
        }
    }

    /// The executor kind spelled `value`, or `None` for any other text.
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "host_agent" => Some(Self::HostAgent),
            "mod_runtime" => Some(Self::ModRuntime),
            _ => None,
        }
    }
}

#[cfg(test)]
#[path = "tests/executor_kind.rs"]
mod tests;
