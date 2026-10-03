//! The mission vocabulary enums more than one API domain names.
//!
//! **Role:** the terrain and game mode identifiers a mission, a match record, a server status and
//! an event hub all speak, each with its one spelling on the wire and in Postgres.
//! **Position:** the lowest API crate beside `api_identifiers`, on serde and sqlx alone; read by
//! missions, match telemetry, operations and server infrastructure and by the API's integration
//! suites.
//! **Signals & state:** none; plain data.
//! **Invariants:** each enum and its Postgres ENUM hold the same values, spelled snake_case on the
//! wire and in SQL; `as_str` and `FromStr` are inverse over exactly those values.

mod error;
pub mod game_mode;
pub mod prelude;
pub mod terrain_type;

pub use error::{Error, Result};
pub use game_mode::GameMode;
pub use terrain_type::TerrainType;

#[cfg(test)]
#[path = "tests/vocabulary_spelling.rs"]
mod tests;
