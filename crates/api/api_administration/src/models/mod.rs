//! Administration wire/database models.
//!
//! Field order and JSON keys are the wire contract: snake_case throughout, an absent value
//! expressed as `skip_serializing_if`, and RFC3339Nano timestamps rendered through
//! [`fleet_wire_contract::rfc3339_timestamps`]. The enums map to Postgres ENUM types.

pub mod audit_log;
pub mod audit_stream;
pub mod personnel_page;
pub mod warning;

pub use audit_log::AuditLog;
pub use warning::Warning;
