//! The mission-domain contract layer: runtime JSON-Schema validation of every document the
//! domain accepts or serves, the kit-aliases table, and the hand-maintained loadout projection.
//! The types generated from the domain's schemas live in the `contract_schema_types` crate.
//!
//! Callers inside the crate name the leaf module they need
//! (`missions::contract::schema_validators::…`); the re-exports below exist so the domain's own
//! contract surface reads as one thing from its boundary.

pub mod loadout_projection;
pub mod schema_validators;
pub mod zone_quantisation;

/// The kit-aliases table lives in the shared map engine, next to the compiler that consumes it;
/// it is re-exported here so the contract surface is reachable from one path.
pub use mission_payload::kit_aliases::{KitAliases, load_kit_aliases};
pub use schema_validators::{
    ContractError, validate_faction_library_doc, validate_mission_document,
    validate_mission_editor_payload, validate_registry_compat_envelope,
    validate_registry_items_envelope,
};
