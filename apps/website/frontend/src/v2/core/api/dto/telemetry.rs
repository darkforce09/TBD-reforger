//! Aggregate figures: the dashboard summary, the leaderboards, and a fire solution.
//!
//! **Role:** the derived numbers the platform reports back — what the dashboard shows at a
//! glance, how the leaderboards rank, and the ballistic answer the mortar tool asks for.
//! **Position:** deserialised straight from the backend's JSON and handed to the pages that
//! render it; re-serialised unchanged by the round-trip tests.
//! **Signals & state:** none — these are plain data.
//! **Invariants:** the fire solution is computed on the backend; the mortar page sends the geometry and
//! renders what comes back, so nothing here is recomputed on the client.

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
    pub server_status: Option<ServerStatusDto>,
    pub current_modpack: Option<ModpackDto>,
    pub recent_announcements: Vec<Value>,
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
