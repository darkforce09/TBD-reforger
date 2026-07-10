//! Contract layer — Rust port of `internal/contract`: runtime JSON-Schema validation,
//! the kit-aliases table, and (added by codegen) the cross-boundary type projections.

pub mod generated;
pub mod validate;

// Kit-aliases table ported to the shared crate (T-145 Phase 2); re-exported so
// `crate::contract::…` callers (mission_compile) are unchanged.
pub use map_engine_core::mission::kit::{KitAliases, load_kit_aliases};
pub use validate::{
    ContractError, validate_mission_document, validate_mission_editor_payload,
    validate_registry_compat_envelope, validate_registry_items_envelope,
};
