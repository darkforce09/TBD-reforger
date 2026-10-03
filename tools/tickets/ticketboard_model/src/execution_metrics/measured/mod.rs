//! The measured run receipts, per ticket and per agent.
//!
//! **Role:** reads `.ai/tickets/metrics/<id>/*.json` as `ticket_metrics::RunRecord`s, checks each
//! receipt, sums them per ticket and per agent as `MetricsState`, and formats token counts.
//! **Position:** loaded by `crate::application_state::background_loading`; the desktop application
//! paints and sorts the tables; `crate::execution_metrics::estimated` reuses its error rows and id
//! checks.
//! **Signals & state:** none; plain data and pure functions over file text.
//! **Invariants:** a malformed receipt is a named error row and never part of a sum; an unfinished
//! run counts in runs, never in elapsed time.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use ticket_metrics::RunRecord;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

mod models;
pub use models::*;
mod aggregation;
pub use aggregation::*;
mod services;
pub use services::*;
mod formatting;
pub use formatting::*;

#[cfg(test)]
#[path = "tests/measured.rs"]
mod tests;
