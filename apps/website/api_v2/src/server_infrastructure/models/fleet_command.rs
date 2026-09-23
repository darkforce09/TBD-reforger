//! Fleet commands: the durable record of one operator command to one server, from acceptance
//! through an executor's claim to its observed outcome.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use super::machine_credential::ExecutorKind;
use crate::core::wire_format::{rfc3339_utc, rfc3339_utc_opt};

/// What a command asks a server to do. The executor maps each action to a fixed program
/// invocation, RCON packet or in-game operation; no action carries free text to a shell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FleetAction {
    Start,
    Stop,
    Restart,
    ListPlayers,
    Broadcast,
    Kick,
    /// A same-terrain transition: the game runtime loads a deployment's artifact and restarts
    /// the scenario in-process.
    LoadMission,
    /// A cross-terrain transition: the host agent restarts the server process on the scenario
    /// of a deployment's terrain.
    RestartWithMission,
}

impl FleetAction {
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
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        [
            Self::Start,
            Self::Stop,
            Self::Restart,
            Self::ListPlayers,
            Self::Broadcast,
            Self::Kick,
            Self::LoadMission,
            Self::RestartWithMission,
        ]
        .into_iter()
        .find(|action| action.as_str() == value)
    }

    /// The program that performs the action: process control and the RCON player list run on
    /// the host agent; a broadcast, a kick and an in-process mission load run inside the game
    /// runtime, which messages and knows its players by identity and restarts its own scenario.
    /// Reforger's RCON has no broadcast command.
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
    /// runs at a time.
    pub fn process_changing(self) -> bool {
        matches!(
            self,
            Self::Start | Self::Stop | Self::Restart | Self::LoadMission | Self::RestartWithMission
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
            Self::ListPlayers | Self::Broadcast | Self::Kick => 30,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FleetCommandState {
    Queued,
    Claimed,
    Executing,
    Succeeded,
    Failed,
    Expired,
    Cancelled,
    /// An executor stopped reporting while a non-idempotent effect may have happened; nothing
    /// repeats it and an operator decides.
    Indeterminate,
}

impl FleetCommandState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Claimed => "claimed",
            Self::Executing => "executing",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Expired => "expired",
            Self::Cancelled => "cancelled",
            Self::Indeterminate => "indeterminate",
        }
    }
}

/// `POST /servers/{id}/commands` body.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FleetCommandRequest {
    pub action: FleetAction,
    #[serde(default)]
    pub arguments: serde_json::Map<String, Value>,
}

/// The receipt of one command: everything an operator can observe about it.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct FleetCommandReceipt {
    pub id: Uuid,
    pub server_id: Uuid,
    pub executor_kind: String,
    pub action: String,
    pub arguments: sqlx::types::Json<Value>,
    pub requested_by: String,
    #[serde(with = "rfc3339_utc")]
    pub requested_at: DateTime<Utc>,
    #[serde(with = "rfc3339_utc")]
    pub expires_at: DateTime<Utc>,
    pub state: String,
    pub attempts: i32,
    #[serde(with = "rfc3339_utc_opt", skip_serializing_if = "Option::is_none")]
    pub claimed_at: Option<DateTime<Utc>>,
    #[serde(with = "rfc3339_utc_opt", skip_serializing_if = "Option::is_none")]
    pub executing_at: Option<DateTime<Utc>>,
    #[serde(with = "rfc3339_utc_opt", skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outcome: Option<sqlx::types::Json<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure_reason: Option<String>,
}

/// `GET /servers/{id}/commands` response.
#[derive(Debug, Serialize)]
pub struct FleetCommandList {
    pub items: Vec<FleetCommandReceipt>,
}

/// A claimed command as its executor receives it, with the fencing token every later report
/// must carry.
#[derive(Debug, Clone, Serialize)]
pub struct ClaimedFleetCommand {
    pub command_id: Uuid,
    pub server_id: Uuid,
    pub action: String,
    pub arguments: Value,
    pub fencing_token: i64,
    #[serde(with = "rfc3339_utc")]
    pub lease_expires_at: DateTime<Utc>,
}

/// `POST /fleet-executor/commands/{commandId}/executing` body.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionStart {
    pub fencing_token: i64,
}

/// `POST /fleet-executor/commands/{commandId}/result` body.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionResult {
    pub fencing_token: i64,
    pub succeeded: bool,
    #[serde(default)]
    pub outcome: Option<serde_json::Map<String, Value>>,
    #[serde(default)]
    pub failure_reason: Option<String>,
}
