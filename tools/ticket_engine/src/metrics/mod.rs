//! Metrics for the ticket domain.

use anyhow::{Result, bail};

use serde::{Deserialize, Serialize};

use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use walkdir::WalkDir;

use crate::repository::{METRICS_DIR, METRICS_SCHEMA};

pub mod estimates;

mod model;

pub use model::{RunRecord, TokensConsumed, elapsed_sec, metrics_root, validate_record};

mod token_usage;

pub use token_usage::parse_tokens_from_cli_json;

mod receipts;

pub use receipts::{
    has_receipt, land_receipt_refusal, latest_run_file, missing_receipts, stamp_land,
    stamp_land_at, write_run_file,
};

use receipts::read_record;

mod verification;

pub use verification::check_as_errors;

mod summary;

pub use summary::{cmd_metrics, summarize_by_agent};

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
