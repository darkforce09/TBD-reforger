//! Aggregate figures: the dashboard summary with its fleet, the leaderboards, and the fire
//! missions of the mortar tool.
//!
//! **Role:** the derived numbers the platform reports back — what the dashboard shows at a
//! glance (the configured fleet and its totals among it, and the newest announcements), how the
//! leaderboards rank, the ballistic answer the mortar tool asks for, and the fire missions saved
//! against an event.
//! **Position:** deserialised straight from the backend's JSON and handed to the pages that
//! render it; re-serialised unchanged by the round-trip tests.
//! **Signals & state:** none — these are plain data.
//! **Invariants:** the fire solution is computed on the backend; the mortar page sends the geometry and
//! renders what comes back, so nothing here is recomputed on the client. The fleet totals are
//! the backend's sums over the active servers and are rendered as sent, never re-added from the
//! rows. The fleet's server statuses carry the telemetry queue reading the contract defines. A
//! null the backend sends — an unmeasured K/D ratio, a saved fire mission's unrecorded
//! coordinates or solution figures — stays an explicit `null` when serialising.
//! @contract match-telemetry.schema.json#/definitions/TelemetryQueueStatus
//! @contract command-center.schema.json#/definitions/Dashboard
//! @contract command-center.schema.json#/definitions/LeaderboardPage
//! @contract fire-mission.schema.json#/definitions/FireSolution
//! @contract fire-mission.schema.json#/definitions/FireMissionList

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::content::{Announcement, ModpackDto};
use super::servers::ServerStatusDto;

/// Everything the dashboard renders in one payload.
/// @contract command-center.schema.json#/definitions/Dashboard
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct DashboardResponse {
    pub next_event: Option<Value>,
    pub my_assignment: Option<Value>,
    pub fleet: FleetOverviewDto,
    pub current_modpack: Option<ModpackDto>,
    /// The three newest published announcements, newest first.
    pub recent_announcements: Vec<Announcement>,
}

/// The configured fleet: every active server, ordered by name then id, with the fleet totals.
/// @contract command-center.schema.json#/definitions/FleetOverview
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FleetOverviewDto {
    pub servers: Vec<FleetServerDto>,
    pub totals: FleetTotalsDto,
}

/// One active server of the fleet.
/// @contract command-center.schema.json#/definitions/FleetServer
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FleetServerDto {
    pub server_id: String,
    pub name: String,
    /// Absent when the server has no status row yet; such a server counts as offline.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<ServerStatusDto>,
}

/// Aggregates over the fleet. `online`, `players` and `max_players` count online servers only;
/// the two telemetry figures sum every reported queue reading.
/// @contract command-center.schema.json#/definitions/FleetTotals
#[allow(dead_code)]
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FleetTotalsDto {
    /// How many active servers the fleet has.
    pub configured: i64,
    pub online: i64,
    pub players: i64,
    pub max_players: i64,
    pub telemetry_backlog: i64,
    pub telemetry_dropped_total: i64,
}

/// One page of a leaderboard, already ranked and filtered by the backend.
/// @contract command-center.schema.json#/definitions/LeaderboardPage
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct Leaderboard {
    /// The ordering the page was ranked by: `kd`, `command_win`, `missions`, `longest_kill` or
    /// `team_kills`.
    pub category: String,
    pub data: Vec<LeaderboardRow>,
}

/// One member's combat totals on a leaderboard page.
///
/// `kd_ratio` is kills per death rounded to two places, `None` when no match recorded a death
/// count; `command_win_rate` is a fraction between zero and one; `rank` is the 1-based position
/// on the requested page.
/// @contract command-center.schema.json#/definitions/LeaderboardRow
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LeaderboardRow {
    pub discord_id: String,
    pub username: String,
    /// The avatar address; empty when the member has none.
    pub avatar_url: String,
    pub kills: i64,
    pub deaths: i64,
    pub kd_ratio: Option<f64>,
    pub team_kills: i64,
    pub longest_kill_m: i64,
    pub vehicles_destroyed: i64,
    pub missions_played: i64,
    pub command_wins: i64,
    pub command_win_rate: f64,
    pub rank: i64,
}

/// A firing solution for one target: the elevations and flight times the backend
/// computed, with whatever the request could not be answered for reported alongside.
/// @contract fire-mission.schema.json#/definitions/FireSolution
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct FireSolution {
    pub weapon_system: String,
    pub distance_m: i64,
    pub azimuth_deg: f64,
    pub azimuth_mils: i64,
    pub elevation_mils: i64,
    pub charge: i64,
    pub time_of_flight_s: f64,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, Value>,
}

/// One fire mission saved against an event, as the per-event list and the save answer carry it.
///
/// **Typed, and with no default on anything the backend marks required.** A renamed column fails
/// the decode and the saved list goes to its error state, instead of rendering a confident `0 m`.
///
/// `event_id` is absent for a fire mission saved with no event. The four coordinates and the
/// charge, `azimuth_mils` and `time_of_flight_s` are `null` on a row stored before they were
/// recorded; they are also defaulted, so a response captured before those columns existed still
/// decodes. Every other field stays required.
/// @contract fire-mission.schema.json#/definitions/FireMission
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SavedFire {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,
    pub created_by: String,
    pub weapon_system: String,
    pub fp_grid: String,
    pub target_grid: String,
    pub distance_m: i64,
    pub azimuth_deg: f64,
    pub elevation_mils: i64,
    /// The four coordinates, as real numbers. `None` for a row written before the columns
    /// existed, whose only record of them is the `x, y` text of the two grid strings.
    #[serde(default)]
    pub fp_x: Option<f64>,
    #[serde(default)]
    pub fp_y: Option<f64>,
    #[serde(default)]
    pub tgt_x: Option<f64>,
    #[serde(default)]
    pub tgt_y: Option<f64>,
    #[serde(default)]
    pub azimuth_mils: Option<i64>,
    #[serde(default)]
    pub charge: Option<i64>,
    #[serde(default)]
    pub time_of_flight_s: Option<f64>,
    pub created_at: String,
}
