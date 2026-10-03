//! Role: the `apply_faction` module tree and the items it re-exports.
//! Position: the `apply_faction` module of `mission_operations`; hosted commands drive it.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use mission_document::MissionDocCore;
use serde_json::Value;
mod library;

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
pub(crate) use authorship::plural;
use authorship::squad_authorship;
use authorship::squad_slot_ids;
use authorship::squad_vehicle_ids;
#[cfg(test)]
mod tests;
