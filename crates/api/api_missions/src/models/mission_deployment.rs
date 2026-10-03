//! Mission deployments: an approved artifact selected for a server, the transition that runs it
//! and its outcome, and what a game runtime reads to know what to run.

use api_identifiers::{
    ArmaPlayerId, EventId, EventMissionId, FleetCommandId, MissionArtifactId, MissionDeploymentId,
    MissionId, RuntimeSessionId, ScenarioId, ServerId,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use fleet_wire_contract::rfc3339_timestamps::{rfc3339_utc, rfc3339_utc_opt};

/// `POST /servers/{id}/deployments` body.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeploymentRequest {
    /// The mission to deploy.
    pub mission_id: MissionId,
    /// The approved artifact of that mission the server loads.
    pub artifact_id: MissionArtifactId,
    /// The event mission whose ORBAT slots bind to the artifact's slots; `None` outside an event.
    #[serde(default)]
    pub event_mission_id: Option<EventMissionId>,
}

/// `POST /game-runtime/deployments` body: an in-game administrator's selection, relayed by the
/// server's runtime with the Arma identity that made it.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RelayedDeploymentRequest {
    /// The mission to deploy.
    pub mission_id: MissionId,
    /// The approved artifact of that mission the server loads.
    pub artifact_id: MissionArtifactId,
    /// The event mission whose ORBAT slots bind to the artifact's slots; `None` outside an event.
    #[serde(default)]
    pub event_mission_id: Option<EventMissionId>,
    /// The Arma identity of the in-game administrator who made the selection.
    pub requested_by_arma_id: ArmaPlayerId,
}

/// How a deployment reaches the running server.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeploymentTransition {
    /// The runtime already runs this terrain: it loads the artifact and restarts the scenario.
    ScenarioRestart,
    /// The host agent restarts the server process on the terrain's scenario.
    HostRestart,
}

impl DeploymentTransition {
    /// The wire and database spelling: `scenario_restart` or `host_restart`.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ScenarioRestart => "scenario_restart",
            Self::HostRestart => "host_restart",
        }
    }

    /// Seconds from the request until a runtime session must have confirmed the artifact.
    pub fn deadline_seconds(self) -> i64 {
        match self {
            Self::ScenarioRestart => 600,
            Self::HostRestart => 1200,
        }
    }
}

/// A deployment as operators observe it.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
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
    /// The artifact's identity digest (lowercase hex SHA-256 over its compile inputs).
    pub artifact_digest: String,
    /// Lowercase hex SHA-256 of the artifact's compiled document, verified by the runtime.
    pub artifact_sha256: String,
    /// The event mission whose ORBAT slots the deployment binds; `None` outside an event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_mission_id: Option<EventMissionId>,
    /// The terrain the artifact targets.
    pub terrain_key: String,
    /// The scenario the fleet runs for that terrain.
    pub scenario_id: ScenarioId,
    /// How the deployment reaches the server, as [`DeploymentTransition::as_str`] spells it.
    pub transition: String,
    /// The fleet command that performs the transition.
    pub fleet_command_id: FleetCommandId,
    /// The current state of that fleet command.
    pub fleet_command_state: String,
    /// Discord id of the user who requested the deployment.
    pub requested_by: String,
    /// Where the request came from: `web` or `game_runtime`.
    pub requested_via: String,
    /// When the deployment was requested (RFC 3339, UTC).
    #[serde(with = "rfc3339_utc")]
    pub requested_at: DateTime<Utc>,
    /// When a runtime session must have confirmed the artifact (RFC 3339, UTC).
    #[serde(with = "rfc3339_utc")]
    pub deadline_at: DateTime<Utc>,
    /// Deployment state: `requested`, `confirmed`, `failed` or `cancelled`.
    pub state: String,
    /// The runtime session that confirmed loading the artifact; set only when `confirmed`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confirmed_runtime_session_id: Option<RuntimeSessionId>,
    /// When the deployment left `requested` (RFC 3339, UTC); `None` while in flight.
    #[serde(with = "rfc3339_utc_opt", skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<DateTime<Utc>>,
    /// Why the deployment failed or was cancelled; `None` otherwise.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure_reason: Option<String>,
    /// How many event ORBAT slots the deployment binds to the artifact's compiled slots.
    pub bound_slots: i64,
}

/// `GET /servers/{id}/deployments` response, newest first.
#[derive(Debug, Serialize)]
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

/// `GET /game-runtime/deployment`: what the runtime runs — the deployment in flight, else the
/// latest confirmed one.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct RuntimeDeployment {
    /// The deployment's id.
    pub deployment_id: MissionDeploymentId,
    /// Deployment state: `requested` (in flight) or `confirmed`.
    pub state: String,
    /// The deployed mission.
    pub mission_id: MissionId,
    /// The artifact the runtime loads.
    pub artifact_id: MissionArtifactId,
    /// Lowercase hex SHA-256 of the artifact's compiled document, verified after download.
    pub artifact_sha256: String,
    /// Size of the artifact's compiled document in bytes.
    pub artifact_bytes: i32,
    /// The terrain the artifact targets.
    pub terrain_key: String,
    /// The scenario the fleet runs for that terrain.
    pub scenario_id: ScenarioId,
    /// The event the deployment serves; `None` outside an event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_id: Option<EventId>,
    /// The event mission whose ORBAT slots the deployment binds; `None` outside an event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_mission_id: Option<EventMissionId>,
}

/// One mission an in-game administrator may deploy to this server.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct DeployableMission {
    /// The mission's id.
    pub mission_id: MissionId,
    /// The mission's title.
    pub title: String,
    /// The terrain the mission's approved artifact targets.
    pub terrain_key: String,
    /// The mission's approved artifact.
    pub artifact_id: MissionArtifactId,
    /// Lowercase hex SHA-256 of the approved artifact's compiled document.
    pub artifact_sha256: String,
}

/// `GET /game-runtime/missions` response.
#[derive(Debug, Serialize)]
pub struct DeployableMissionList {
    /// The missions the server may deploy.
    pub missions: Vec<DeployableMission>,
}
