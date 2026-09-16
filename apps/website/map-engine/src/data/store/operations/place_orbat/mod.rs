//! Role: Module boundary for doc/operations/place_orbat.
//! Position: `doc/operations/place_orbat` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use crate::data::store::MissionDocCore;
use serde_json::Value;
mod placement;

/// Expose placement ::  place orbat error at this domain boundary.
pub use placement::PlaceOrbatError;
#[cfg(test)]
use placement::is_minted_squad_name;
#[cfg(test)]
use placement::is_open_for_placement;

/// Expose placement :: place character under side at this domain boundary.
pub use placement::place_character_under_side;
#[cfg(test)]
mod tests;
