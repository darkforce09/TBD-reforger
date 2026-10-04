//! Mission deployments: an approved artifact selected for a server, the transition that runs it,
//! and its outcome.
//!
//! **Role:** the deployment `POST /servers/:id/deployments` answers with (202) and the deployment
//! reads return, the page of a server's deployments, and the request body.
//! **Position:** deserialised straight from the backend's JSON and handed to the server control
//! screen's deployments panel; re-serialised unchanged by the round-trip tests.
//! **Signals & state:** none — these are plain data.
//! **Invariants:** states, transitions and the request channel are carried as the strings the
//! backend sends, so a value added there still lists. A deployment is confirmed only by a runtime
//! session of the server, started after the request, that reports the artifact with the exact
//! document SHA-256, so `confirmed_runtime_session_id` is present only on a confirmed deployment.

use super::identifiers::{
    EventMissionId, FleetCommandId, MissionArtifactId, MissionDeploymentId, MissionId,
    RuntimeSessionId, ScenarioId, ServerId,
};
use serde::{Deserialize, Serialize};

/// One deployment as operators observe it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MissionDeployment {
    /// The deployment's id.
    pub id: MissionDeploymentId,
    /// The server the deployment runs on.
    pub server_id: ServerId,
    /// The deployed mission.
    pub mission_id: MissionId,
    /// The deployed mission's title.
    pub mission_title: String,
    /// The artifact the deployment loads.
    pub artifact_id: MissionArtifactId,
    /// The artifact's identity digest, lowercase hex.
    pub artifact_digest: String,
    /// SHA-256 of the artifact's document bytes — what the runtime must report to confirm it.
    pub artifact_sha256: String,
    /// The event mission whose seats were bound to the artifact's slots, when one was named.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_mission_id: Option<EventMissionId>,
    /// The terrain the artifact targets.
    pub terrain_key: String,
    /// The scenario header the fleet runs for the terrain.
    pub scenario_id: ScenarioId,
    /// `scenario_restart` (the runtime already runs the terrain and restarts in-process) or
    /// `host_restart` (the host agent restarts the server on the terrain's scenario).
    pub transition: String,
    /// The fleet command that performs the transition, and its state.
    pub fleet_command_id: FleetCommandId,
    /// The current state of that fleet command.
    pub fleet_command_state: String,
    /// Discord id of the user who requested the deployment.
    pub requested_by: String,
    /// `web` or `game_runtime` (an in-game administrator's selection, relayed by the runtime).
    pub requested_via: String,
    /// When the deployment was requested (RFC 3339, UTC).
    pub requested_at: String,
    /// By when a runtime session must have confirmed the artifact.
    pub deadline_at: String,
    /// `requested`, `confirmed`, `failed` or `cancelled`.
    pub state: String,
    /// The runtime session that confirmed loading the artifact; set only when `confirmed`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confirmed_runtime_session_id: Option<RuntimeSessionId>,
    /// When the deployment left `requested` (RFC 3339, UTC); `None` while in flight.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<String>,
    /// Why the deployment failed or was cancelled; `None` otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure_reason: Option<String>,
    /// How many event seats were bound to compiled slots.
    pub bound_slots: i64,
}

/// `GET /servers/:id/deployments`: the server's deployments, newest first.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MissionDeploymentPage {
    /// The page of deployments, newest first.
    pub items: Vec<MissionDeployment>,
    /// Total number of the server's deployments.
    pub total: i64,
    /// The page size applied.
    pub limit: i64,
    /// The number of deployments skipped before this page.
    pub offset: i64,
}

/// `POST /servers/:id/deployments` body: the live mission, the artifact its latest approval
/// decided, and optionally the event mission whose seats the deployment binds.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DeploymentRequest {
    /// The mission to deploy.
    pub mission_id: MissionId,
    /// The approved artifact of that mission the server loads.
    pub artifact_id: MissionArtifactId,
    /// The event mission whose ORBAT slots bind to the artifact's slots; `None` outside an event.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_mission_id: Option<EventMissionId>,
}
