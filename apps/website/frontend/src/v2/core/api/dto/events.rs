//! Scheduled operations: the listing, the order of battle, and leave requests.
//!
//! **Role:** everything the operations screens read — an event in a list, the squads and
//! slots its order of battle is made of, the dossier its detail page renders, and the leave
//! requests the administration screens review.
//! **Position:** deserialised straight from the backend's JSON and handed to the pages that
//! render it; re-serialised unchanged by the round-trip tests.
//! **Signals & state:** none — these are plain data.
//! **Invariants:** an unclaimed slot serialises as an explicit null rather than being omitted, which is
//! what the captured order-of-battle fixtures carry; omitting it would drift the round-trip. Leave
//! status is carried as a string rather than an enum so that a new status added on the backend
//! cannot make the app reject the response.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::missions::ArmoryFaction;

/// One operation in a listing: when it runs, how full it is, and the copy the card shows.
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct EventListItem {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name_override: Option<String>,
    pub start_time: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub briefing: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub banner_image_url: Option<String>,
    pub status: String,
    pub registration_locked: bool,
    pub max_slots: i64,
    pub mission_count: i64,
    pub registered: i64,
    pub filled: i64,
    pub total_slots: i64,
    /// Percentage filled, as a whole number. The backend computes it in integer arithmetic, so
    /// the wire never carries a fraction and a float here would re-serialise `55` as `55.0`.
    pub percent: i64,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, Value>,
}

/// One seat in an order of battle, and who holds it.
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct OrbatSlot {
    pub id: String,
    pub number: i64,
    pub role: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loadout: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
    pub slot_index: i64,
    /// Who holds the seat, or null when nobody does. The backend always sends the key — an
    /// unclaimed seat is an explicit null, not an omission — so this must not be skipped when
    /// serialising or the round-trip drifts.
    #[serde(default)]
    pub assigned_to: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assigned_name: Option<String>,
}

/// One squad in an order of battle, with its seats.
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct OrbatSquad {
    pub faction: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub callsign: Option<String>,
    pub squad: String,
    pub filled: i64,
    pub total: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reserved_by: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reserved_by_name: Option<String>,
    pub slots: Vec<OrbatSlot>,
}

/// The mission attached to an operation, as its detail page summarises it.
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct EventMissionDossier {
    pub armory_by_faction: Vec<ArmoryFaction>,
    pub event_mission_id: String,
    pub factions: Vec<String>,
    pub filled: i64,
    pub game_mode: String,
    pub mission_id: String,
    pub start_time: String,
    pub terrain: String,
    pub title: String,
    pub total: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub briefing: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thumbnail_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub my_state: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub my_slot_id: Option<String>,
}

/// Everything the operation detail page renders in one payload.
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct EventHub {
    pub created_at: String,
    pub created_by: String,
    pub id: String,
    pub max_slots: i64,
    pub missions: Vec<EventMissionDossier>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name_override: Option<String>,
    pub registration_locked: bool,
    pub start_time: String,
    pub status: String,
    pub updated_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub briefing: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub banner_image_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modpack_id: Option<String>,
}

/// The operations a viewer is signed up for, grouped for the dashboard.
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct Deployments {
    pub total_operations: i64,
    pub attendance_rate: f64,
    pub kills: i64,
    pub deaths: i64,
    pub kd_ratio: Option<f64>,
    pub command_games: i64,
    pub command_wins: i64,
    pub command_win_rate: Option<f64>,
    pub service_history: Vec<Value>,
    pub upcoming: Vec<Value>,
}

/// One leave request, with its review stamp when it has been decided.
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LeaveRequest {
    pub id: String,
    pub discord_id: String,
    pub starts_on: String,
    pub ends_on: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub reason: String,
    /// One of pending, approved or denied. Carried as a string rather than an enum: the set is
    /// stable today, but a hard enum would make the app reject the response the day a fourth
    /// value lands.
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reviewed_by: Option<String>,
    pub created_at: String,
}

/// The body that files a leave request.
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateLeaveInput {
    pub starts_on: String,
    pub ends_on: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub reason: String,
}

/// The body that approves or denies a leave request.
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct ReviewLeaveInput {
    pub status: String,
}
