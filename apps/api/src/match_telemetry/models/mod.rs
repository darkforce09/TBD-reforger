//! Match-telemetry database and wire models: the stored match record, its outcome enum, the
//! per-player line items that hang off it, and the machine-authenticated ingest shapes
//! (registration, results revisions, event batches, refusals) with the event read page.
//!
//! Field order and JSON keys are the wire contract: snake_case throughout, an absent value
//! expressed as `skip_serializing_if`, and RFC3339Nano timestamps rendered through
//! [`crate::core::wire_format`]. The enums map to the Postgres ENUM types. Soft-delete columns
//! are absent from these structs — the filter is enforced in the query layer.

pub mod match_event;
pub mod match_event_page;
pub mod match_record;
pub mod match_registration;
pub mod match_results_revision;
pub mod telemetry_refusal;

pub use match_record::{Match, MatchPlayerStat, MissionOutcome};
