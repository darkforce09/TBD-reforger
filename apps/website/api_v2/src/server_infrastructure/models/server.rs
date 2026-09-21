//! The three rows that describe a dedicated server: its registration, its single hot status
//! row, and the time series those status rows are archived into.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::core::wire_format::rfc3339_utc;

/// Registered Arma Reforger server instance. `ip` is Postgres `inet` bound as text
/// (queries must `SELECT ip::text`).
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Server {
    pub id: Uuid,
    pub name: String,
    pub ip: String,
    pub port: i64,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub required_modpack_id: Option<Uuid>,
    pub is_active: bool,
}

/// Single hot row of current state per server.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ServerStatus {
    pub server_id: Uuid,
    pub is_online: bool,
    pub player_count: i64,
    pub max_players: i64,
    /// `numeric(5,1)` — queries must `CAST(server_fps AS double precision)`.
    pub server_fps: f64,
    pub uptime_seconds: i64,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub current_match_id: Option<Uuid>,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub ingame_time: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub ingame_weather: String,
    #[serde(with = "rfc3339_utc")]
    pub updated_at: DateTime<Utc>,
}

/// Time-series feed for the "FPS dropped below 20" alert. `id` is a bigint sequence.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ServerStatusHistory {
    pub id: i64,
    pub server_id: Uuid,
    pub player_count: i64,
    /// `numeric(5,1)` — queries must `CAST(server_fps AS double precision)`.
    pub server_fps: f64,
    #[serde(with = "rfc3339_utc")]
    pub recorded_at: DateTime<Utc>,
}
