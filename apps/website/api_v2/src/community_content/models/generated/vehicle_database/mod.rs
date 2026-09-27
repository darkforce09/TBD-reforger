// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/vehicle-database.schema.json — regenerate with: cargo xtask ci schema-codegen

//! Types generated from `contracts_v2/definitions/vehicle-database.schema.json`, one module per schema definition.

pub mod error;
mod vehicle;
pub use vehicle::*;
mod vehicle_database_contract;
pub use vehicle_database_contract::*;
mod vehicle_list;
pub use vehicle_list::*;
mod vehicle_patch;
pub use vehicle_patch::*;
mod vehicle_write;
pub use vehicle_write::*;
