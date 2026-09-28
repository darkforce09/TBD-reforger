//! The three rows that describe a dedicated server: its registration, its single hot status
//! row (with its outbound telemetry queue reading), and the time series those status rows are
//! archived into.
//!
//! @contract server-intel.schema.json#/definitions/ServerStatus
//! @contract server-intel.schema.json#/definitions/TelemetryQueueStatus

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

/// Single hot row of current state per server, read through [`ServerStatusRow`].
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    /// The last outbound telemetry queue reading; absent when the server never reported one.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub telemetry_queue: Option<TelemetryQueueStatus>,
}

/// A game runtime's outbound telemetry queue as its last heartbeat reported it.
/// @contract match-telemetry.schema.json#/definitions/TelemetryQueueStatus
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TelemetryQueueStatus {
    pub backlog: i64,
    pub capacity: i64,
    pub dropped_total: i64,
    pub oldest_age_seconds: i64,
    #[serde(with = "rfc3339_utc")]
    pub reported_at: DateTime<Utc>,
}

/// The flat `server_statuses` projection `server_status_columns!` reads; converts into
/// [`ServerStatus`], folding the five queue columns (all set or all null) into one value.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ServerStatusRow {
    pub server_id: Uuid,
    pub is_online: bool,
    pub player_count: i64,
    pub max_players: i64,
    pub server_fps: f64,
    pub uptime_seconds: i64,
    pub current_match_id: Option<Uuid>,
    pub ingame_time: String,
    pub ingame_weather: String,
    pub updated_at: DateTime<Utc>,
    pub telemetry_queue_backlog: Option<i64>,
    pub telemetry_queue_capacity: Option<i64>,
    pub telemetry_queue_dropped_total: Option<i64>,
    pub telemetry_queue_oldest_age_seconds: Option<i64>,
    pub telemetry_queue_reported_at: Option<DateTime<Utc>>,
}

/// The columns of [`ServerStatusRow`] over the `server_statuses` alias `s`, as a literal so
/// `concat!` builds `'static` queries (sqlx refuses runtime-built SQL without an assertion).
macro_rules! server_status_columns {
    () => {
        "s.server_id, s.is_online, s.player_count, s.max_players, \
         s.server_fps::float8 AS server_fps, s.uptime_seconds, s.current_match_id, \
         COALESCE(s.ingame_time, '') AS ingame_time, \
         COALESCE(s.ingame_weather, '') AS ingame_weather, \
         COALESCE(s.updated_at, '0001-01-01 00:00:00+00'::timestamptz) AS updated_at, \
         s.telemetry_queue_backlog, s.telemetry_queue_capacity, s.telemetry_queue_dropped_total, \
         s.telemetry_queue_oldest_age_seconds, s.telemetry_queue_reported_at"
    };
}
pub(crate) use server_status_columns;

impl From<ServerStatusRow> for ServerStatus {
    fn from(row: ServerStatusRow) -> Self {
        let telemetry_queue = match (
            row.telemetry_queue_backlog,
            row.telemetry_queue_capacity,
            row.telemetry_queue_dropped_total,
            row.telemetry_queue_oldest_age_seconds,
            row.telemetry_queue_reported_at,
        ) {
            (
                Some(backlog),
                Some(capacity),
                Some(dropped_total),
                Some(oldest_age_seconds),
                Some(reported_at),
            ) => Some(TelemetryQueueStatus {
                backlog,
                capacity,
                dropped_total,
                oldest_age_seconds,
                reported_at,
            }),
            _ => None,
        };
        Self {
            server_id: row.server_id,
            is_online: row.is_online,
            player_count: row.player_count,
            max_players: row.max_players,
            server_fps: row.server_fps,
            uptime_seconds: row.uptime_seconds,
            current_match_id: row.current_match_id,
            ingame_time: row.ingame_time,
            ingame_weather: row.ingame_weather,
            updated_at: row.updated_at,
            telemetry_queue,
        }
    }
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
