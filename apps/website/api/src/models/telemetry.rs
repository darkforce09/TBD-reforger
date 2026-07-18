//! Telemetry models — Rust port of `internal/models/telemetry.go`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::TerrainType;
use crate::models::serde_helpers::{go_time, go_time_opt};

/// Mission outcomes (Postgres ENUM `mission_outcome`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "mission_outcome", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum MissionOutcome {
    Success,
    Failure,
    Aborted,
    Pending,
}

impl MissionOutcome {
    /// The Postgres/JSON wire string.
    pub fn as_str(self) -> &'static str {
        match self {
            MissionOutcome::Success => "success",
            MissionOutcome::Failure => "failure",
            MissionOutcome::Aborted => "aborted",
            MissionOutcome::Pending => "pending",
        }
    }
}

/// One completed operation instance.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Match {
    pub id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub source_match_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub event_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub mission_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub terrain: Option<TerrainType>,
    #[serde(with = "go_time")]
    pub started_at: DateTime<Utc>,
    #[serde(with = "go_time_opt", skip_serializing_if = "Option::is_none", default)]
    pub ended_at: Option<DateTime<Utc>>,
    pub outcome: MissionOutcome,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub winning_faction: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub aar_replay_url: String,
    #[serde(with = "go_time")]
    pub created_at: DateTime<Utc>,
}

/// Per-player line item ingested from the game server.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct MatchPlayerStat {
    pub id: Uuid,
    pub match_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub discord_id: Option<String>,
    pub arma_id: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub role_played: String,
    pub kills: i64,
    pub deaths: i64,
    pub team_kills: i64,
    pub longest_kill_m: i64,
    pub vehicles_destroyed: i64,
    pub is_command: bool,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub command_win: Option<bool>,
    pub source_event_id: String,
    #[serde(with = "go_time")]
    pub created_at: DateTime<Utc>,
}

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
    #[serde(with = "go_time")]
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
    #[serde(with = "go_time")]
    pub recorded_at: DateTime<Utc>,
}
