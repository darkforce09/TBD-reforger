// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/fleet-command.schema.json — regenerate with: cargo xtask ci schema-codegen

//! Types generated from `contracts_v2/definitions/fleet-command.schema.json`, one module per schema definition.

mod claim_request;
pub mod error;
pub use claim_request::*;
mod claimed_fleet_command;
pub use claimed_fleet_command::*;
mod execution_result;
pub use execution_result::*;
mod execution_start;
pub use execution_start::*;
mod executor_kind;
pub use executor_kind::*;
mod fleet_action;
pub use fleet_action::*;
mod fleet_command_contract;
pub use fleet_command_contract::*;
mod fleet_command_list;
pub use fleet_command_list::*;
mod fleet_command_receipt;
pub use fleet_command_receipt::*;
mod fleet_command_request;
pub use fleet_command_request::*;
