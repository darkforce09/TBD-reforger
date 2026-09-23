//! `ticket sync`: regenerate the dispatch queue and the ticket-derived parts of two documents.
//!
//! **Role:** writes the dispatch queue (`queue.json`), the roadmap's recommended-next-work
//! block between its markers, and the ticket column of the gap-analysis tables.
//!
//! **Position:** reads the parents-only registry projection built by [`crate::registry`].
//! `cargo xtask ticket sync` calls [`cmd_sync`], and so does every registry mutator in
//! [`crate::cli`] that re-syncs after its write; `set-status` regenerates only the queue through
//! [`generate_queue_json`]. [`gap_analysis`] also serves `ticket check`, which verifies that the
//! gap-analysis tables round-trip.
//!
//! **Signals & state:** none held; each run rewrites its targets from the registry it is given.
//!
//! **Invariants:** an absent roadmap or gap-analysis file is skipped, never created; no write
//! collapses a marker block to a bare heading ([`refuse_empty_write`]); the gap-analysis ticket
//! column is rewritten only after the file's tables round-trip byte for byte.

use anyhow::{Result, bail};

use serde_json::{Value, json};

use crate::registry::*;
use crate::sync::gap_analysis::sync_gap_analysis_ticket_column;
use crate::validation::constants::*;
use std::fs;
use std::path::Path;
pub mod gap_analysis;

mod runner;

pub use runner::{cmd_sync, refuse_empty_write};

mod queue_json;

pub use queue_json::generate_queue_json;

mod markers;

use markers::inject_next_block;
#[cfg(test)]
use markers::marker_inner_is_vacuous;

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
