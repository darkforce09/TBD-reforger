//! HTTP handlers owned by the match-telemetry domain, plus the pieces the two ingest endpoints
//! share. [`server_heartbeat`] and [`match_results`] are the routed entry points registered in
//! [`super::routes`]; [`ingest_parsing`], [`match_results_contract`], [`match_upsert`] and
//! [`attendance_attribution`] are the wire contract and the write steps behind them.

pub mod ingest_parsing;
pub mod match_results;
pub mod match_results_contract;
pub mod match_upsert;
pub mod server_heartbeat;
