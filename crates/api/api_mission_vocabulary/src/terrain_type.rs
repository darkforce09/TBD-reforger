//! The terrain a mission is authored on.
//!
//! **Role:** the terrain identifier missions, match records and server intel carry.
//! **Position:** stored in `missions.terrain` and `matches.terrain`; read by missions, match
//! telemetry, operations and the server intel handler.
//! **Signals & state:** none; plain data.
//! **Invariants:** [`TerrainType`] and the Postgres enum `terrain_type` hold the same three
//! values, spelled snake_case on the wire and in SQL; [`TerrainType::as_str`] and its `FromStr`
//! parse are inverse over exactly those values.

use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::error::Error;

/// Terrain identifiers (Postgres ENUM `terrain_type`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "terrain_type", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum TerrainType {
    /// Everon, the built-in island.
    Everon,
    /// Arland, the built-in island.
    Arland,
    /// Any terrain a mod adds.
    Custom,
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

impl FromStr for TerrainType {
    type Err = Error;

    /// The terrain `value` spells, or [`Error::UnknownTerrain`].
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "everon" => Ok(Self::Everon),
            "arland" => Ok(Self::Arland),
            "custom" => Ok(Self::Custom),
            _ => Err(Error::UnknownTerrain(value.to_string())),
        }
    }
}
