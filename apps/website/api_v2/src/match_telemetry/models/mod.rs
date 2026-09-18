//! Match-telemetry database and wire models: the stored match record, its outcome enum, and the
//! per-player line items that hang off it.
//!
//! Field order and JSON keys are the wire contract: snake_case throughout, an absent value
//! expressed as `skip_serializing_if`, and RFC3339Nano timestamps rendered through
//! [`crate::core::wire_format`]. The enums map to the Postgres ENUM types. Soft-delete columns
//! are absent from these structs — the filter is enforced in the query layer.

pub mod match_record;

pub use match_record::{Match, MatchPlayerStat, MissionOutcome};
