//! The mission-domain contract layer: runtime JSON-Schema validation of every document the
//! domain accepts or serves, and the hand-maintained loadout projection.
//! The types generated from the domain's schemas live in the `contract_schema_types` crate.
//!
//! Callers inside the crate name the leaf module they need
//! (`contract::schema_validators::…`); the re-exports below exist so the domain's own
//! contract surface reads as one thing from its boundary.

pub mod loadout_projection;
pub mod schema_validators;
pub mod zone_quantisation;

pub use schema_validators::{
    ContractError, validate_faction_library_doc, validate_mission_document,
    validate_mission_editor_payload, validate_registry_compat_envelope,
    validate_registry_items_envelope,
};
