// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/audit-log.schema.json — regenerate with: cargo xtask ci schema-codegen

//! Types generated from `contracts_v2/definitions/audit-log.schema.json`, one module per schema definition.

mod audit_log_contract;
pub mod error;
pub use audit_log_contract::*;
mod audit_log_entry;
pub use audit_log_entry::*;
mod audit_log_page;
pub use audit_log_page::*;
mod audit_stream_ready;
pub use audit_stream_ready::*;
mod audit_stream_reset;
pub use audit_stream_reset::*;
