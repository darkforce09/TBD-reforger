//! The names a caller of the match telemetry models and ingest services imports with
//! `use api_match_telemetry::prelude::*;`.

pub use crate::models::match_record::{Match, MatchPlayerStat, MissionOutcome};
pub use crate::services::match_registration::register_match;
pub use crate::services::match_results_ingest::ingest_results_revision;
