//! Aggregate figures: the dashboard summary with its fleet, and the leaderboards.
//!
//! **Role:** the derived numbers the platform reports back — what the dashboard shows at a
//! glance (the configured fleet and its totals among it, and the newest announcements), and how
//! the leaderboards rank.
//! **Position:** deserialised straight from the backend's JSON and handed to the pages that
//! render it; re-serialised unchanged by the round-trip tests.
//! **Signals & state:** none — these are plain data.
//! **Invariants:** the fleet totals are the backend's sums over the active servers and are
//! rendered as sent, never re-added from the rows. The fleet's server statuses carry the
//! telemetry queue reading the contract defines. A null the backend sends — an unmeasured K/D
//! ratio — stays an explicit `null` when serialising.
//! @contract match-telemetry.schema.json#/definitions/TelemetryQueueStatus
//! @contract command-center.schema.json#/definitions/Dashboard
//! @contract command-center.schema.json#/definitions/LeaderboardPage

#[cfg(any(target_arch = "wasm32", test))]
use serde::{Deserialize, Serialize};
#[cfg(any(target_arch = "wasm32", test))]
use serde_json::Value;

#[cfg(any(target_arch = "wasm32", test))]
use super::content::{Announcement, ModpackDto};
#[cfg(any(target_arch = "wasm32", test))]
use super::servers::ServerStatusDto;

/// Everything the dashboard renders in one payload.
/// @contract command-center.schema.json#/definitions/Dashboard
#[cfg(any(target_arch = "wasm32", test))]
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
#[cfg(any(target_arch = "wasm32", test))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FleetOverviewDto {
    pub servers: Vec<FleetServerDto>,
    pub totals: FleetTotalsDto,
}

/// One active server of the fleet.
/// @contract command-center.schema.json#/definitions/FleetServer
#[cfg(any(target_arch = "wasm32", test))]
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
#[cfg(any(target_arch = "wasm32", test))]
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
#[cfg(any(target_arch = "wasm32", test))]
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
#[cfg(any(target_arch = "wasm32", test))]
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
