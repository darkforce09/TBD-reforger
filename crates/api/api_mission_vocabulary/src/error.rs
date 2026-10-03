//! Why a string names no terrain or game mode.
//!
//! **Role:** the crate's error type and its `Result` alias.
//! **Position:** returned by the `FromStr` parses of [`crate::TerrainType`] and
//! [`crate::GameMode`]; the mission validation answers it as a 400, the match ingest reads it as
//! an absent terrain.
//! **Signals & state:** none; plain data.
//! **Invariants:** each variant carries the refused string exactly as it was given.

/// A string that names no value of a mission vocabulary enum.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// The string is not `everon`, `arland` or `custom`.
    #[error("unknown terrain `{0}`")]
    UnknownTerrain(String),
    /// The string is not `pve_coop`, `pvp` or `zeus`.
    #[error("unknown game mode `{0}`")]
    UnknownGameMode(String),
}

/// The result of a parse of this crate.
pub type Result<T> = std::result::Result<T, Error>;
