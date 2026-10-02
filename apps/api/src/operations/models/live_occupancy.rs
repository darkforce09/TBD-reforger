//! Game-runtime deployment authorization and live slot occupancy on the wire.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::core::wire_format::rfc3339_utc;

/// `POST /game-runtime/sessions/{sessionId}/deployments` body: one player life asking to spawn
/// into one ORBAT slot. `player_life_id` is chosen by the runtime and identifies retries.
#[derive(Debug, Clone, Deserialize)]
pub struct DeploymentRequest {
    pub event_mission_id: Uuid,
    pub orbat_slot_id: Uuid,
    pub arma_id: String,
    pub player_life_id: String,
}

/// Why a deployment was refused. A refusal holds no state; asking again decides again.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DeploymentDenial {
    /// The Arma identity belongs to no platform account.
    IdentityNotLinked,
    /// The account is banned or deleted.
    AccountUnavailable,
    /// Another participant holds the slot's reservation.
    SlotReserved,
    /// The player's own reservation in this mission is for another slot.
    ReservedAnotherSlot,
    /// The slot's effective access policy does not admit the player.
    AccessPolicy,
    /// Admission depends on a Discord membership observation that has not been made yet.
    MembershipVerificationRequired,
    /// Another life occupies the slot.
    LiveSlotOccupied,
    /// The player already has an open life in this runtime session.
    PlayerAlreadyDeployed,
    /// No deployment of the artifact this runtime session reported loading binds the slot.
    SlotNotInLoadedMission,
}

impl DeploymentDenial {
    pub fn message(self) -> &'static str {
        match self {
            Self::IdentityNotLinked => "this Arma identity is not linked to a platform account",
            Self::AccountUnavailable => "the account is unavailable",
            Self::SlotReserved => "the slot is reserved for another participant",
            Self::ReservedAnotherSlot => "the player's reservation is for another slot",
            Self::AccessPolicy => "the slot's access policy does not admit the player",
            Self::MembershipVerificationRequired => {
                "Discord membership must be verified before this slot admits the player"
            }
            Self::LiveSlotOccupied => "another player occupies the slot",
            Self::PlayerAlreadyDeployed => "the player already occupies a slot in this session",
            Self::SlotNotInLoadedMission => {
                "the slot is not a seat of the mission this server runtime loaded"
            }
        }
    }
}

/// What entitled a life to its slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DeploymentAuthority {
    /// The player's own active reservation of this slot.
    Reservation,
    /// An unreserved slot whose effective policy admits the player.
    OpenSlotPolicy,
}

impl DeploymentAuthority {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Reservation => "reservation",
            Self::OpenSlotPolicy => "open_slot_policy",
        }
    }
}

/// The recorded life an allowed deployment opened.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, sqlx::FromRow)]
pub struct LiveOccupancy {
    pub occupancy_id: Uuid,
    pub runtime_session_id: Uuid,
    pub event_mission_id: Uuid,
    pub orbat_slot_id: Uuid,
    pub arma_id: String,
    pub player_life_id: String,
    pub authorized_by: String,
    #[serde(with = "rfc3339_utc")]
    pub started_at: DateTime<Utc>,
}

/// The deployment decision: an open life, or a refusal with its reason.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "decision", rename_all = "snake_case")]
pub enum DeploymentDecision {
    Allowed(LiveOccupancy),
    Denied {
        reason: DeploymentDenial,
        message: String,
        /// The open life that blocks this one, when the player already occupies a slot.
        #[serde(skip_serializing_if = "Option::is_none")]
        occupancy_id: Option<Uuid>,
    },
}

impl DeploymentDecision {
    pub fn denied(reason: DeploymentDenial) -> Self {
        Self::Denied {
            reason,
            message: reason.message().to_owned(),
            occupancy_id: None,
        }
    }
}

/// `POST /game-runtime/sessions/{sessionId}/deployments/{occupancyId}/end` response.
#[derive(Debug, Serialize)]
pub struct EndedLife {
    pub occupancy_id: Uuid,
    pub ended: bool,
    pub end_reason: String,
}
