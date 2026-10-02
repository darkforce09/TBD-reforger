// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts/definitions/match-telemetry.schema.json — regenerate with: cargo xtask ci schema-codegen

//! Types generated from `contracts/definitions/match-telemetry.schema.json`, one module per schema definition.

mod combat_death_payload;
pub mod error;
pub use combat_death_payload::*;
mod combat_kill_payload;
pub use combat_kill_payload::*;
mod match_event;
pub use match_event::*;
mod match_event_batch;
pub use match_event_batch::*;
mod match_event_batch_answer;
pub use match_event_batch_answer::*;
mod match_event_page;
pub use match_event_page::*;
mod match_registration;
pub use match_registration::*;
mod match_registration_answer;
pub use match_registration_answer::*;
mod match_report;
pub use match_report::*;
mod match_results_answer;
pub use match_results_answer::*;
mod match_results_revision;
pub use match_results_revision::*;
mod medical_incapacitated_payload;
pub use medical_incapacitated_payload::*;
mod medical_revived_payload;
pub use medical_revived_payload::*;
mod player_counters;
pub use player_counters::*;
mod player_line;
pub use player_line::*;
mod removed_line;
pub use removed_line::*;
mod telemetry_queue_reading;
pub use telemetry_queue_reading::*;
mod telemetry_queue_status;
pub use telemetry_queue_status::*;
mod telemetry_refusal;
pub use telemetry_refusal::*;
mod vehicle_destroyed_payload;
pub use vehicle_destroyed_payload::*;
mod vehicle_entered_payload;
pub use vehicle_entered_payload::*;
mod vehicle_exited_payload;
pub use vehicle_exited_payload::*;
