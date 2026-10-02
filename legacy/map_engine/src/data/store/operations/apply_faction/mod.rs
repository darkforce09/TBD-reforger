//! Role: Module boundary for doc/operations/apply_faction.
//! Position: `doc/operations/apply_faction` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use crate::data::store::MissionDocCore;
use serde_json::Value;
mod library;

/// Expose library :: apply anchor x at this domain boundary.
pub use library::APPLY_ANCHOR_X;

/// Expose library :: apply anchor y at this domain boundary.
pub use library::APPLY_ANCHOR_Y;

/// Expose library ::  apply faction error at this domain boundary.
pub use library::ApplyFactionError;

/// Expose library ::  apply faction result at this domain boundary.
pub use library::ApplyFactionResult;

/// Expose library ::  authored squad at this domain boundary.
pub use library::AuthoredSquad;

/// Expose library ::  faction library input at this domain boundary.
pub use library::FactionLibraryInput;

/// Expose library ::  faction library role at this domain boundary.
pub use library::FactionLibraryRole;

/// Expose library ::  faction library vehicle at this domain boundary.
pub use library::FactionLibraryVehicle;
use library::SLOT_SPACING_X;
use library::VALID_SIDES;
#[cfg(test)]
use library::apply_anchor_for_terrain;
use library::apply_anchor_xy;
mod apply;

/// Expose apply :: apply faction library at this domain boundary.
pub use apply::apply_faction_library;
mod authorship;
use authorship::faction_squad_ids;
#[cfg(test)]
use authorship::is_minted_squad_name;
use authorship::mint_slot_id;
use authorship::mint_squad_id;
use authorship::mint_vehicle_id;
use authorship::plural;
use authorship::squad_authorship;
use authorship::squad_slot_ids;
use authorship::squad_vehicle_ids;
#[cfg(test)]
mod tests;
