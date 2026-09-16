//! Role: Module boundary for mission/extensions/modules.
//! Position: `mission/extensions/modules` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use serde_json::{Map, Value};
mod spawns;
/// Expose spawns ::  authored spawn module at this domain boundary.
pub use spawns::AuthoredSpawnModule;
/// Expose spawns :: faction keys at this domain boundary.
pub use spawns::FACTION_KEYS;
/// Expose spawns :: kinds at this domain boundary.
pub use spawns::KINDS;
/// Expose spawns :: max alive at this domain boundary.
pub use spawns::MAX_ALIVE;
/// Expose spawns :: parse at this domain boundary.
pub use spawns::parse;
/// Expose spawns :: placement is exclusive at this domain boundary.
pub use spawns::placement_is_exclusive;
/// Expose spawns :: validate at this domain boundary.
pub use spawns::validate;
#[cfg(test)]
mod tests;
