//! Transaction services for accepting and attributing game telemetry: match registration, the
//! results-revision decision and its writes, detailed event batches, and the shared ingest
//! parsers.
pub mod ingest_parsing;
pub mod match_event_batches;
pub mod match_registration;
pub mod match_results_ingest;
pub mod match_revision_write;
pub mod registered_match;
pub mod results_revision;
