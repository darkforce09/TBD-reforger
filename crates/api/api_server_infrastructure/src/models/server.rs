//! The three rows that describe a dedicated server: its registration, its single hot status
//! row (with its outbound telemetry queue reading), and the time series those status rows are
//! archived into.
//!
//! @contract server-intel.schema.json#/definitions/ServerStatus
//! @contract server-intel.schema.json#/definitions/TelemetryQueueStatus

use api_identifiers::{MatchId, ModpackId, ServerId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use fleet_wire_contract::rfc3339_timestamps::rfc3339_utc;

/// Registered Arma Reforger server instance. `ip` is Postgres `inet` bound as text
/// (queries must `SELECT ip::text`).
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Server {
    /// The server's id.
    pub id: ServerId,
    /// The display name administrators gave the server.
    pub name: String,
    /// The game host address, Postgres `inet` read back as text.
    pub ip: String,
    /// The game port players connect to.
    pub port: i64,
    /// The modpack the server requires, when it requires one.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub required_modpack_id: Option<ModpackId>,
    /// Whether the server belongs to the configured fleet.
    pub is_active: bool,
}

/// Single hot row of current state per server, read through [`ServerStatusRow`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerStatus {
    /// The server the status describes.
    pub server_id: ServerId,
    /// Whether the last heartbeat reported the server online.
    pub is_online: bool,
    /// The players connected at the last heartbeat.
    pub player_count: i64,
    /// The player slots the server offers.
    pub max_players: i64,
    /// `numeric(5,1)` — queries must `CAST(server_fps AS double precision)`.
    pub server_fps: f64,
    /// Seconds the game runtime has been up.
    pub uptime_seconds: i64,
    /// The match the server is running, when one is live.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub current_match_id: Option<MatchId>,
    /// The in-game clock the last heartbeat reported; empty when none was reported.
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub ingame_time: String,
    /// The in-game weather the last heartbeat reported; empty when none was reported.
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub ingame_weather: String,
    /// When the status row was last written.
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
    /// Telemetry lines waiting to be sent.
    pub backlog: i64,
    /// The queue's capacity in lines.
    pub capacity: i64,
    /// Lines dropped since the runtime started because the queue was full.
    pub dropped_total: i64,
    /// Age in seconds of the oldest line still waiting.
    pub oldest_age_seconds: i64,
    /// When the runtime took the reading.
    #[serde(with = "rfc3339_utc")]
    pub reported_at: DateTime<Utc>,
}

/// The flat `server_statuses` projection `server_status_columns!` reads; converts into
/// [`ServerStatus`], folding the five queue columns (all set or all null) into one value.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ServerStatusRow {
    /// The `server_id` column.
    pub server_id: ServerId,
    /// The `is_online` column.
    pub is_online: bool,
    /// The `player_count` column.
    pub player_count: i64,
    /// The `max_players` column.
    pub max_players: i64,
    /// The `server_fps` column, cast to `float8`.
    pub server_fps: f64,
    /// The `uptime_seconds` column.
    pub uptime_seconds: i64,
    /// The `current_match_id` column.
    pub current_match_id: Option<MatchId>,
    /// The `ingame_time` column, empty when null.
    pub ingame_time: String,
    /// The `ingame_weather` column, empty when null.
    pub ingame_weather: String,
    /// The `updated_at` column, the zero time when null.
    pub updated_at: DateTime<Utc>,
    /// The `telemetry_queue_backlog` column.
    pub telemetry_queue_backlog: Option<i64>,
    /// The `telemetry_queue_capacity` column.
    pub telemetry_queue_capacity: Option<i64>,
    /// The `telemetry_queue_dropped_total` column.
    pub telemetry_queue_dropped_total: Option<i64>,
    /// The `telemetry_queue_oldest_age_seconds` column.
    pub telemetry_queue_oldest_age_seconds: Option<i64>,
    /// The `telemetry_queue_reported_at` column.
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
