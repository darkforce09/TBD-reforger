//! Scheduled operations: the listing, the order of battle, and leave requests.
//!
//! **Role:** everything the operations screens read — an event in a list, the squads and
//! slots its order of battle is made of, the dossier its detail page renders with the viewer's
//! own standing, the answers to registering and to promoting waiting participants, and the leave
//! requests the administration screens review.
//! **Position:** deserialised straight from the backend's JSON and handed to the pages that
//! render it; re-serialised unchanged by the round-trip tests.
//! **Signals & state:** none — these are plain data.
//! **Invariants:** an unclaimed slot serialises as an explicit null rather than being omitted, which is
//! what the captured order-of-battle fixtures carry; omitting it would drift the round-trip. Leave
//! status, seat eligibility, policy sources and release reasons are carried as strings rather than
//! enums so that a value added on the backend cannot make the app reject the response.
//! @contract event-schedule.schema.json#/definitions/EventListItem
//! @contract service-record.schema.json#/definitions/ServiceRecord
//! @contract leave-request.schema.json#/definitions/LeaveRequest

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::event_viewer_access::{EventViewerAccess, ReservationQuotaAvailability};
use super::identifiers::{
    DiscordUserId, EventId, EventMissionId, EventRegistrationId, LeaveRequestId, MissionId,
    ModpackId, OrbatSlotId, ServerId,
};
use super::missions::ArmoryFaction;

/// One operation in a listing: when it runs, how full it is, and the copy the card shows.
/// @contract event-schedule.schema.json#/definitions/EventListItem
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct EventListItem {
    /// Event id (uuid).
    pub id: EventId,
    /// Display name replacing the mission-derived title; empty (absent on the wire) when unset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name_override: Option<String>,
    /// When the operation starts (RFC 3339 UTC on the wire).
    pub start_time: String,
    /// Briefing text shown on the event; empty (absent on the wire) when unset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub briefing: Option<String>,
    /// Banner image URL; empty (absent on the wire) when unset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub banner_image_url: Option<String>,
    /// Lifecycle state of the event.
    pub status: String,
    /// Whether signups are closed to members regardless of `status`.
    pub registration_locked: bool,
    /// Event-wide cap on places across its missions; `0` means no cap.
    pub max_slots: i64,
    /// How many missions the event schedules.
    pub mission_count: i64,
    /// How many members registered for the event.
    pub registered: i64,
    /// How many slots are filled across the event's missions.
    pub filled: i64,
    /// How many slots the event's missions offer.
    pub total_slots: i64,
    /// Percentage filled, as a whole number. The backend computes it in integer arithmetic, so
    /// the wire never carries a fraction and a float here would re-serialise `55` as `55.0`.
    pub percent: i64,
    /// The game server the operation is scheduled on; absent when none is set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server_id: Option<ServerId>,
    /// Every field the API sends beyond the ones named here, kept so a round trip loses nothing.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, Value>,
}

/// One seat in an order of battle, and who holds it.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct OrbatSlot {
    /// Slot id (uuid).
    pub id: OrbatSlotId,
    /// The slot's position within its squad, from one.
    pub number: i64,
    /// Role played in this slot (for example rifleman or medic).
    pub role: String,
    /// Loadout name the slot spawns with; empty (absent on the wire) when unset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loadout: Option<String>,
    /// Free-text tag from the mission's ORBAT template; empty (absent on the wire) when unset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
    /// Zero-based position of the slot within its squad.
    pub slot_index: i64,
    /// Who holds the seat, or null when nobody does. The backend always sends the key — an
    /// unclaimed seat is an explicit null, not an omission — so this must not be skipped when
    /// serialising or the round-trip drifts.
    #[serde(default)]
    pub assigned_to: Option<String>,
    /// The name of the member holding the slot; absent while it is open.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assigned_name: Option<String>,
    /// `eligible` when the seat's effective policy admits the viewer now, `restricted` otherwise.
    /// Any other value is read as restricted, the direction that never offers a seat the backend
    /// would refuse.
    pub viewer_access: String,
    /// Which policy decides the seat: `event`, `squad` or `slot` — the nearest one that is set.
    pub policy_source: String,
}

/// One squad in an order of battle, with its seats.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct OrbatSquad {
    /// Faction (side) the slot's squad fights for.
    pub faction: String,
    /// Squad radio callsign; empty (absent on the wire) when unset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub callsign: Option<String>,
    /// Squad name the slot belongs to.
    pub squad: String,
    /// How many of the squad's slots are filled.
    pub filled: i64,
    /// How many slots the squad has.
    pub total: i64,
    /// The Discord id of the member who reserved the squad; absent while unreserved.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reserved_by: Option<String>,
    /// The name of the member who reserved the squad; absent while unreserved.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reserved_by_name: Option<String>,
    /// The squad's slots, in order.
    pub slots: Vec<OrbatSlot>,
}

/// The mission attached to an operation, as its detail page summarises it.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct EventMissionDossier {
    /// The mission's armory, one entry per faction.
    pub armory_by_faction: Vec<ArmoryFaction>,
    /// The scheduled mission inside the event.
    pub event_mission_id: EventMissionId,
    /// The factions the mission fields.
    pub factions: Vec<String>,
    /// How many of the mission's slots are filled.
    pub filled: i64,
    /// Game mode the mission is played as.
    pub game_mode: String,
    /// The mission the event schedules.
    pub mission_id: MissionId,
    /// When the mission starts, as an RFC 3339 instant.
    pub start_time: String,
    /// Terrain the mission plays on.
    pub terrain: String,
    /// Display title shown in the library.
    pub title: String,
    /// How many slots the mission offers.
    pub total: i64,
    /// Briefing text; empty (omitted on the wire) when unset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub briefing: Option<String>,
    /// Library thumbnail URL; empty (omitted on the wire) when unset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thumbnail_url: Option<String>,
    /// The caller's registration state for the mission; absent when not registered.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub my_state: Option<String>,
    /// Current reservation status controls slotting actions independently of attendance.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub my_reservation_state: Option<String>,
    /// Recorded attendance is independent of the current reservation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub my_attendance_state: Option<String>,
    /// The slot the caller holds in the mission; absent when none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub my_slot_id: Option<OrbatSlotId>,
    /// Some seat of this mission admits the viewer under current membership authority.
    pub viewer_eligible: bool,
    /// Why the viewer's signup was released, while the released signup is retained: one of
    /// `participant_withdrew`, `mission_removed`, `event_cancelled`, `event_deleted`,
    /// `eligibility_lost`, `access_policy_changed`, `account_unavailable` or `seat_cleared`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub my_release_reason: Option<String>,
    /// When the viewer's signup was first released, as an RFC 3339 UTC instant.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub my_withdrawn_at: Option<String>,
    /// The viewer's one-based position in this mission's waiting queue, while waitlisted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub my_waiting_position: Option<i64>,
}

/// Everything the operation detail page renders in one payload.
///
/// A viewer admitted only by squad or slot policies receives only the admitted missions and no
/// operation briefing; an operation the viewer may not see answers exactly like a missing one.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct EventHub {
    /// When the row was created (RFC 3339 UTC on the wire).
    pub created_at: String,
    /// Discord id of the member who created the event.
    pub created_by: String,
    /// Event id (uuid).
    pub id: EventId,
    /// Event-wide cap on places across its missions; `0` means no cap.
    pub max_slots: i64,
    /// The event's scheduled missions, each with its dossier.
    pub missions: Vec<EventMissionDossier>,
    /// Display name replacing the mission-derived title; empty (absent on the wire) when unset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name_override: Option<String>,
    /// Whether signups are closed to members regardless of `status`.
    pub registration_locked: bool,
    /// When the operation starts (RFC 3339 UTC on the wire).
    pub start_time: String,
    /// Lifecycle state of the event.
    pub status: String,
    /// When the row last changed (RFC 3339 UTC on the wire).
    pub updated_at: String,
    /// Briefing text shown on the event; empty (absent on the wire) when unset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub briefing: Option<String>,
    /// Banner image URL; empty (absent on the wire) when unset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub banner_image_url: Option<String>,
    /// The game server this operation is scheduled on; absent on the wire when unset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server_id: Option<ServerId>,
    /// Modpack this operation requires. Per-event, not the global `/modpacks/current`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modpack_id: Option<ModpackId>,
    /// How much of the operation the viewer sees, and the pool a new place comes from first.
    pub viewer_access: EventViewerAccess,
    /// The member, guest and open pools, in that order.
    pub reservation_quotas: Vec<ReservationQuotaAvailability>,
    /// Places left under the operation-wide limit. The backend always sends the key, as an explicit
    /// null when the operation is uncapped, so it must not be skipped when serialising.
    pub remaining_event_places: Option<i64>,
}

/// An upcoming signup carries allocation and attendance independently.
/// @contract service-record.schema.json#/definitions/UpcomingDeployment
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeploymentUpcoming {
    /// The event the deployment serves; `None` outside an event.
    pub event_id: EventId,
    /// The event mission whose ORBAT holds this slot.
    pub event_mission_id: EventMissionId,
    /// The upcoming event's name.
    pub name: String,
    /// The terrain the event plays on.
    pub terrain: String,
    /// When the event starts, as an RFC 3339 instant.
    pub start_time: String,
    /// Combined state: the attendance state when recorded, otherwise the reservation state.
    pub state: String,
    /// Reservation state: `registered`, `waitlisted`, `withdrawn` or `legacy_unknown`.
    pub reservation_state: String,
    /// The API includes this key as null when no attendance is recorded.
    #[serde(default)]
    pub attendance_state: Option<String>,
    /// Faction (side) the slot's squad fights for.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub faction: String,
    /// Squad name the slot belongs to.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub squad: String,
    /// Role played in this slot (for example rifleman or medic).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub role: String,
}

/// The operations a viewer is signed up for, grouped for the dashboard.
/// @contract service-record.schema.json#/definitions/ServiceRecord
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct Deployments {
    /// How many operations the member has deployed on.
    pub total_operations: i64,
    /// The member's attendance rate, in percent.
    pub attendance_rate: f64,
    /// Kills over every recorded match.
    pub kills: i64,
    /// Deaths over every recorded match.
    pub deaths: i64,
    /// `NULL` when no `match_player_stats` row for this player has a measured `deaths` reading.
    /// Distinct from `0.0` (measured zero-death / flawless aggregate).
    pub kd_ratio: Option<f64>,
    /// How many operations the member led.
    pub command_games: i64,
    /// Command-role lines that recorded a command win.
    pub command_wins: i64,
    /// Command wins over command-role lines, from 0 to 1, rounded to three places; 0 with no
    /// command-role line.
    pub command_win_rate: Option<f64>,
    /// The member's service history entries, as the API sends them.
    pub service_history: Vec<Value>,
    /// The upcoming events the member holds a slot in.
    pub upcoming: Vec<DeploymentUpcoming>,
}

/// One leave request, with its review stamp when it has been decided.
/// @contract leave-request.schema.json#/definitions/LeaveRequest
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LeaveRequest {
    /// Leave request id (uuid).
    pub id: LeaveRequestId,
    /// Discord id of the member requesting leave.
    pub discord_id: DiscordUserId,
    /// First day of the leave (a midnight-UTC timestamp on the wire).
    pub starts_on: String,
    /// Last day of the leave (a midnight-UTC timestamp on the wire).
    pub ends_on: String,
    /// The member's stated reason; empty (absent on the wire) when the row holds none.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub reason: String,
    /// One of pending, approved or denied. Carried as a string rather than an enum: the set is
    /// stable today, but a hard enum would make the app reject the response the day a fourth
    /// value lands.
    pub status: String,
    /// Discord id of the reviewing admin; `None` (absent on the wire) until reviewed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reviewed_by: Option<String>,
    /// When the request was filed (RFC 3339 UTC on the wire).
    pub created_at: String,
}

/// The body that files a leave request.
/// @contract leave-request.schema.json#/definitions/LeaveRequestSubmission
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateLeaveInput {
    /// First day of the leave (a midnight-UTC timestamp on the wire).
    pub starts_on: String,
    /// Last day of the leave (a midnight-UTC timestamp on the wire).
    pub ends_on: String,
    /// The member's stated reason; empty (absent on the wire) when the row holds none.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub reason: String,
}

/// Registration actions keep attendance and allocation independent.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReservationResponse {
    /// Compatibility state: the attendance state once recorded, otherwise the reservation state.
    pub state: String,
    /// Reservation outcome: registered, waitlisted, withdrawn or legacy unknown.
    pub reservation_state: String,
    /// Recorded attendance (attended or no show); absent until attendance is recorded.
    pub attendance_state: Option<String>,
    /// The ORBAT seat the registration holds; absent when it names none.
    pub slot_id: Option<OrbatSlotId>,
}

/// The waiting participants one promotion request moved into seats and places, in queue order.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WaitlistPromotion {
    /// Every registration the promotion moved off the waitlist.
    pub promoted: Vec<PromotedRegistration>,
}

/// One waiting participant a promotion seated.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PromotedRegistration {
    /// The promoted registration.
    pub registration_id: EventRegistrationId,
    /// The member whose registration was promoted.
    pub discord_id: DiscordUserId,
    /// The slot the member now holds.
    pub slot_id: OrbatSlotId,
}
