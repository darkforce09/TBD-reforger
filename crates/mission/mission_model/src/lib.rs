//! The mission model: what a compiled mission document holds and what an author may write.
//!
//! **Role:** the compiled rows of a mission document ([`compiled`]), the ORBAT projection of the
//! editor graph ([`orbat`]), the authored extension blocks with their parse and validate rules
//! ([`authored_blocks`] and the block modules), the plain-text slot line ([`slot_line`]) and the
//! newtype ids all of them name ([`ids`]).
//! **Position:** mission tier 1, over `newtype_ids`, `serde`, `serde_json` and `thiserror`. The
//! payload compiler (`mission_payload`), the game-document compiler (`mission_compiler`), the
//! validator (`mission_validation`), the API and the Mission Creator build on it; it depends on no compiler.
//! **Signals & state:** none; plain data types and pure functions.
//! **Invariants:** every row serialises to the exact shape of `mission.schema.json` it projects
//! (ids are serde-transparent); every refusal is one readable sentence ([`Error`]); the
//! [`authored_blocks::AUTHORED_BLOCKS`] order is the order the compiled document emits.

pub mod authored_blocks;
pub mod compiled;
pub mod environment;
mod error;
pub mod ids;
pub mod objectives;
pub mod orbat;
pub mod prelude;
pub mod radio_plan;
pub mod slot_line;
pub mod spawn_modules;
pub mod tactical_graphics;

/// Why a mission model check refuses a value.
pub use error::Error;
/// The result of a mission model check.
pub use error::Result;
