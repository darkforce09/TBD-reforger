//! Fleet commands: an operator's command to one server, and the receipt that follows it from
//! acceptance to its observed outcome.
//!
//! **Role:** the receipt `POST /servers/:id/commands` answers with (202) and `GET …/commands` and
//! `GET …/commands/:commandId` read, the list of a server's receipts, the request body with one
//! constructor per action an operator may request, and the reply a console command reports.
//! **Position:** deserialised straight from the backend's JSON and handed to the server control
//! screen's command console; re-serialised unchanged by the round-trip tests.
//! **Signals & state:** none — these are plain data.
//! **Invariants:** a receipt's action, executor kind and state are carried as the strings the
//! backend sends, so a value added there still lists. The request constructors build exactly the
//! argument shapes the backend accepts — no arguments for process control and the player list, a
//! message for a broadcast, the Arma identity with the runtime session it was issued against for a
//! kick, and one line for a console command — and there is none for the two actions only a mission
//! deployment issues. A receipt's arguments and outcome are action-specific objects, carried whole;
//! only a `console_command` receipt's outcome is also read as a [`ConsoleCommandOutcome`].
//! @contract fleet-command.schema.json#/definitions/FleetCommandReceipt
//! @contract fleet-command.schema.json#/definitions/FleetCommandList
//! @contract fleet-command.schema.json#/definitions/FleetCommandRequest
//! @contract fleet-command.schema.json#/definitions/ConsoleCommandArguments
//! @contract fleet-command.schema.json#/definitions/ConsoleCommandOutcome

use super::identifiers::{ArmaPlayerId, FleetCommandId, RuntimeSessionId, ServerId};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// One fleet command as operators observe it.
/// @contract fleet-command.schema.json#/definitions/FleetCommandReceipt
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FleetCommandReceipt {
    /// The deployment's id.
    pub id: FleetCommandId,
    /// The server the deployment runs on.
    pub server_id: ServerId,
    /// `host_agent` or `mod_runtime`: the program that performs the action.
    pub executor_kind: String,
    /// `start`, `stop`, `restart`, `list_players`, `broadcast`, `kick`, `console_command`, or one
    /// of the two a deployment issues: `load_mission` and `restart_with_mission`.
    pub action: String,
    /// The validated arguments, as the backend stored them.
    pub arguments: Map<String, Value>,
    /// Discord id of the user who requested the deployment.
    pub requested_by: String,
    /// When the deployment was requested (RFC 3339, UTC).
    pub requested_at: String,
    /// When an unclaimed command expires.
    pub expires_at: String,
    /// `queued`, `claimed`, `executing`, `succeeded`, `failed`, `expired`, `cancelled` or
    /// `indeterminate` — the executor stopped reporting after the effect may have started, so the
    /// outcome is unknown and nothing repeats the command.
    pub state: String,
    /// How many times an executor has claimed it.
    pub attempts: i64,
    /// When the host agent claimed the command; absent until claimed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claimed_at: Option<String>,
    /// When the host agent started executing the command; absent until then.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub executing_at: Option<String>,
    /// When the deployment left `requested` (RFC 3339, UTC); `None` while in flight.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<String>,
    /// What the executor observed; for `list_players`, `{players: [{player_id, arma_id, name}]}`,
    /// and for `console_command`, `{response, response_truncated}`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outcome: Option<Map<String, Value>>,
    /// Why the deployment failed or was cancelled; `None` otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure_reason: Option<String>,
}

impl FleetCommandReceipt {
    /// The reply a console command reported; `None` for another action, for a command with no
    /// outcome yet, and for an outcome of another shape.
    pub fn console_outcome(&self) -> Option<ConsoleCommandOutcome> {
        if self.action != "console_command" {
            return None;
        }
        let outcome = self.outcome.as_ref()?;
        serde_json::from_value(Value::Object(outcome.clone())).ok()
    }
}

/// What a succeeded console command reports: the server's reply to the line, cut by the host
/// agent on a character boundary at [`ConsoleCommandOutcome::RESPONSE_MAX_BYTES`], and whether it
/// was cut.
/// @contract fleet-command.schema.json#/definitions/ConsoleCommandOutcome
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsoleCommandOutcome {
    /// The server's reply.
    pub response: String,
    /// Whether the reply was cut at [`Self::RESPONSE_MAX_BYTES`].
    pub response_truncated: bool,
}

impl ConsoleCommandOutcome {
    /// The most bytes of a reply the host agent keeps.
    pub const RESPONSE_MAX_BYTES: usize = 4096;
}

/// `GET /servers/:id/commands`: the server's commands, newest first.
/// @contract fleet-command.schema.json#/definitions/FleetCommandList
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FleetCommandList {
    /// The server's commands, newest first.
    pub items: Vec<FleetCommandReceipt>,
}

/// `POST /servers/:id/commands` body: the action, and its arguments when it takes any.
/// @contract fleet-command.schema.json#/definitions/FleetCommandRequest
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FleetCommandRequest {
    /// What the command asks the server to do.
    pub action: String,
    /// The action's arguments; absent reads as an empty object.
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub arguments: Map<String, Value>,
}

impl FleetCommandRequest {
    /// The most bytes a console line may hold once trimmed.
    pub const CONSOLE_LINE_MAX_BYTES: usize = 256;

    /// An action that takes no arguments: `start`, `stop`, `restart` or `list_players`.
    fn bare(action: &str) -> Self {
        Self {
            action: action.to_string(),
            arguments: Map::new(),
        }
    }

    /// Start the server process.
    pub fn start() -> Self {
        Self::bare("start")
    }

    /// Stop the server process.
    pub fn stop() -> Self {
        Self::bare("stop")
    }

    /// Restart the server process.
    pub fn restart() -> Self {
        Self::bare("restart")
    }

    /// Read the connected players through the host agent.
    pub fn list_players() -> Self {
        Self::bare("list_players")
    }

    /// Show `message` to everyone in the running game.
    pub fn broadcast(message: &str) -> Self {
        let mut arguments = Map::new();
        arguments.insert("message".to_string(), Value::from(message));
        Self {
            action: "broadcast".to_string(),
            arguments,
        }
    }

    /// Remove the player `arma_id` from the runtime session `runtime_session_id`, which must be the
    /// server's open session; `reason` is shown to them when given.
    pub fn kick(
        arma_id: &ArmaPlayerId,
        runtime_session_id: &RuntimeSessionId,
        reason: Option<&str>,
    ) -> Self {
        let mut arguments = Map::new();
        arguments.insert("arma_id".to_string(), Value::from(arma_id.as_str()));
        arguments.insert(
            "runtime_session_id".to_string(),
            Value::from(runtime_session_id.as_str()),
        );
        if let Some(reason) = reason {
            arguments.insert("reason".to_string(), Value::from(reason));
        }
        Self {
            action: "kick".to_string(),
            arguments,
        }
    }

    /// Send `line` — one line, already trimmed — to the server's RCON console through the host
    /// agent, which transmits it once and never repeats it.
    /// @contract fleet-command.schema.json#/definitions/ConsoleCommandArguments
    pub fn console_command(line: &str) -> Self {
        let mut arguments = Map::new();
        arguments.insert("line".to_string(), Value::from(line));
        Self {
            action: "console_command".to_string(),
            arguments,
        }
    }
}
