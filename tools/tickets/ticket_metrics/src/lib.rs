//! The run receipts of the ticket registry and the token estimates mined from git history.
//!
//! **Role:** writes, stamps, checks and summarises the run receipts under `.ai/tickets/metrics/`
//! ([`RunRecord`], [`write_run_file`], [`stamp_land`], [`check_as_errors`], [`cmd_metrics`]),
//! parses the token usage an agent run prints, and plans, writes and checks the per-ticket token
//! estimates ([`estimates`]).
//! **Position:** tier 3 of `tools/tickets`, over `ticket_model`, `repository_layout`,
//! `time_source` and `process_runner`; `ticket_registry`'s `stamp-sha` verb and `ticket check`,
//! xtask's `ticket metrics`, and xtask's `platform slice-run` and wave landing call it.
//! **Signals & state:** none; every function takes the checkout root and reads or writes files.
//! **Invariants:** a receipt and an estimate validate against their committed schemas before
//! they count; a stamp is RFC 3339 UTC; an estimate is derived from git history alone.

mod error;
pub mod estimates;
mod model;
pub mod prelude;
mod receipts;
mod summary;
mod token_usage;
mod verification;

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;

pub use error::{Error, Result};
pub use model::{RunRecord, TokensConsumed, elapsed_sec, metrics_root, validate_record};
pub use receipts::{
    has_receipt, land_receipt_refusal, latest_run_file, missing_receipts, stamp_land,
    stamp_land_at, write_run_file,
};
pub use summary::{cmd_metrics, summarize_by_agent};
pub use token_usage::parse_tokens_from_cli_json;
pub use verification::check_as_errors;
