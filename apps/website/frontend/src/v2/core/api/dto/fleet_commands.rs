//! Fleet commands: an operator's command to one server, and the receipt that follows it from
//! acceptance to its observed outcome.
//!
//! **Role:** the receipt `POST /servers/:id/commands` answers with (202) and `GET …/commands` and
//! `GET …/commands/:commandId` read, the list of a server's receipts, and the request body with one
//! constructor per action an operator may request.
//! **Position:** deserialised straight from the backend's JSON and handed to the server control
//! screen's command console; re-serialised unchanged by the round-trip tests.
//! **Signals & state:** none — these are plain data.
//! **Invariants:** a receipt's action, executor kind and state are carried as the strings the
//! backend sends, so a value added there still lists. The request constructors build exactly the
//! argument shapes the backend accepts — no arguments for process control and the player list, a
//! message for a broadcast, and the Arma identity with the runtime session it was issued against
//! for a kick — and there is none for the two actions only a mission deployment issues. A receipt's
//! arguments and outcome are action-specific objects, carried whole.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// One fleet command as operators observe it.
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FleetCommandReceipt {
    pub id: String,
    pub server_id: String,
    /// `host_agent` or `mod_runtime`: the program that performs the action.
    pub executor_kind: String,
    /// `start`, `stop`, `restart`, `list_players`, `broadcast`, `kick`, or one of the two a
    /// deployment issues: `load_mission` and `restart_with_mission`.
    pub action: String,
    /// The validated arguments, as the backend stored them.
    pub arguments: Map<String, Value>,
    pub requested_by: String,
    pub requested_at: String,
    /// When an unclaimed command expires.
    pub expires_at: String,
    /// `queued`, `claimed`, `executing`, `succeeded`, `failed`, `expired`, `cancelled` or
    /// `indeterminate` — the executor stopped reporting after the effect may have started, so the
    /// outcome is unknown and nothing repeats the command.
    pub state: String,
    /// How many times an executor has claimed it.
    pub attempts: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claimed_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub executing_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<String>,
    /// What the executor observed; for `list_players`, `{players: [{player_id, arma_id, name}]}`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outcome: Option<Map<String, Value>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure_reason: Option<String>,
}

/// `GET /servers/:id/commands`: the server's commands, newest first.
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FleetCommandList {
    pub items: Vec<FleetCommandReceipt>,
}

/// `POST /servers/:id/commands` body: the action, and its arguments when it takes any.
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FleetCommandRequest {
    pub action: String,
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub arguments: Map<String, Value>,
}

#[allow(dead_code)]
impl FleetCommandRequest {
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
    pub fn kick(arma_id: &str, runtime_session_id: &str, reason: Option<&str>) -> Self {
        let mut arguments = Map::new();
        arguments.insert("arma_id".to_string(), Value::from(arma_id));
        arguments.insert(
            "runtime_session_id".to_string(),
            Value::from(runtime_session_id),
        );
        if let Some(reason) = reason {
            arguments.insert("reason".to_string(), Value::from(reason));
        }
        Self {
            action: "kick".to_string(),
            arguments,
        }
    }
}
