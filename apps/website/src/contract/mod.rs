//! Contract layer — Rust port of `internal/contract`: runtime JSON-Schema validation,
//! the kit-aliases table, and (added by codegen) the cross-boundary type projections.

pub mod generated;
pub mod kit_aliases;
pub mod validate;

pub use kit_aliases::{KitAliases, load_kit_aliases};
pub use validate::{ContractError, validate_mission_document, validate_mission_editor_payload};
