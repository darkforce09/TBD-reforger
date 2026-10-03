//! Mission library models: the library row, its immutable version snapshots, the armory,
//! the authored-default census rows, and bookmarks.
//!
//! @contract mission-review.schema.json#/definitions/MissionRow
//! @contract mission-review.schema.json#/definitions/MissionVersion
//! @contract mission-library.schema.json#/definitions/MissionArmory
//! @contract mission-default-overrides.schema.json#/definitions/MissionDefaultOverride
//! @contract mission-default-overrides.schema.json#/definitions/MissionDefaultValueBucket

use api_identifiers::{
    DiscordUserId, MissionArmoryId, MissionArtifactId, MissionId, MissionVersionId,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use api_foundation::wire_format::RawJson;
use api_mission_vocabulary::{GameMode, TerrainType};
use fleet_wire_contract::rfc3339_timestamps::{rfc3339_utc, rfc3339_utc_opt};

/// Mission lifecycle states (Postgres ENUM `mission_status`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "mission_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum MissionStatus {
    /// Being authored; not yet submitted for review.
    Draft,
    /// Submitted for review and awaiting an approval decision.
    PendingApproval,
    /// Approved; listed in the library and deployable.
    Live,
    /// Declined by a reviewer; `rejection_reason` says why.
    Rejected,
    /// Retired from the library.
    Archived,
}

/// Weather presets (Postgres ENUM `weather_type`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "weather_type", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum WeatherType {
    /// Clear sky.
    Clear,
    /// Overcast sky without precipitation.
    Overcast,
    /// Heavy rain.
    HeavyRain,
    /// Dense fog.
    DenseFog,
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
    /// Primary key (uuid).
    pub id: MissionId,
    /// Display title shown in the library.
    pub title: String,
    /// Discord id of the mission author.
    pub author_id: DiscordUserId,
    /// Terrain the mission plays on.
    pub terrain: TerrainType,
    /// Terrain name when `terrain` is a custom terrain; empty (omitted on the wire) otherwise.
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub custom_terrain_name: String,
    /// Game mode the mission is played as.
    pub game_mode: GameMode,
    /// Weather preset.
    pub weather: WeatherType,
    /// `time without time zone` — queries must `SELECT time_of_day::text`.
    pub time_of_day: String,
    /// Player capacity of the mission.
    pub max_players: i64,
    /// Lifecycle state of the mission.
    pub status: MissionStatus,
    /// Library thumbnail URL; empty (omitted on the wire) when unset.
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub thumbnail_url: String,
    /// Briefing text; empty (omitted on the wire) when unset.
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub briefing: String,
    /// Latest saved [`MissionVersion`]; absent before the first save.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub current_version_id: Option<MissionVersionId>,
    /// The artifact the latest approval decided; deployments load exactly these bytes. Absent on
    /// a mission never approved from an artifact.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub approved_artifact_id: Option<MissionArtifactId>,
    /// Reviewer's reason for the latest rejection; empty (omitted on the wire) otherwise.
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub rejection_reason: String,
    /// Discord id of the reviewer who made the latest review decision.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub reviewed_by: Option<String>,
    /// When the latest review decision was made (RFC 3339 UTC).
    #[serde(
        with = "rfc3339_utc_opt",
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub reviewed_at: Option<DateTime<Utc>>,
    /// When the mission row was created (RFC 3339 UTC).
    #[serde(with = "rfc3339_utc")]
    pub created_at: DateTime<Utc>,
    /// When the mission row last changed (RFC 3339 UTC).
    #[serde(with = "rfc3339_utc")]
    pub updated_at: DateTime<Utc>,
}

/// Immutable snapshot of the 2D editor output; unique per `(mission, semver)`.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct MissionVersion {
    /// Primary key (uuid).
    pub id: MissionVersionId,
    /// The [`Mission`] this version belongs to.
    pub mission_id: MissionId,
    /// Semantic version string of this snapshot, e.g. `0.1.0`.
    pub semver: String,
    /// `jsonb` — passthrough of the Postgres-normalized bytes (hazard #8), never
    /// round-tripped through a re-serialization.
    pub json_payload: RawJson,
    /// Author's notes on this version; empty (omitted on the wire) when unset.
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub editor_notes: String,
    /// Discord id of the user who saved this version.
    pub created_by: String,
    /// When this version was saved (RFC 3339 UTC).
    #[serde(with = "rfc3339_utc")]
    pub created_at: DateTime<Utc>,
}

/// One weapon/vehicle/equipment line on the Mission Overview armory.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct MissionArmory {
    /// Primary key (uuid).
    pub id: MissionArmoryId,
    /// The [`Mission`] this armory line belongs to.
    pub mission_id: MissionId,
    /// Faction name the line belongs to; matches an ORBAT slot faction of the mission exactly.
    pub faction: String,
    /// Armory category the line is grouped under (weapon, vehicle, equipment).
    pub category: String,
    /// Display name of the item.
    pub item_name: String,
    /// `null` = unlimited.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub quantity: Option<i64>,
    /// Icon reference for the line; empty (omitted on the wire) when unset.
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub icon: String,
    /// Ascending display order of the line within the armory.
    pub sort_order: i64,
}

/// One authored default key on `GET /api/v1/admin/mission-default-overrides`.
///
/// The row answers, for a single schema-`default`-bearing key, "how often does an author
/// change this away from what the mod would do if they wrote nothing?" — the query WOG could
/// only get by machine-parsing 171 shipped PBOs (`wog.md:1078`), which TBD owns as a table.
///
/// `default_value` and the `key` pointer are read FROM `mission.schema.json` at runtime (see
/// `api_missions::handlers::mission_default_overrides::schema_default_keys`); nothing here is
/// hardcoded, because the
/// schema owns the defaults and nothing else restates them. The counts are over the LATEST
/// version of every mission (the `current_version_id` join, the same one the library reads).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionDefaultOverride {
    /// JSON-pointer-style path to the key inside a stored `zones[].rules` object, e.g.
    /// `zones[].rules.graceSeconds`. This is the AUTHORED (editor-payload) location, not the
    /// compiled-document pointer — the two are distinct namespaces.
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

/// One `(value, count)` bar of a [`MissionDefaultOverride`] histogram.
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
    /// Discord id of the member who bookmarked the mission.
    pub discord_id: DiscordUserId,
    /// The bookmarked [`Mission`].
    pub mission_id: MissionId,
    /// When the bookmark was created (RFC 3339 UTC).
    #[serde(with = "rfc3339_utc")]
    pub created_at: DateTime<Utc>,
}
