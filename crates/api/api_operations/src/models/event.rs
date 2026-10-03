//! The scheduled operation container, the missions attached to it, and the ORBAT seats, squad
//! holds and registrations that hang off those missions.
//!
//! @contract event-schedule.schema.json#/definitions/Event
//! @contract event-schedule.schema.json#/definitions/EventStatus
//! @contract event-schedule.schema.json#/definitions/EventMission
//! @contract reservation-actions.schema.json#/definitions/SquadReservation

use api_identifiers::{
    DiscordUserId, EventId, EventMissionId, EventRegistrationId, MissionId, ModpackId,
    OrbatReservationId, OrbatSlotId, ServerId,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use fleet_wire_contract::rfc3339_timestamps::{rfc3339_utc, rfc3339_utc_opt};

/// Event lifecycle states (Postgres ENUM `event_status`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "event_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum EventStatus {
    /// Announced on the schedule; wire value `scheduled`.
    Scheduled,
    /// Accepting signups; wire value `open`.
    Open,
    /// Signups frozen ahead of play; wire value `locked`.
    Locked,
    /// The operation is being played now; wire value `live`.
    Live,
    /// The operation has been played; wire value `completed`.
    Completed,
    /// The operation was called off; wire value `cancelled`.
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
    /// Reservation state: holds a place in the mission; wire value `registered`.
    Registered,
    /// Reservation state: queued on the waiting list; wire value `waitlisted`.
    Waitlisted,
    /// Reservation state: the member left the mission; wire value `withdrawn`.
    Withdrawn,
    /// Attendance state: the member played the mission; wire value `attended`.
    Attended,
    /// Attendance state: the member held a place but did not play; wire value `no_show`.
    NoShow,
    /// Reservation state of a historical row whose reservation is unknown; `legacy_unknown`.
    LegacyUnknown,
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
            RegistrationState::LegacyUnknown => "legacy_unknown",
        }
    }
}

/// Scheduled operation containing one or more sequential missions (campaign container).
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Event {
    /// Event id (uuid).
    pub id: EventId,
    /// Display name replacing the mission-derived title; empty (absent on the wire) when unset.
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub name_override: String,
    /// When the operation starts (RFC 3339 UTC on the wire).
    #[serde(with = "rfc3339_utc")]
    pub start_time: DateTime<Utc>,
    /// Briefing text shown on the event; empty (absent on the wire) when unset.
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub briefing: String,
    /// Banner image URL; empty (absent on the wire) when unset.
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub banner_image_url: String,
    /// Lifecycle state of the event.
    pub status: EventStatus,
    /// Whether signups are closed to members regardless of `status`.
    pub registration_locked: bool,
    /// Event-wide cap on places across its missions; `0` means no cap.
    pub max_slots: i64,
    /// Discord id of the member who created the event.
    pub created_by: String,
    /// Game server this operation is scheduled on. Nullable uuid — no FK in schema (house
    /// style; see migration 0011). Absent on the wire when unset.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub server_id: Option<ServerId>,
    /// Modpack this operation requires. Per-event, not the global `/modpacks/current`.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub modpack_id: Option<ModpackId>,
    /// When the row was created (RFC 3339 UTC on the wire).
    #[serde(with = "rfc3339_utc")]
    pub created_at: DateTime<Utc>,
    /// When the row last changed (RFC 3339 UTC on the wire).
    #[serde(with = "rfc3339_utc")]
    pub updated_at: DateTime<Utc>,
}

/// Links an Event to a Mission with its own start time. ORBAT slots + registrations
/// hang off this row.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct EventMission {
    /// Event-mission id (uuid).
    pub id: EventMissionId,
    /// The event this mission belongs to.
    pub event_id: EventId,
    /// The library mission played.
    pub mission_id: MissionId,
    /// When this mission starts within the event (RFC 3339 UTC on the wire).
    #[serde(with = "rfc3339_utc")]
    pub start_time: DateTime<Utc>,
    /// When the row was created (RFC 3339 UTC on the wire).
    #[serde(with = "rfc3339_utc")]
    pub created_at: DateTime<Utc>,
    /// When the row last changed (RFC 3339 UTC on the wire).
    #[serde(with = "rfc3339_utc")]
    pub updated_at: DateTime<Utc>,
}

/// One fillable position in the Order of Battle for a mission within an event.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct OrbatSlot {
    /// Slot id (uuid).
    pub id: OrbatSlotId,
    /// The event mission whose ORBAT holds this slot.
    pub event_mission_id: EventMissionId,
    /// Faction (side) the slot's squad fights for.
    pub faction: String,
    /// Squad name the slot belongs to.
    pub squad: String,
    /// Squad radio callsign; empty (absent on the wire) when unset.
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub callsign: String,
    /// Role played in this slot (for example rifleman or medic).
    pub role: String,
    /// Loadout name the slot spawns with; empty (absent on the wire) when unset.
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub loadout: String,
    /// Free-text tag from the mission's ORBAT template; empty (absent on the wire) when unset.
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub tag: String,
    /// Zero-based position of the slot within its squad.
    pub slot_index: i64,
    /// Discord id of the member seated here; `None` (absent on the wire) when the seat is free.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub assigned_to: Option<String>,
    /// When the current member took the seat; `None` (absent on the wire) when free.
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
    /// Squad hold id (uuid).
    pub id: OrbatReservationId,
    /// The event mission whose squad is held.
    pub event_mission_id: EventMissionId,
    /// Name of the held squad.
    pub squad: String,
    /// Discord id of the leader who placed the hold.
    pub reserved_by: String,
    /// When the hold was placed (RFC 3339 UTC on the wire).
    #[serde(with = "rfc3339_utc")]
    pub reserved_at: DateTime<Utc>,
}

/// A user signed up for a specific mission within an event.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct EventRegistration {
    /// Registration id (uuid).
    pub id: EventRegistrationId,
    /// The event mission signed up for.
    pub event_mission_id: EventMissionId,
    /// Discord id of the registered member.
    pub discord_id: DiscordUserId,
    /// ORBAT seat the member holds; `None` (absent on the wire) when unseated.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub slot_id: Option<OrbatSlotId>,
    /// Combined state: the attendance state when recorded, otherwise the reservation state.
    pub state: RegistrationState,
    /// Reservation state: `registered`, `waitlisted`, `withdrawn` or `legacy_unknown`.
    pub reservation_state: RegistrationState,
    /// Attendance state (`attended` or `no_show`); `None` until attendance is recorded.
    pub attendance_state: Option<RegistrationState>,
    /// When the member signed up (RFC 3339 UTC on the wire).
    #[serde(with = "rfc3339_utc")]
    pub registered_at: DateTime<Utc>,
}
