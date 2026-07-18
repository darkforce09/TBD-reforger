//! Mission models — Rust port of `internal/models/mission.go`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::RawJson;
use crate::models::serde_helpers::{go_time, go_time_opt};

/// Mission lifecycle states (Postgres ENUM `mission_status`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "mission_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum MissionStatus {
    Draft,
    PendingApproval,
    Live,
    Rejected,
    Archived,
}

/// Terrain identifiers (Postgres ENUM `terrain_type`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "terrain_type", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum TerrainType {
    Everon,
    Arland,
    Custom,
}

/// Game modes (Postgres ENUM `game_mode`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "game_mode", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum GameMode {
    PveCoop,
    Pvp,
    Zeus,
}

impl GameMode {
    /// The Postgres/JSON wire string.
    pub fn as_str(self) -> &'static str {
        match self {
            GameMode::PveCoop => "pve_coop",
            GameMode::Pvp => "pvp",
            GameMode::Zeus => "zeus",
        }
    }
}

/// Weather presets (Postgres ENUM `weather_type`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "weather_type", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum WeatherType {
    Clear,
    Overcast,
    HeavyRain,
    DenseFog,
}

impl TerrainType {
    /// The Postgres/JSON wire string.
    pub fn as_str(self) -> &'static str {
        match self {
            TerrainType::Everon => "everon",
            TerrainType::Arland => "arland",
            TerrainType::Custom => "custom",
        }
    }
}

impl WeatherType {
    /// The Postgres/JSON wire string.
    pub fn as_str(self) -> &'static str {
        match self {
            WeatherType::Clear => "clear",
            WeatherType::Overcast => "overcast",
            WeatherType::HeavyRain => "heavy_rain",
            WeatherType::DenseFog => "dense_fog",
        }
    }
}

/// Custom mission library row; the heavy 2D-editor payload lives in `MissionVersion`.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Mission {
    pub id: Uuid,
    pub title: String,
    pub author_id: String,
    pub terrain: TerrainType,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub custom_terrain_name: String,
    pub game_mode: GameMode,
    pub weather: WeatherType,
    /// `time without time zone` — queries must `SELECT time_of_day::text`.
    pub time_of_day: String,
    pub max_players: i64,
    pub status: MissionStatus,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub thumbnail_url: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub briefing: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub current_version_id: Option<Uuid>,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub rejection_reason: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub reviewed_by: Option<String>,
    #[serde(with = "go_time_opt", skip_serializing_if = "Option::is_none", default)]
    pub reviewed_at: Option<DateTime<Utc>>,
    #[serde(with = "go_time")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "go_time")]
    pub updated_at: DateTime<Utc>,
}

/// Immutable snapshot of the 2D editor output; unique per `(mission, semver)`.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct MissionVersion {
    pub id: Uuid,
    pub mission_id: Uuid,
    pub semver: String,
    /// `jsonb` — passthrough of the Postgres-normalized bytes (hazard #8), never
    /// round-tripped through a re-serialization.
    pub json_payload: RawJson,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub editor_notes: String,
    pub created_by: String,
    #[serde(with = "go_time")]
    pub created_at: DateTime<Utc>,
}

/// One weapon/vehicle/equipment line on the Mission Overview armory.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct MissionArmory {
    pub id: Uuid,
    pub mission_id: Uuid,
    pub faction: String,
    pub category: String,
    pub item_name: String,
    /// `null` = unlimited.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub quantity: Option<i64>,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub icon: String,
    pub sort_order: i64,
}

/// Backs the "Bookmarked" tab in the Mission Library.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct MissionBookmark {
    pub discord_id: String,
    pub mission_id: Uuid,
    #[serde(with = "go_time")]
    pub created_at: DateTime<Utc>,
}
