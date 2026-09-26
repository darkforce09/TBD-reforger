//! Aggregate figures: the dashboard summary with its fleet, the leaderboards, and a fire solution.
//!
//! **Role:** the derived numbers the platform reports back — what the dashboard shows at a
//! glance (the configured fleet and its totals among it), how the leaderboards rank, and the
//! ballistic answer the mortar tool asks for.
//! **Position:** deserialised straight from the backend's JSON and handed to the pages that
//! render it; re-serialised unchanged by the round-trip tests.
//! **Signals & state:** none — these are plain data.
//! **Invariants:** the fire solution is computed on the backend; the mortar page sends the geometry and
//! renders what comes back, so nothing here is recomputed on the client. The fleet totals are
//! the backend's sums over the active servers and are rendered as sent, never re-added from the
//! rows. The fleet shape has no JSON Schema definition yet; its server statuses carry the
//! telemetry queue reading the contract defines.
//! @contract match-telemetry.schema.json#/definitions/TelemetryQueueStatus

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::content::ModpackDto;
use super::servers::ServerStatusDto;

/// Everything the dashboard renders in one payload.
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct DashboardResponse {
    pub next_event: Option<Value>,
    pub my_assignment: Option<Value>,
    pub fleet: FleetOverviewDto,
    pub current_modpack: Option<ModpackDto>,
    pub recent_announcements: Vec<Value>,
}

/// The configured fleet: every active server, ordered by name then id, with the fleet totals.
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FleetOverviewDto {
    pub servers: Vec<FleetServerDto>,
    pub totals: FleetTotalsDto,
}

/// One active server of the fleet.
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

/// One leaderboard, already ranked by the backend.
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct Leaderboard {
    pub category: String,
    pub data: Vec<Value>,
}

/// A firing solution for one target: the elevations and flight times the backend
/// computed, with whatever the request could not be answered for reported alongside.
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
