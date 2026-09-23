//! What an operator is told when a deployment request or cancellation is refused.
//!
//! **Role:** reads a refused deployment into one of the reasons the backend names, with the details
//! each carries, and words it — listing every unbound seat and unseated slot of an ORBAT mismatch.
//! **Position:** read by the deployments panel after a request or a cancellation fails.
//! **Signals & state:** none; pure over the refusal.
//! **Invariants:** a refused request persists nothing, so every sentence says what to change before
//! asking again. A reason this build does not know falls back to the backend's own sentence.

use super::super::fleet_commands::command_wording::state_label as command_state;
use super::deployment_wording::state_label as deployment_state;
use crate::v2::core::api::client::ApiRefusal;
use serde_json::Value;

/// Why a deployment request or cancellation was refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum DeploymentRefusal {
    /// The artifact is not the one a live mission's latest approval decided.
    ArtifactNotApproved {
        approved_artifact_id: Option<String>,
        mission_status: Option<String>,
    },
    /// The artifact compiled against another modpack than the server requires.
    ModpackMismatch {
        artifact_modpack_id: Option<String>,
        server_modpack_id: Option<String>,
    },
    /// No fleet scenario is registered for the artifact's terrain.
    TerrainNotRunnable { terrain_key: Option<String> },
    /// The event mission does not run this mission on an operation scheduled on this server.
    EventMissionNotOnServer,
    /// The event mission's seats and the artifact's compiled slots do not correspond one to one.
    OrbatArtifactMismatch {
        unbound_seats: Vec<String>,
        unseated_slots: Vec<String>,
        document_disagrees: Option<String>,
    },
    /// Another deployment of this server is in flight.
    DeploymentInProgress { deployment_id: Option<String> },
    /// Only a deployment in flight can be cancelled.
    DeploymentNotInFlight { state: Option<String> },
    /// The deployment's command has been claimed, so it can no longer be cancelled.
    CommandNotCancellable { state: Option<String> },
    /// The server is deactivated.
    ServerInactive,
    /// Any other refusal, carrying the sentence to show.
    Other(String),
}

/// The text list at `key` of a refusal's details.
fn detail_list(refusal: &ApiRefusal, key: &str) -> Vec<String> {
    refusal
        .details
        .as_ref()
        .and_then(|d| d.get(key))
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

impl DeploymentRefusal {
    /// Read a refused request or cancellation; `fallback` is shown when the backend sent no
    /// sentence.
    pub(crate) fn from_refusal(refusal: &ApiRefusal, fallback: &str) -> Self {
        let text = |key: &str| refusal.detail(key).map(str::to_string);
        match refusal.code() {
            Some("ARTIFACT_NOT_APPROVED") => Self::ArtifactNotApproved {
                approved_artifact_id: text("approved_artifact_id"),
                mission_status: text("mission_status"),
            },
            Some("MODPACK_MISMATCH") => Self::ModpackMismatch {
                artifact_modpack_id: text("artifact_modpack_id"),
                server_modpack_id: text("server_modpack_id"),
            },
            Some("TERRAIN_NOT_RUNNABLE") => Self::TerrainNotRunnable {
                terrain_key: text("terrain_key"),
            },
            Some("EVENT_MISSION_NOT_ON_SERVER") => Self::EventMissionNotOnServer,
            Some("ORBAT_ARTIFACT_MISMATCH") => Self::OrbatArtifactMismatch {
                unbound_seats: detail_list(refusal, "unbound_seats"),
                unseated_slots: detail_list(refusal, "unseated_slots"),
                document_disagrees: text("document_disagrees"),
            },
            Some("DEPLOYMENT_IN_PROGRESS") => Self::DeploymentInProgress {
                deployment_id: text("deployment_id"),
            },
            Some("DEPLOYMENT_NOT_IN_FLIGHT") => Self::DeploymentNotInFlight {
                state: text("state"),
            },
            Some("COMMAND_NOT_CANCELLABLE") => Self::CommandNotCancellable {
                state: text("state"),
            },
            Some("SERVER_INACTIVE") => Self::ServerInactive,
            _ => Self::Other(refusal.message_or(fallback)),
        }
    }

    /// The sentence the operator is shown.
    pub(crate) fn sentence(&self) -> String {
        let none = |value: &Option<String>| value.clone().unwrap_or_else(|| "none".to_string());
        match self {
            Self::ArtifactNotApproved {
                approved_artifact_id,
                mission_status,
            } => format!(
                "Only the artifact a live mission's latest approval decided can be deployed. This \
                 mission is {} and its approved artifact is {}. Read the library again and choose \
                 once more.",
                mission_status.as_deref().unwrap_or("not live"),
                none(approved_artifact_id)
            ),
            Self::ModpackMismatch {
                artifact_modpack_id,
                server_modpack_id,
            } => format!(
                "The artifact was compiled against modpack {} but this server requires modpack {}. \
                 Resubmit the mission under the server's modpack, or change the server's modpack.",
                none(artifact_modpack_id),
                none(server_modpack_id)
            ),
            Self::TerrainNotRunnable { terrain_key } => format!(
                "No fleet scenario is registered for terrain {}. Register one under Fleet \
                 scenarios, then deploy again.",
                terrain_key.as_deref().unwrap_or("of this artifact")
            ),
            Self::EventMissionNotOnServer => "The event mission must run this mission on an \
                                              operation scheduled on this server. Choose another \
                                              event mission, or none."
                .to_string(),
            Self::OrbatArtifactMismatch {
                document_disagrees, ..
            } => match document_disagrees {
                Some(why) => format!(
                    "The event mission's seats cannot be bound to the artifact's slots: {why}."
                ),
                None => "The event mission's seats and the artifact's compiled slots do not \
                         correspond one to one. Every unbound seat and unseated slot is listed \
                         below; align the ORBAT with the approved version, or deploy without the \
                         event mission."
                    .to_string(),
            },
            Self::DeploymentInProgress { deployment_id } => format!(
                "Another deployment of this server is in flight{}. Wait for it to finish, or \
                 cancel it while its command is still queued.",
                deployment_id
                    .as_deref()
                    .map(|id| format!(" ({id})"))
                    .unwrap_or_default()
            ),
            Self::DeploymentNotInFlight { state } => format!(
                "Only a deployment in flight can be cancelled; this one is {}.",
                state
                    .as_deref()
                    .map(|s| deployment_state(s).to_lowercase())
                    .unwrap_or_else(|| "finished".to_string())
            ),
            Self::CommandNotCancellable { state } => format!(
                "Its fleet command is {} — an executor has taken it up, so the deployment can no \
                 longer be cancelled.",
                state
                    .as_deref()
                    .map(|s| command_state(s).to_lowercase())
                    .unwrap_or_else(|| "no longer queued".to_string())
            ),
            Self::ServerInactive => {
                "This server is deactivated and accepts no deployment.".to_string()
            }
            Self::Other(sentence) => sentence.clone(),
        }
    }

    /// The seats and slots an ORBAT mismatch names, as `(heading, rows)` groups with any rows.
    pub(crate) fn listed_details(&self) -> Vec<(&'static str, Vec<String>)> {
        match self {
            Self::OrbatArtifactMismatch {
                unbound_seats,
                unseated_slots,
                ..
            } => [
                ("Seats with no compiled slot", unbound_seats.clone()),
                ("Compiled slots no seat stands for", unseated_slots.clone()),
            ]
            .into_iter()
            .filter(|(_, rows)| !rows.is_empty())
            .collect(),
            _ => Vec::new(),
        }
    }
}
