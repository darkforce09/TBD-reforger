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

/// T-683 — one authored default key on `GET /api/v1/admin/mission-default-overrides`.
///
/// The row answers, for a single schema-`default`-bearing key, "how often does an author
/// change this away from what the mod would do if they wrote nothing?" — the query WOG could
/// only get by machine-parsing 171 shipped PBOs (`wog.md:1078`), which TBD owns as a table.
///
/// `default_value` and the `key` pointer are read FROM `mission.schema.json` at runtime (see
/// [`crate::handlers::missions::schema_default_keys`]); nothing here is hardcoded, because the
/// ticket's whole point is that the schema owns the defaults. The counts are over the LATEST
/// version of every mission (the `current_version_id` join, the same one the library reads).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionDefaultOverride {
    /// JSON-pointer-style path to the key inside a stored `zones[].rules` object, e.g.
    /// `zones[].rules.graceSeconds`. This is the AUTHORED (editor-payload) location, not the
    /// compiled-document pointer — the two are distinct namespaces (T-357).
    pub key: String,
    /// The `default` this key declares in `mission.schema.json` — the value an author gets by
    /// writing nothing. Carried verbatim as JSON so a string default (`"warn"`), a number
    /// (`30`) and a bool (`true`) all round-trip unchanged.
    pub default_value: serde_json::Value,
    /// Missions whose latest version has AT LEAST ONE authored zone (the population this
    /// fraction is over — a mission that authors no zone rules cannot override a rule).
    pub missions_total: i64,
    /// Of `missions_total`, how many authored a value for this key, in any zone, that DIFFERS
    /// from `default_value`.
    pub missions_overriding: i64,
    /// `missions_overriding / missions_total`, or `0.0` when `missions_total` is 0. The single
    /// number `wog.md:1078` turns on ("if 43% of missions disable your default…").
    pub override_fraction: f64,
    /// Every DISTINCT authored value for this key across all latest versions, with its mission
    /// count — the value histogram. Sorted by descending count then value for a stable wire.
    pub histogram: Vec<MissionDefaultValueBucket>,
}

/// T-683 — one `(value, count)` bar of a [`MissionDefaultOverride`] histogram.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionDefaultValueBucket {
    /// A distinct authored value for the key (the default's value included when authors write
    /// it explicitly), carried verbatim as JSON.
    pub value: serde_json::Value,
    /// Missions (latest-version, distinct) that authored this value for the key in any zone.
    pub count: i64,
}

/// Backs the "Bookmarked" tab in the Mission Library.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct MissionBookmark {
    pub discord_id: String,
    pub mission_id: Uuid,
    #[serde(with = "go_time")]
    pub created_at: DateTime<Utc>,
}
