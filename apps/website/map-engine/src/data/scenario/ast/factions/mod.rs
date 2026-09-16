//! Role: Module boundary for mission/ast/factions.
//! Position: `mission/ast/factions` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
mod orbat_slot_template;
/// Expose orbat slot template ::  orbat slot template at this domain boundary.
pub use orbat_slot_template::OrbatSlotTemplate;
/// Expose orbat slot template ::  orbat squad template at this domain boundary.
pub use orbat_slot_template::OrbatSquadTemplate;
/// Expose orbat slot template :: derive orbat from editor at this domain boundary.
pub use orbat_slot_template::derive_orbat_from_editor;
/// Expose orbat slot template :: parse orbat template at this domain boundary.
pub use orbat_slot_template::parse_orbat_template;
/// Expose orbat slot template :: validate faction join key at this domain boundary.
pub use orbat_slot_template::validate_faction_join_key;
#[cfg(test)]
mod tests;
