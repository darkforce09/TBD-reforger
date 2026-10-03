//! Role: the `place_orbat` module tree and the items it re-exports.
//! Position: the `place_orbat` module of `mission_operations`; hosted commands drive it.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use mission_document::MissionDocCore;
use serde_json::Value;
mod placement;

#[cfg(test)]
use placement::is_minted_squad_name;
#[cfg(test)]
use placement::is_open_for_placement;

/// Expose placement :: place character under side at this domain boundary.
pub use placement::place_character_under_side;
#[cfg(test)]
mod tests;
