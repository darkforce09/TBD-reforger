//! The game mode a mission is played in.
//!
//! **Role:** the game mode identifier missions carry and the event hub reports per attachment.
//! **Position:** stored in `missions.game_mode`; read by missions and operations.
//! **Signals & state:** none; plain data.
//! **Invariants:** [`GameMode`] and the Postgres enum `game_mode` hold the same three values,
//! spelled snake_case on the wire and in SQL; [`GameMode::as_str`] and its `FromStr` parse are
//! inverse over exactly those values.

use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::error::Error;

/// Game modes (Postgres ENUM `game_mode`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "game_mode", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum GameMode {
    /// Players together against AI.
    PveCoop,
    /// Players against players.
    Pvp,
    /// A game master directs the session live.
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

impl FromStr for GameMode {
    type Err = Error;

    /// The game mode `value` spells, or [`Error::UnknownGameMode`].
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "pve_coop" => Ok(Self::PveCoop),
            "pvp" => Ok(Self::Pvp),
            "zeus" => Ok(Self::Zeus),
            _ => Err(Error::UnknownGameMode(value.to_string())),
        }
    }
}
