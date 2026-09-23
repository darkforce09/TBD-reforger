// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/mission-review.schema.json — regenerate with: cargo xtask ci schema-codegen

use super::{GameMode, TerrainType, WeatherType};

///The mission fields the compiler read, in the canonical form the metadata digest covers.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct ArtifactMetadata {
    pub author: ::std::string::String,
    pub custom_terrain_name: ::std::string::String,
    pub game_mode: GameMode,
    pub id: ::uuid::Uuid,
    pub max_players: i64,
    pub terrain: TerrainType,
    pub time_of_day: ::std::string::String,
    pub title: ::std::string::String,
    pub weather: WeatherType,
}
