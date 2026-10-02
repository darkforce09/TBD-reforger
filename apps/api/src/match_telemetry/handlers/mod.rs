//! HTTP handlers owned by the match-telemetry domain. [`server_heartbeat`],
//! [`match_registration`], [`match_results`], [`match_event_batches`] and [`match_event_reads`]
//! are the routed entry points registered in [`super::routes`]; [`server_heartbeat_contract`] is
//! the heartbeat's wire input. The ingest wire models live in [`super::models`] and the
//! transactions in [`super::services`].

pub mod match_event_batches;
pub mod match_event_reads;
pub mod match_registration;
pub mod match_results;
pub mod server_heartbeat;
pub mod server_heartbeat_contract;
