//! Administration wire/database models.
//!
//! Field order and JSON keys are the wire contract: snake_case throughout, an absent value
//! expressed as `skip_serializing_if`, and RFC3339Nano timestamps rendered through
//! [`crate::core::wire_format`]. The enums map to Postgres ENUM types.

pub mod audit_log;
pub mod warning;

pub use audit_log::{AuditLog, AuditSeverity};
pub use warning::Warning;
