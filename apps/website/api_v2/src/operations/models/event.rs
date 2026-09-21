//! The scheduled operation container, the missions attached to it, and the ORBAT seats, squad
//! holds and registrations that hang off those missions.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::core::wire_format::{rfc3339_utc, rfc3339_utc_opt};

/// Event lifecycle states (Postgres ENUM `event_status`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "event_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum EventStatus {
    Scheduled,
    Open,
    Locked,
    Live,
    Completed,
    Cancelled,
}

impl EventStatus {
    /// The Postgres/JSON wire string.
    pub fn as_str(self) -> &'static str {
        match self {
            EventStatus::Scheduled => "scheduled",
            EventStatus::Open => "open",
            EventStatus::Locked => "locked",
            EventStatus::Live => "live",
            EventStatus::Completed => "completed",
            EventStatus::Cancelled => "cancelled",
        }
    }
}

/// Registration states (Postgres ENUM `registration_state`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "registration_state", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum RegistrationState {
    Registered,
    Waitlisted,
    Withdrawn,
    Attended,
    NoShow,
}

impl RegistrationState {
    /// The Postgres/JSON wire string.
    pub fn as_str(self) -> &'static str {
        match self {
            RegistrationState::Registered => "registered",
            RegistrationState::Waitlisted => "waitlisted",
            RegistrationState::Withdrawn => "withdrawn",
            RegistrationState::Attended => "attended",
            RegistrationState::NoShow => "no_show",
        }
    }
}

/// Scheduled operation containing one or more sequential missions (campaign container).
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Event {
    pub id: Uuid,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub name_override: String,
    #[serde(with = "rfc3339_utc")]
    pub start_time: DateTime<Utc>,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub briefing: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub banner_image_url: String,
    pub status: EventStatus,
    pub registration_locked: bool,
    pub max_slots: i64,
    pub created_by: String,
    /// Game server this operation is scheduled on. Nullable uuid — no FK in schema (house
    /// style; see migration 0011). Absent on the wire when unset.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub server_id: Option<Uuid>,
    /// Modpack this operation requires. Per-event, not the global `/modpacks/current`.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub modpack_id: Option<Uuid>,
    #[serde(with = "rfc3339_utc")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "rfc3339_utc")]
    pub updated_at: DateTime<Utc>,
}

/// Links an Event to a Mission with its own start time. ORBAT slots + registrations
/// hang off this row.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct EventMission {
    pub id: Uuid,
    pub event_id: Uuid,
    pub mission_id: Uuid,
    #[serde(with = "rfc3339_utc")]
    pub start_time: DateTime<Utc>,
    #[serde(with = "rfc3339_utc")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "rfc3339_utc")]
    pub updated_at: DateTime<Utc>,
}

/// One fillable position in the Order of Battle for a mission within an event.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct OrbatSlot {
    pub id: Uuid,
    pub event_mission_id: Uuid,
    pub faction: String,
    pub squad: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub callsign: String,
    pub role: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub loadout: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub tag: String,
    pub slot_index: i64,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub assigned_to: Option<String>,
    #[serde(
        with = "rfc3339_utc_opt",
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub assigned_at: Option<DateTime<Utc>>,
}

/// One-click hold a leader places on a whole squad within a mission.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct OrbatReservation {
    pub id: Uuid,
    pub event_mission_id: Uuid,
    pub squad: String,
    pub reserved_by: String,
    #[serde(with = "rfc3339_utc")]
    pub reserved_at: DateTime<Utc>,
}

/// A user signed up for a specific mission within an event.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct EventRegistration {
    pub id: Uuid,
    pub event_mission_id: Uuid,
    pub discord_id: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub slot_id: Option<Uuid>,
    pub state: RegistrationState,
    #[serde(with = "rfc3339_utc")]
    pub registered_at: DateTime<Utc>,
}
