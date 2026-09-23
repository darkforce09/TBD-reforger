//! Mission deployments: an approved artifact selected for a server, the transition that runs it
//! and its outcome, and what a game runtime reads to know what to run.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::core::wire_format::{rfc3339_utc, rfc3339_utc_opt};

/// `POST /servers/{id}/deployments` body.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeploymentRequest {
    pub mission_id: Uuid,
    pub artifact_id: Uuid,
    #[serde(default)]
    pub event_mission_id: Option<Uuid>,
}

/// `POST /game-runtime/deployments` body: an in-game administrator's selection, relayed by the
/// server's runtime with the Arma identity that made it.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RelayedDeploymentRequest {
    pub mission_id: Uuid,
    pub artifact_id: Uuid,
    #[serde(default)]
    pub event_mission_id: Option<Uuid>,
    pub requested_by_arma_id: String,
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
    pub id: Uuid,
    pub server_id: Uuid,
    pub mission_id: Uuid,
    pub mission_title: String,
    pub artifact_id: Uuid,
    pub artifact_digest: String,
    pub artifact_sha256: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_mission_id: Option<Uuid>,
    pub terrain_key: String,
    pub scenario_id: String,
    pub transition: String,
    pub fleet_command_id: Uuid,
    pub fleet_command_state: String,
    pub requested_by: String,
    pub requested_via: String,
    #[serde(with = "rfc3339_utc")]
    pub requested_at: DateTime<Utc>,
    #[serde(with = "rfc3339_utc")]
    pub deadline_at: DateTime<Utc>,
    pub state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confirmed_runtime_session_id: Option<Uuid>,
    #[serde(with = "rfc3339_utc_opt", skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure_reason: Option<String>,
    pub bound_slots: i64,
}

/// `GET /servers/{id}/deployments` response, newest first.
#[derive(Debug, Serialize)]
pub struct MissionDeploymentPage {
    pub items: Vec<MissionDeployment>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

/// `GET /game-runtime/deployment`: what the runtime runs — the deployment in flight, else the
/// latest confirmed one.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct RuntimeDeployment {
    pub deployment_id: Uuid,
    pub state: String,
    pub mission_id: Uuid,
    pub artifact_id: Uuid,
    pub artifact_sha256: String,
    pub artifact_bytes: i32,
    pub terrain_key: String,
    pub scenario_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_mission_id: Option<Uuid>,
}

/// One mission an in-game administrator may deploy to this server.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct DeployableMission {
    pub mission_id: Uuid,
    pub title: String,
    pub terrain_key: String,
    pub artifact_id: Uuid,
    pub artifact_sha256: String,
}

/// `GET /game-runtime/missions` response.
#[derive(Debug, Serialize)]
pub struct DeployableMissionList {
    pub missions: Vec<DeployableMission>,
}
